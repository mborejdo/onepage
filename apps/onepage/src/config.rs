//! `onepage.toml`, the page's config file, read from the page's folder.
//!
//! Every key is optional; a missing file means every default. An unknown key or a malformed
//! value is an error that names the file and the key.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use pagekit::{Csp, Page, Wasm};
use serde::Deserialize;

/// The file name looked up in the page's folder.
pub const PROJECT_FILE: &str = "onepage.toml";

/// The page's settings, as written in [`PROJECT_FILE`].
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The module the bundle starts from.
    pub entry: PathBuf,
    /// The HTML shell, with a `<!--css-->` and a `<!--js-->` slot.
    pub shell: PathBuf,
    /// Where the built page is written.
    pub out: PathBuf,
    /// A stylesheet appended after the generated utilities.
    pub style: Option<PathBuf>,
    /// An SVG inlined as the page's icon.
    pub favicon: Option<PathBuf>,
    /// The page's `<title>`, replacing the shell's.
    pub title: Option<String>,
    /// Extra sources for the Content-Security-Policy.
    pub csp: Csp,
    /// The WebAssembly opt-in.
    pub wasm: Option<Wasm>,
    /// A token file: CSS variables for both schemes, and its colour names as utilities.
    pub tokens: Option<PathBuf>,
    /// A folder of unpacked npm packages, for bare imports.
    pub vendor: Option<PathBuf>,
    /// The import map: a bare specifier or package name to a package folder or a file.
    pub imports: BTreeMap<String, PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        let page = Page::new("");
        Self {
            entry: page.entry,
            shell: page.shell,
            out: "dist/index.html".into(),
            style: None,
            favicon: None,
            title: None,
            csp: Csp::default(),
            wasm: None,
            tokens: None,
            vendor: None,
            imports: BTreeMap::new(),
        }
    }
}

impl Config {
    /// Read `dir/onepage.toml`, or the defaults when there is none.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(PROJECT_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        toml::from_str(&text).with_context(|| path.display().to_string())
    }

    /// The page in `dir` these settings describe.
    #[must_use]
    pub fn page(&self, dir: &Path, minify: bool) -> Page {
        Page {
            root: dir.to_path_buf(),
            entry: self.entry.clone(),
            shell: self.shell.clone(),
            style: self.style.clone(),
            favicon: self.favicon.clone(),
            title: self.title.clone(),
            csp: self.csp.clone(),
            wasm: self.wasm.clone(),
            tokens: self.tokens.clone(),
            imports: self.imports.clone(),
            vendor: self.vendor.clone(),
            minify,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_optional_and_unknown_keys_fail() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.out, PathBuf::from("dist/index.html"));
        let config: Config = toml::from_str("[wasm]\nfiles = [\"a.wasm\"]").unwrap();
        assert_eq!(config.wasm.unwrap().files, [PathBuf::from("a.wasm")]);
        assert!(toml::from_str::<Config>("nmae = \"x\"").is_err());
        assert!(toml::from_str::<Config>("[csp]\nscript = []").is_err());
        let config: Config = toml::from_str(
            "tokens = \"t.toml\"\nvendor = \"../vendor\"\n[imports]\nkit = \"../kit\"\n",
        )
        .unwrap();
        assert_eq!(config.imports["kit"], PathBuf::from("../kit"));
        assert_eq!(config.vendor, Some(PathBuf::from("../vendor")));
    }
}
