//! Bundle a TSX module graph into one classic script with swc: strip TypeScript,
//! compile JSX to `h`/`Fragment`, link the relative imports and the bare ones the page maps
//! or vendors ([`packages`](crate::packages)), read `preact` and its hooks and JSX runtime
//! from the UMD globals, and optionally minify.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result, anyhow, bail};
use swc_core::atoms::Atom;
use swc_core::bundler::{Bundler, Config, Hook, Load, ModuleData, ModuleRecord, ModuleType};
use swc_core::common::comments::SingleThreadedComments;
use swc_core::common::{
    DUMMY_SP, FileName, GLOBALS, Globals, Mark, SourceMap, Span, Spanned, SyntaxContext, sync::Lrc,
};
use swc_core::ecma::ast::{
    Decl, EsVersion, Ident, IdentName, ImportSpecifier, KeyValuePatProp, KeyValueProp, MemberExpr,
    MemberProp, Module, ModuleDecl, ModuleExportName, ModuleItem, ObjectPat, ObjectPatProp, Pat,
    Program, PropName, Stmt, VarDecl, VarDeclKind, VarDeclarator,
};
use swc_core::ecma::codegen::{Config as CodegenConfig, Emitter, text_writer::JsWriter};
use swc_core::ecma::loader::resolve::{Resolution, Resolve};
use swc_core::ecma::loader::{TargetEnv, resolvers::node::NodeModulesResolver};
use swc_core::ecma::minifier::{
    optimize, option::CompressOptions, option::ExtraOptions, option::MinifyOptions,
};
use swc_core::ecma::parser::{
    EsSyntax, Syntax, TsSyntax, parse_file_as_module, parse_file_as_script,
};
use swc_core::ecma::transforms::base::helpers::{HELPERS, Helpers};
use swc_core::ecma::transforms::base::{fixer::fixer, hygiene::hygiene, resolver};
use swc_core::ecma::transforms::react::{Options as ReactOptions, Runtime, react};
use swc_core::ecma::transforms::typescript::strip;

use crate::packages::Imports;
use crate::{Finding, check};

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
    /// Minify the output (swc minifier, names mangled).
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
    /// Every module file loaded, canonical, in load order.
    pub modules: Vec<PathBuf>,
}

/// Bundle `entry` (and everything it imports from inside `root`, the import map and the
/// vendor folder) into one script. Runs on a thread with a large stack: swc recurses per tree
/// level.
pub fn bundle(root: &Path, entry: &Path, opts: &Options<'_>) -> Result<Bundled> {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn_scoped(s, || bundle_here(root, entry, opts))?
            .join()
            .map_err(|_| anyhow!("the bundler panicked"))?
    })
}

fn bundle_here(root: &Path, entry: &Path, opts: &Options<'_>) -> Result<Bundled> {
    let root = root
        .canonicalize()
        .with_context(|| format!("{}", root.display()))?;
    let cm: Lrc<SourceMap> = Lrc::default();
    let globals = Globals::new();
    let config = Config {
        external_modules: EXTERNALS.iter().map(|(e, _)| Atom::from(*e)).collect(),
        module: ModuleType::Es,
        ..Default::default()
    };
    let vendor = opts
        .imports
        .vendor
        .as_ref()
        .map(|v| v.canonicalize().unwrap_or_else(|_| v.clone()));
    let loader = Loader {
        cm: cm.clone(),
        root: root.clone(),
        vendor,
        opts,
        modules: Rc::default(),
    };
    let mut roots = vec![root.clone()];
    roots.extend(opts.imports.roots());
    let resolver = Resolver {
        inner: NodeModulesResolver::new(TargetEnv::Browser, HashMap::default(), true),
        root: root.clone(),
        roots,
        imports: opts.imports,
    };
    let loader_modules = loader.modules.clone();
    let mut bundler = Bundler::new(
        &globals,
        cm.clone(),
        loader,
        resolver,
        config,
        Box::new(NoHook),
    );
    let entries = HashMap::from([("main".to_string(), FileName::Real(root.join(entry)))]);
    let mut module = bundler
        .bundle(entries)?
        .pop()
        .context("swc produced no bundle")?
        .module;
    drop(bundler);
    let modules = loader_modules.take();
    externalize(&mut module, opts.host)?;
    if opts.minify {
        module = GLOBALS.set(&globals, || minify(&cm, module));
    }
    let script = format!("(function(){{{}}})();", emit(&cm, &module, opts.minify)?);
    // A browser drops a script it cannot parse, so check it parses as a classic one.
    let fm = cm.new_source_file(FileName::Anon.into(), script.clone());
    let mut errors = vec![];
    let parsed = parse_file_as_script(&fm, Syntax::default(), EsVersion::Es2022, None, &mut errors);
    if let Some(e) = parsed.err().or_else(|| errors.into_iter().next()) {
        bail!(
            "the bundle is not a valid classic script ({}); a top-level `await` is the usual cause",
            e.kind().msg()
        );
    }
    Ok(Bundled { script, modules })
}

/// `import.meta` is reported by the checks, so the bundler never asks for it.
struct NoHook;
impl Hook for NoHook {
    fn get_import_meta_props(&self, _: Span, _: &ModuleRecord) -> Result<Vec<KeyValueProp>> {
        Ok(vec![])
    }
}

/// Relative imports that stay inside the page directory, a mapped package or the vendor
/// folder, and the bare imports those resolve.
struct Resolver<'a> {
    inner: NodeModulesResolver,
    root: PathBuf,
    roots: Vec<PathBuf>,
    imports: &'a Imports,
}

impl Resolve for Resolver<'_> {
    fn resolve(&self, base: &FileName, spec: &str) -> Result<Resolution> {
        let at = |message: String| match base {
            FileName::Real(p) => Finding::at_text(&self.root, p, spec, message),
            _ => Finding::new(spec, 0, message),
        };
        if !spec.starts_with('.') && !spec.starts_with('/') {
            match self.imports.resolve(spec) {
                Ok(Some(path)) => {
                    return Ok(Resolution {
                        filename: FileName::Real(path),
                        slug: None,
                    });
                },
                Ok(None) => {},
                Err(e) => return Err(at(format!("imports `{spec}`: {e:#}")).into()),
            }
            let known = EXTERNALS.iter().take(2).map(|(e, _)| format!("`{e}`"));
            let known = known.collect::<Vec<_>>().join(", ");
            return Err(at(format!(
                "imports `{spec}`; only relative paths, {known} and the packages in [imports] or \
                 the vendor folder can be imported"
            ))
            .into());
        }
        let resolved = self.inner.resolve(base, spec)?;
        if let FileName::Real(p) = &resolved.filename {
            let p = p.canonicalize().unwrap_or_else(|_| p.clone());
            if !self.roots.iter().any(|r| p.starts_with(r)) {
                let msg = format!("imports `{spec}`, which is outside the page directory");
                return Err(at(msg).into());
            }
        }
        Ok(resolved)
    }
}

/// Parses each module, runs the checks, and compiles TypeScript and JSX away.
struct Loader<'a> {
    cm: Lrc<SourceMap>,
    root: PathBuf,
    /// Vendored code is third-party JavaScript: parsed and linked, not checked.
    vendor: Option<PathBuf>,
    opts: &'a Options<'a>,
    /// Every file loaded, for the class scan.
    modules: Rc<RefCell<Vec<PathBuf>>>,
}

impl Loader<'_> {
    fn finding(&self, path: &Path, span: Span, message: String) -> anyhow::Error {
        let line = self.cm.lookup_char_pos(span.lo).line;
        Finding::new(path.strip_prefix(&self.root).unwrap_or(path), line, message).into()
    }
}

impl Load for Loader<'_> {
    fn load(&self, file: &FileName) -> Result<ModuleData> {
        let FileName::Real(path) = file else {
            bail!("cannot load {file:?}");
        };
        let source =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        self.modules.borrow_mut().push(canonical.clone());
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let jsx = matches!(ext, "tsx" | "jsx");
        // Plain JavaScript is parsed as such: a few JS spellings are not valid TypeScript.
        let syntax = if matches!(ext, "js" | "mjs" | "jsx") {
            Syntax::Es(EsSyntax {
                jsx,
                ..Default::default()
            })
        } else {
            Syntax::Typescript(TsSyntax {
                tsx: jsx,
                ..Default::default()
            })
        };
        let fm = self.cm.new_source_file(file.clone().into(), source);
        let mut errors = vec![];
        let parsed = parse_file_as_module(&fm, syntax, EsVersion::Es2022, None, &mut errors);
        let module = match (parsed, errors.into_iter().next()) {
            (Err(e), _) | (Ok(_), Some(e)) => {
                return Err(self.finding(path, e.span(), e.kind().msg().into_owned()));
            },
            (Ok(m), None) => m,
        };
        let glue = self.opts.glue.as_ref().is_some_and(|g| {
            let g = self.root.join(g);
            canonical.starts_with(g.canonicalize().unwrap_or(g))
        });
        let vendored = self
            .vendor
            .as_ref()
            .is_some_and(|v| canonical.starts_with(v));
        let checked = !glue && !vendored;
        if let Some(span) = check::import_meta(&module) {
            let msg = "uses `import.meta`, which a classic script does not have".into();
            return Err(self.finding(path, span, msg));
        }
        if let Some((span, msg)) = check::jsx_factory(&module).filter(|_| checked) {
            return Err(self.finding(path, span, msg));
        }
        let (module, undeclared) = HELPERS.set(&Helpers::new(false), || {
            let unresolved = Mark::new();
            let top_level = Mark::new();
            let mut program = Program::Module(module).apply(resolver(unresolved, top_level, true));
            if jsx {
                let o = ReactOptions {
                    runtime: Some(Runtime::Classic),
                    pragma: Some("h".into()),
                    pragma_frag: Some("Fragment".into()),
                    ..Default::default()
                };
                let comments = Some(SingleThreadedComments::default());
                program = program.apply(react(self.cm.clone(), comments, o, top_level, unresolved));
            }
            let module = program.apply(strip(unresolved, top_level)).expect_module();
            let found = checked
                .then(|| check::undeclared(&module, unresolved, self.opts.wasm, self.opts.globals))
                .flatten();
            (module, found)
        });
        if let Some((span, msg)) = undeclared {
            return Err(self.finding(path, span, msg));
        }
        Ok(ModuleData {
            fm,
            module,
            helpers: Helpers::default(),
        })
    }
}

/// Turn each external `import { a, b as c } from "preact"` into
/// `const { a, b: c } = window.preact` (and `import * as p` into `const p = window.preact`),
/// and drop the remaining module syntax.
fn externalize(module: &mut Module, host: &str) -> Result<()> {
    let mut body = Vec::with_capacity(module.body.len());
    for item in std::mem::take(&mut module.body) {
        match item {
            ModuleItem::Stmt(s) => body.push(s),
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(e)) => body.push(Stmt::Decl(e.decl)),
            ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
                let src = import.src.value.as_str().unwrap_or_default();
                let Some((_, global)) = EXTERNALS.iter().find(|(e, _)| *e == src) else {
                    bail!("import of `{src}` survived bundling");
                };
                let init = MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(Ident::new_no_ctxt(host.into(), DUMMY_SP).into()),
                    prop: MemberProp::Ident(IdentName::new((*global).into(), DUMMY_SP)),
                };
                let mut namespaces = vec![];
                let props = import.specifiers.into_iter().filter_map(|s| {
                    let (key, local) = match s {
                        ImportSpecifier::Named(n) => match n.imported {
                            Some(ModuleExportName::Ident(i)) => {
                                (PropName::Ident(i.into()), n.local)
                            },
                            Some(ModuleExportName::Str(s)) => (PropName::Str(s), n.local),
                            None => (PropName::Ident(n.local.clone().into()), n.local),
                        },
                        ImportSpecifier::Default(d) => (
                            PropName::Ident(IdentName::new("default".into(), DUMMY_SP)),
                            d.local,
                        ),
                        ImportSpecifier::Namespace(n) => {
                            namespaces.push(n.local);
                            return None;
                        },
                    };
                    let value = Box::new(Pat::Ident(local.into()));
                    Some(ObjectPatProp::KeyValue(KeyValuePatProp { key, value }))
                });
                let pattern = Pat::Object(ObjectPat {
                    span: DUMMY_SP,
                    props: props.collect(),
                    optional: false,
                    type_ann: None,
                });
                let names = namespaces.into_iter().map(|n| Pat::Ident(n.into()));
                for name in std::iter::once(pattern).chain(names) {
                    let decl = VarDecl {
                        ctxt: SyntaxContext::empty(),
                        kind: VarDeclKind::Const,
                        decls: vec![VarDeclarator {
                            span: DUMMY_SP,
                            name,
                            init: Some(Box::new(init.clone().into())),
                            definite: false,
                        }],
                        ..Default::default()
                    };
                    body.push(Stmt::Decl(Decl::Var(Box::new(decl))));
                }
            },
            ModuleItem::ModuleDecl(_) => {},
        }
    }
    module.body = body.into_iter().map(ModuleItem::Stmt).collect();
    Ok(())
}

/// swc's minifier with its defaults, mangling top-level names too: the script is
/// wrapped in a function, so nothing leaks.
fn minify(cm: &Lrc<SourceMap>, module: Module) -> Module {
    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();
    let program = Program::Module(module).apply(resolver(unresolved_mark, top_level_mark, false));
    let options = MinifyOptions {
        compress: Some(CompressOptions::default()),
        mangle: Some(swc_core::ecma::minifier::option::MangleOptions {
            top_level: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let extra = ExtraOptions {
        unresolved_mark,
        top_level_mark,
        mangle_name_cache: None,
    };
    optimize(program, cm.clone(), None, None, &options, &extra)
        .apply(hygiene())
        .apply(fixer(None))
        .expect_module()
}

fn emit(cm: &Lrc<SourceMap>, module: &Module, minify: bool) -> Result<String> {
    let mut buf = vec![];
    let mut emitter = Emitter {
        // `inline_script` escapes `</script` and `<!--` inside strings.
        cfg: CodegenConfig::default()
            .with_minify(minify)
            .with_inline_script(true)
            .with_target(EsVersion::Es2020),
        cm: cm.clone(),
        comments: None,
        wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
    };
    emitter.emit_module(module)?;
    Ok(String::from_utf8(buf)?)
}
