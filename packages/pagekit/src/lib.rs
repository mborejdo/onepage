//! A Preact TSX page to one self-contained HTML file.
//!
//! [`build`] bundles the page's module graph into one classic script ([`bundle`], swc), writes
//! the utility sheet for the class names it uses ([`css`], encre-css), fills the shell's
//! `<!--css-->` and `<!--js-->` slots and pins the result with a strict Content-Security-Policy
//! ([`html`]). The checks in [`check`] fail the build on mistakes a browser would only show as
//! a blank page.
//!
//! ```no_run
//! let page = pagekit::Page::new("examples/hello");
//! let built = pagekit::build(&page)?;
//! std::fs::write("index.html", built.html)?;
//! # anyhow::Ok(())
//! ```

pub mod bundle;
pub mod check;
pub mod css;
pub mod html;
pub mod packages;
#[cfg(feature = "test")]
pub mod test;
pub mod tokens;

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

/// The slot in the shell the stylesheet goes into.
pub const CSS_SLOT: &str = "<!--css-->";
/// The slot in the shell the script goes into.
pub const JS_SLOT: &str = "<!--js-->";

/// The Preact runtime, inlined before the bundle: `window.preact`, `window.preactHooks` and
/// `window.preactJsx` (the automatic JSX runtime, for vendored packages compiled with it).
pub const RUNTIME: &str = concat!(
    include_str!("../assets/preact.umd.js"),
    "\n",
    include_str!("../assets/hooks.umd.js"),
    "\n",
    include_str!("../assets/jsx.js"),
);

/// One page to build. Paths are relative to [`root`](Self::root).
#[derive(Debug, Clone)]
pub struct Page {
    /// The page's directory.
    pub root: PathBuf,
    /// The module the bundle starts from.
    pub entry: PathBuf,
    /// The HTML shell with the two slots.
    pub shell: PathBuf,
    /// A stylesheet appended after the utilities.
    pub style: Option<PathBuf>,
    /// An SVG inlined as the page's icon.
    pub favicon: Option<PathBuf>,
    /// The `<title>`, replacing the shell's.
    pub title: Option<String>,
    /// Extra sources for the policy.
    pub csp: Csp,
    /// The WebAssembly opt-in.
    pub wasm: Option<Wasm>,
    /// A token file ([`tokens`]): CSS variables, and colour names for the utilities.
    pub tokens: Option<PathBuf>,
    /// Changes to single tokens, applied before the variables are written. The caller checks
    /// the result's contrast ([`tokens::Tokens::pairs`]).
    pub theme: Option<tokens::Theme>,
    /// The import map: a bare specifier or package name to a package folder or a file.
    pub imports: BTreeMap<String, PathBuf>,
    /// A folder of unpacked npm packages, for the bare imports the map does not name.
    pub vendor: Option<PathBuf>,
    /// Minify the script and strip the sheet.
    pub minify: bool,
}

impl Page {
    /// A page in `root` with every default: `src/app.tsx`, `index.html`, minified.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            entry: "src/app.tsx".into(),
            shell: "index.html".into(),
            style: None,
            favicon: None,
            title: None,
            csp: Csp::default(),
            wasm: None,
            tokens: None,
            theme: None,
            imports: BTreeMap::new(),
            vendor: None,
            minify: true,
        }
    }

    /// The import map and the vendor folder, joined to [`root`](Self::root).
    #[must_use]
    pub fn resolver(&self) -> packages::Imports {
        packages::Imports {
            map: self
                .imports
                .iter()
                .map(|(k, v)| (k.clone(), self.root.join(v)))
                .collect(),
            vendor: self.vendor.as_ref().map(|v| self.root.join(v)),
        }
    }
}

/// Sources added to the policy's directives of the same name, after the defaults.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Csp {
    /// `connect-src`: where `fetch` and web sockets may go.
    pub connect: Vec<String>,
    /// `img-src`.
    pub img: Vec<String>,
    /// `frame-src`.
    pub frame: Vec<String>,
    /// `font-src`.
    pub font: Vec<String>,
    /// `media-src`.
    pub media: Vec<String>,
}

/// The WebAssembly opt-in: `WebAssembly` and `Worker` may be used, the policy gets
/// `'wasm-unsafe-eval'`, and `files` are copied next to the page.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Wasm {
    /// Files copied next to the built page, by file name.
    pub files: Vec<PathBuf>,
    /// A directory of generated glue code that the checks skip.
    pub glue: Option<PathBuf>,
}

/// A built page.
#[derive(Debug)]
pub struct Built {
    /// The HTML file.
    pub html: String,
    /// Files to copy next to it (absolute or relative to the working directory).
    pub files: Vec<PathBuf>,
}

/// A mistake in the page's source, at a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The file, relative to the page.
    pub file: PathBuf,
    /// The 1-based line, or 0 when unknown.
    pub line: usize,
    /// What is wrong and how to fix it.
    pub message: String,
}

impl Finding {
    /// A finding at `line` of `file`.
    pub fn new(file: impl AsRef<Path>, line: usize, message: String) -> Self {
        Self {
            file: file.as_ref().to_path_buf(),
            line,
            message,
        }
    }

    /// A finding at the first line of `path` that quotes `needle`.
    #[must_use]
    pub fn at_text(root: &Path, path: &Path, needle: &str, message: String) -> Self {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let line = text
            .lines()
            .position(|l| {
                l.contains(&format!("\"{needle}\"")) || l.contains(&format!("'{needle}'"))
            })
            .map_or(0, |i| i + 1);
        Self::new(path.strip_prefix(root).unwrap_or(path), line, message)
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.file.display(), self.line, self.message)
    }
}

impl std::error::Error for Finding {}

/// Build `page` into one HTML file.
pub fn build(page: &Page) -> Result<Built> {
    let read = |p: &Path| {
        std::fs::read_to_string(page.root.join(p))
            .with_context(|| format!("reading {}", page.root.join(p).display()))
    };
    let shell = read(&page.shell)?;
    for slot in [CSS_SLOT, JS_SLOT] {
        ensure!(
            shell.contains(slot),
            "{} has no {slot} slot",
            page.shell.display()
        );
    }
    let imports = page.resolver();
    let options = bundle::Options {
        minify: page.minify,
        host: "window",
        wasm: page.wasm.is_some(),
        glue: page.wasm.as_ref().and_then(|w| w.glue.clone()),
        globals: &[],
        imports: &imports,
    };
    let bundled = bundle::bundle(&page.root, &page.entry, &options)?;
    let script = bundled.script;
    let src = page
        .root
        .join(page.entry.parent().unwrap_or_else(|| Path::new("")));
    let mut sources = css::sources(&src)?;
    // A mapped package's modules are the page's own code: their class names count, and its
    // sheet (`style` in its package.json) comes with it. Vendored packages bring neither.
    let mut base = vec![];
    for kit in imports.kits() {
        let used: Vec<_> = bundled
            .modules
            .iter()
            .filter(|m| m.starts_with(&kit))
            .collect();
        if used.is_empty() {
            continue;
        }
        for module in used {
            sources.push(
                std::fs::read_to_string(module)
                    .with_context(|| format!("reading {}", module.display()))?,
            );
        }
        if let Some(sheet) = packages::style(&kit)? {
            base.push(
                std::fs::read_to_string(&sheet)
                    .with_context(|| format!("reading {}", sheet.display()))?,
            );
        }
    }
    let tokens = page
        .tokens
        .as_ref()
        .map(|t| {
            let tokens = tokens::Tokens::load(&page.root.join(t))?;
            match &page.theme {
                Some(theme) => tokens.with(theme),
                None => Ok(tokens),
            }
        })
        .transpose()?;
    let style = css::Style {
        tokens: tokens.as_ref(),
        base: &base,
    };
    let mut sheet = css::generate(&sources, &shell, &style);
    if let Some(style) = &page.style {
        sheet.push('\n');
        sheet.push_str(&read(style)?);
    }
    if page.minify {
        sheet = css::minify(&sheet);
    }
    let favicon = page.favicon.as_ref().map(|p| read(p)).transpose()?;
    let html = html::fill(&shell, &sheet, &format!("{RUNTIME}\n{script}"));
    let html = html::head(&html, page.title.as_deref(), favicon.as_deref())?;
    let html = html::secure(&html, &page.csp, page.wasm.is_some())?;
    let files = page.wasm.iter().flat_map(|w| &w.files);
    Ok(Built {
        html,
        files: files.map(|f| page.root.join(f)).collect(),
    })
}
