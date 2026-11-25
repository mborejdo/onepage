//! Bundle a TSX module graph into one classic script with rolldown (Oxc inside): strip
//! TypeScript, compile JSX to `h`/`Fragment`, link the relative imports and the bare ones the
//! page maps or vendors ([`crate::packages`]), read `preact` and its hooks and JSX
//! runtime from the UMD globals, and optionally minify.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Context, Result, anyhow, bail};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};
use rolldown::plugin::{
    HookResolveIdArgs, HookResolveIdOutput, HookResolveIdReturn, HookTransformArgs,
    HookTransformReturn, HookUsage, Plugin, PluginContext, Pluginable,
    SharedTransformPluginContext,
};
use rolldown::{
    Bundler, BundlerOptions, BundlerTransformOptions, CommentsOptions, Either, GlobalsOutputOption,
    InputItem, IsExternal, JsxOptions, OutputFormat, Platform, RawMinifyOptions, TsConfig,
};

use crate::packages::{self, Imports};
use crate::{Finding, check, line_at};

/// The modules read from globals instead of being bundled: specifier and global name. They win
/// over the import map and the vendor folder, so the page and every package it imports share
/// one Preact.
pub const EXTERNALS: &[(&str, &str)] = &[
    ("preact", "preact"),
    ("preact/hooks", "preactHooks"),
    ("preact/jsx-runtime", "preactJsx"),
    ("preact/jsx-dev-runtime", "preactJsx"),
];

/// How a script is bundled.
#[derive(Debug)]
pub struct Options<'a> {
    /// Minify the output (Oxc's minifier, names mangled).
    pub minify: bool,
    /// The object the externals are read from: `window` on a page.
    pub host: &'a str,
    /// Allow the WebAssembly globals.
    pub wasm: bool,
    /// Modules under this directory skip the checks (generated code).
    pub glue: Option<PathBuf>,
    /// Extra names the host defines before the script runs.
    pub globals: &'a [&'a str],
    /// Where bare imports resolve.
    pub imports: &'a Imports,
}

/// A bundled script and the modules it was made of.
#[derive(Debug)]
pub struct Bundled {
    /// The classic script.
    pub script: String,
    /// Every module file in the script, canonical, in execution order.
    pub modules: Vec<PathBuf>,
}

/// Bundle `entry` (and everything it imports from inside `root`, the import map and the
/// vendor folder) into one script: an IIFE whose parameters are the Preact globals.
pub fn bundle(root: &Path, entry: &Path, options: &Options<'_>) -> Result<Bundled> {
    let root = root
        .canonicalize()
        .with_context(|| format!("resolve {}", root.display()))?;
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let mut roots = vec![root.clone()];
    roots.extend(options.imports.roots());
    let found = Arc::new(Mutex::new(None));
    let graph = Graph {
        root: root.clone(),
        roots,
        imports: options.imports.clone(),
        vendor: options.imports.vendor.as_deref().map(canonical),
        glue: options.glue.as_ref().map(|g| canonical(&root.join(g))),
        wasm: options.wasm,
        globals: options.globals.iter().map(|g| (*g).to_owned()).collect(),
        found: Arc::clone(&found),
    };
    let externals: Vec<String> = EXTERNALS.iter().map(|(e, _)| (*e).to_owned()).collect();
    let globals = EXTERNALS
        .iter()
        .map(|(e, g)| ((*e).to_owned(), format!("{}.{g}", options.host)))
        .collect();
    let none = CommentsOptions {
        legal: false,
        annotation: false,
        jsdoc: false,
    };
    let bundler_options = BundlerOptions {
        input: Some(vec![InputItem {
            name: Some("page".into()),
            import: root.join(entry).to_string_lossy().into_owned(),
        }]),
        cwd: Some(root.clone()),
        platform: Some(Platform::Browser),
        format: Some(OutputFormat::Iife),
        external: Some(IsExternal::from(externals)),
        globals: Some(GlobalsOutputOption::FxHashMap(globals)),
        minify: Some(RawMinifyOptions::Bool(options.minify)),
        comments: Some(none),
        // The page's own settings win: a tsconfig.json next to it does not change the JSX.
        tsconfig: Some(TsConfig::Auto(false)),
        transform: Some(BundlerTransformOptions {
            jsx: Some(Either::Right(JsxOptions {
                runtime: Some("classic".into()),
                pragma: Some("h".into()),
                pragma_frag: Some("Fragment".into()),
                ..Default::default()
            })),
            // Strip types and compile JSX; lower nothing.
            target: Some(Either::Left("esnext".into())),
            ..Default::default()
        }),
        ..Default::default()
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("start the bundler's runtime")?;
    let output = runtime.block_on(async {
        let mut bundler =
            Bundler::with_plugins(bundler_options, vec![Pluginable::new_shared(graph)])?;
        let output = bundler.generate().await;
        bundler.close().await.ok();
        output
    });
    let finding = found.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(finding) = finding {
        return Err(finding.into());
    }
    let output = output.map_err(|errors| {
        let diagnostics: Vec<String> = errors
            .iter()
            .map(|e| e.to_diagnostic().to_string())
            .collect();
        one_error(&diagnostics)
    })?;
    let mut chunks = output.assets.into_iter().filter_map(|o| match o {
        rolldown_common::Output::Chunk(c) => Some(c),
        rolldown_common::Output::Asset(_) => None,
    });
    let chunk = chunks.next().context("rolldown produced no bundle")?;
    if chunks.next().is_some() {
        bail!("rolldown split the page into chunks; a page is one script");
    }
    let modules = chunk
        .module_ids
        .iter()
        .map(|id| Path::new(id.raw_id()))
        .filter(|p| p.is_file())
        .map(canonical)
        .collect();
    // `</script` is escaped by the printer; `<!--` (which, followed by `<script`, would change how
    // the browser reads the rest of the element) only occurs in strings, regexes and templates,
    // where `\x3C` is the same character.
    let script = chunk.code.trim_end().replace("<!--", "\\x3C!--");
    // A browser drops a script it cannot parse, so check it parses as a classic one.
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &script, SourceType::cjs()).parse();
    if let Some(e) = parsed.diagnostics.first() {
        bail!(
            "the bundle is not a valid classic script: {}; a top-level `await` is the usual cause",
            e.message
        );
    }
    Ok(Bundled { script, modules })
}

/// rolldown's errors as one line: its diagnostics are multi-line, with a code frame, so the
/// first line of the first one is the reason.
fn one_error(diagnostics: &[String]) -> anyhow::Error {
    let first = diagnostics
        .first()
        .map_or("", String::as_str)
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default()
        .trim();
    match diagnostics.len() {
        0 | 1 => anyhow!("rolldown: {first}"),
        n => anyhow!("rolldown: {first}; and {} more", n - 1),
    }
}

/// The plugin that makes the graph a page's: bare imports through the import map and the
/// vendor folder, relative ones kept inside the page, and the checks on every module.
#[derive(Debug)]
struct Graph {
    root: PathBuf,
    /// The folders a module may come from: the page, the mapped folders, the vendor folder.
    roots: Vec<PathBuf>,
    imports: Imports,
    /// Vendored code is third-party JavaScript: linked, not checked.
    vendor: Option<PathBuf>,
    /// Generated glue: linked, not checked.
    glue: Option<PathBuf>,
    wasm: bool,
    globals: Vec<String>,
    /// The first mistake found; rolldown's own error would lose its file and line.
    found: Arc<Mutex<Option<Finding>>>,
}

impl Graph {
    /// Keep the first finding and stop the build.
    fn fail(&self, finding: Finding) -> anyhow::Error {
        let message = finding.to_string();
        self.found
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_or_insert(finding);
        anyhow!(message)
    }

    fn resolve(&self, spec: &str, importer: Option<&str>) -> Result<Option<PathBuf>> {
        let at = |message: String| match importer {
            Some(p) => Finding::at_text(&self.root, Path::new(p), spec, message),
            None => Finding::new(spec, 0, message),
        };
        if EXTERNALS.iter().any(|(e, _)| *e == spec) {
            return Ok(None);
        }
        if !spec.starts_with('.') && !spec.starts_with('/') {
            return match self.imports.resolve(spec) {
                Ok(Some(path)) => Ok(Some(path)),
                Ok(None) => {
                    let known = EXTERNALS.iter().take(2).map(|(e, _)| format!("`{e}`"));
                    let known = known.collect::<Vec<_>>().join(", ");
                    Err(self.fail(at(format!(
                        "imports `{spec}`; only relative paths, {known} and the packages in \
                         [imports] or the vendor folder can be imported"
                    ))))
                },
                Err(e) => Err(self.fail(at(format!("imports `{spec}`: {e:#}")))),
            };
        }
        let base = importer
            .and_then(|p| Path::new(p).parent())
            .unwrap_or(&self.root);
        let Some(path) = packages::probe(&base.join(spec)) else {
            return Err(self.fail(at(format!("imports `{spec}`, which names no file"))));
        };
        let path = path.canonicalize().unwrap_or(path);
        if !self.roots.iter().any(|r| path.starts_with(r)) {
            let msg = format!("imports `{spec}`, which is outside the page directory");
            return Err(self.fail(at(msg)));
        }
        Ok(Some(path))
    }

    /// The checks on one module's source.
    fn check(&self, path: &Path, code: &str) -> Result<()> {
        let finding = |span: Span, message: String| {
            let file = path.strip_prefix(&self.root).unwrap_or(path);
            self.fail(Finding::new(file, line_at(code, span.start), message))
        };
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        // Plain JavaScript is parsed as such: a few JS spellings are not valid TypeScript.
        let source_type = match ext {
            "tsx" => SourceType::tsx(),
            "jsx" => SourceType::jsx(),
            "js" | "mjs" => SourceType::mjs(),
            _ => SourceType::ts(),
        };
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, code, source_type).parse();
        if let Some(e) = parsed.diagnostics.first() {
            let span = e
                .labels
                .first()
                .map_or_else(Span::default, |l| Span::new(l.offset(), l.offset()));
            return Err(finding(span, e.message.to_string()));
        }
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(&parsed.program)
            .semantic;
        let under = |dir: &Option<PathBuf>| dir.as_ref().is_some_and(|d| path.starts_with(d));
        let checked = !under(&self.glue) && !under(&self.vendor);
        if let Some(span) = check::import_meta(&semantic) {
            let msg = "uses `import.meta`, which a classic script does not have".into();
            return Err(finding(span, msg));
        }
        if !checked {
            return Ok(());
        }
        if let Some((span, msg)) = check::jsx_factory(&semantic) {
            return Err(finding(span, msg));
        }
        let globals: Vec<&str> = self.globals.iter().map(String::as_str).collect();
        if let Some((span, msg)) = check::undeclared(&semantic, self.wasm, &globals) {
            return Err(finding(span, msg));
        }
        Ok(())
    }
}

impl Plugin for Graph {
    fn name(&self) -> Cow<'static, str> {
        "pagekit".into()
    }

    // Both hooks do their work before they return: nothing in them waits.
    fn resolve_id(
        &self,
        _ctx: &PluginContext,
        args: &HookResolveIdArgs<'_>,
    ) -> impl Future<Output = HookResolveIdReturn> + Send {
        let resolved = if args.is_entry {
            Ok(None)
        } else {
            self.resolve(args.specifier, args.importer)
                .map(|p| p.map(|p| HookResolveIdOutput::from_id(p.to_string_lossy().into_owned())))
        };
        std::future::ready(resolved)
    }

    fn transform(
        &self,
        _ctx: SharedTransformPluginContext,
        args: &HookTransformArgs<'_>,
    ) -> impl Future<Output = HookTransformReturn> + Send {
        let path = Path::new(args.id);
        let checked = if path.is_file() {
            self.check(path, args.code)
        } else {
            Ok(())
        };
        std::future::ready(checked.map(|()| None))
    }

    fn register_hook_usage(&self) -> HookUsage {
        HookUsage::ResolveId | HookUsage::Transform
    }
}
