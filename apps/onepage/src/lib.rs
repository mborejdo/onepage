//! onepage: a Preact TSX page in, one self-contained HTML file out.
//!
//! The engine is [`pagekit`]; this crate adds the config file ([`config`]), the build that
//! writes the files ([`build`]), the watch-and-serve loop ([`dev`]) and the page scaffold
//! ([`new_page`]).

pub mod config;
pub mod dev;
pub mod new_page;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;

/// The line to print for a failed command, and its exit code: 1 for a mistake in the page
/// (`error: file:line: message`), 2 for anything else (`onepage: reason`).
#[must_use]
pub fn report(e: &anyhow::Error) -> (String, u8) {
    e.chain()
        .find_map(|e| e.downcast_ref::<pagekit::Finding>())
        .map_or_else(
            || (format!("onepage: {e:#}"), 2),
            |finding| (format!("error: {finding}"), 1),
        )
}

/// Build the page in `dir` and write it to `out` (else the config's `out`), with the
/// `[wasm]` files next to it. Returns the path written.
pub fn build(dir: &Path, out: Option<&Path>, minify: bool) -> Result<PathBuf> {
    let config = Config::load(dir)?;
    let built = pagekit::build(&config.page(dir, minify))?;
    let out = out.map_or_else(|| dir.join(&config.out), Path::to_path_buf);
    let folder = out.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(folder).with_context(|| format!("creating {}", folder.display()))?;
    std::fs::write(&out, built.html).with_context(|| format!("writing {}", out.display()))?;
    for file in built.files {
        let name = file.file_name().context("a [wasm] file names no file")?;
        std::fs::copy(&file, folder.join(name))
            .with_context(|| format!("[wasm] files: copying {}", file.display()))?;
    }
    Ok(out)
}
