//! onepage: a Preact TSX page in, one self-contained HTML file out.
//!
//! The engine is [`pagekit`]; this crate adds the config file ([`config`]), the lint
//! ([`lint`]), the build that writes the files ([`build`]), the watch-and-serve loop ([`dev`])
//! and the page scaffold ([`new_page`]).

pub mod config;
pub mod dev;
pub mod new_page;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use pagekit::lint::Report;

use crate::config::Config;

/// The lines to print for a failed command, and its exit code: 1 for a mistake in the page
/// (`file:line: error: message`, one line each), 2 for anything else (`onepage: reason`).
#[must_use]
pub fn report(e: &anyhow::Error) -> (String, u8) {
    if let Some(lint) = e.chain().find_map(|e| e.downcast_ref::<Report>()) {
        return (lint.to_string().trim_end().to_owned(), 1);
    }
    e.chain()
        .find_map(|e| e.downcast_ref::<pagekit::Finding>())
        .map_or_else(
            || (format!("onepage: {e:#}"), 2),
            |finding| {
                let line = format!(
                    "{}:{}: error: {}",
                    finding.file.display(),
                    finding.line,
                    finding.message
                );
                (line, 1)
            },
        )
}

/// Biome over the page in `dir` ([`pagekit::lint::page`]): read-only, a file the formatter
/// would change is a finding; with `fix`, the safe fixes and the formatter are written back.
pub fn lint(dir: &Path, fix: bool) -> Result<Report> {
    let config = Config::load(dir)?;
    pagekit::lint::page(&config.page(dir, false), &config.rules()?, fix)
}

/// Build the page in `dir` and write it to `out` (else the config's `out`), with the
/// `[wasm]` files next to it. The page's sources are fixed and formatted first, so what is
/// bundled is what is on disk; a lint error fails the build with its [`Report`]. Returns the
/// path written and the lint's report (its notes and the files it formatted).
pub fn build(dir: &Path, out: Option<&Path>, minify: bool) -> Result<(PathBuf, Report)> {
    let config = Config::load(dir)?;
    config.check_theme(dir)?;
    let lint = pagekit::lint::page(&config.page(dir, minify), &config.rules()?, true)?;
    if lint.errors() > 0 {
        return Err(lint.into());
    }
    let built = pagekit::build(&config.page(dir, minify))?;
    let out = out.map_or_else(|| dir.join(&config.out), Path::to_path_buf);
    let folder = out.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(folder).with_context(|| format!("create {}", folder.display()))?;
    std::fs::write(&out, built.html).with_context(|| format!("write {}", out.display()))?;
    for file in built.files {
        let name = file.file_name().context("a [wasm] file names no file")?;
        std::fs::copy(&file, folder.join(name))
            .with_context(|| format!("[wasm] files: copy {}", file.display()))?;
    }
    Ok((out, lint))
}
