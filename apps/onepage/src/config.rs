//! `onepage.toml`, the page's config file, read from the page's folder.
//!
//! Every key is optional; a missing file means every default. An unknown key or a malformed
//! value is an error that names the file and the key.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use pagekit::tokens::{Theme, Tokens};
use pagekit::{Csp, Finding, Page, Wasm};
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
    /// Changes to single tokens of [`tokens`](Self::tokens): `[theme.light]`, `[theme.dark]`,
    /// `[theme.fonts]`.
    pub theme: Option<Theme>,
    /// A folder of unpacked npm packages, for bare imports.
    pub vendor: Option<PathBuf>,
    /// The import map: a bare specifier or package name to a package folder or a file.
    pub imports: BTreeMap<String, PathBuf>,
    /// Lint rules moved by group and by name, for the page's own sources.
    pub lint: pagekit::lint::Config,
    /// The type checks' ratchet: per code, how many findings the page may have
    /// (`TS2322 = 3`). Without it, every type finding is a note.
    pub types: Option<BTreeMap<String, usize>>,
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
            theme: None,
            vendor: None,
            imports: BTreeMap::new(),
            lint: pagekit::lint::Config::default(),
            types: None,
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
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let config: Self =
            toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        if config.theme.is_some() && config.tokens.is_none() {
            bail!(
                "{}: [theme] changes a token file, and there is no `tokens`",
                path.display()
            );
        }
        config.rules().with_context(|| path.display().to_string())?;
        Ok(config)
    }

    /// The rules of `[lint]`: an unknown group or rule is an error that names it.
    pub fn rules(&self) -> Result<pagekit::lint::Rules> {
        pagekit::lint::Rules::of(&self.lint).context("[lint]")
    }

    /// With a `[theme]`, hold the changed palettes to the token file's contrast duties
    /// ([`Tokens::pairs`]). The first failing pair a change takes part in is a [`Finding`] at
    /// the change's line.
    pub fn check_theme(&self, dir: &Path) -> Result<()> {
        let (Some(theme), Some(tokens)) = (&self.theme, &self.tokens) else {
            return Ok(());
        };
        let tokens = Tokens::load(&dir.join(tokens))?
            .with(theme)
            .context(PROJECT_FILE)?;
        let text = std::fs::read_to_string(dir.join(PROJECT_FILE)).unwrap_or_default();
        for pair in tokens.pairs().iter().filter(|p| !p.passes()) {
            let changed = if pair.scheme == "light" {
                theme.light.named()
            } else {
                theme.dark.named()
            };
            let is_changed = |name: &str| changed.iter().any(|(n, _)| *n == name);
            let table = format!("theme.{}", pair.scheme);
            let (key, message) = if is_changed(&pair.fg) {
                (
                    pair.fg.as_str(),
                    format!(
                        "[{table}] {} {} is {}:1 on {}, needs {}:1",
                        pair.fg,
                        pair.fg_hex,
                        pair.shown(),
                        pair.bg,
                        pair.need
                    ),
                )
            } else if is_changed(pair.bg) {
                (
                    pair.bg,
                    format!(
                        "[{table}] {} {} is {}:1 on {} {}, needs {}:1",
                        pair.fg,
                        pair.fg_hex,
                        pair.shown(),
                        pair.bg,
                        pair.bg_hex,
                        pair.need
                    ),
                )
            } else {
                continue;
            };
            return Err(Finding::new(PROJECT_FILE, line_of(&text, &table, key), message).into());
        }
        Ok(())
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
            theme: self.theme.clone(),
            imports: self.imports.clone(),
            vendor: self.vendor.clone(),
            minify,
        }
    }
}

/// The 1-based line of `key` in the `[table]` of a TOML text, else the table's line, else 0.
fn line_of(text: &str, table: &str, key: &str) -> usize {
    let header = format!("[{table}]");
    let mut inside = None;
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.starts_with('[') {
            if inside.is_some() {
                break;
            }
            if line.starts_with(&header) {
                inside = Some(i + 1);
            }
        } else if inside.is_some()
            && line
                .strip_prefix(key)
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        {
            return i + 1;
        }
    }
    inside.unwrap_or(0)
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

    #[test]
    fn lint_moves_groups_and_rules_and_names_an_unknown_one() {
        let config: Config = toml::from_str(
            "[lint]\ngroups = { style = \"error\" }\nrules = { \"style/noTernary\" = \"off\" }\n",
        )
        .unwrap();
        assert!(config.rules().is_ok());
        let config: Config =
            toml::from_str("[lint]\nrules = { \"style/noTernery\" = \"off\" }\n").unwrap();
        let error = format!("{:#}", config.rules().unwrap_err());
        assert!(error.contains("no rule `noTernery` in `style`"), "{error}");
        assert!(
            toml::from_str::<Config>("[lint]\nrules = { \"style/noTernary\" = \"warn\" }").is_err()
        );
    }

    #[test]
    fn a_theme_knows_its_token_names() {
        let config: Config =
            toml::from_str("tokens = \"t.toml\"\n[theme.dark]\naccent = \"#b4a9f5\"\n").unwrap();
        assert_eq!(
            config.theme.unwrap().dark.accent.as_deref(),
            Some("#b4a9f5")
        );
        let error = toml::from_str::<Config>("[theme.dark]\naccnet = \"#b4a9f5\"\n").unwrap_err();
        assert!(
            error.to_string().contains("unknown field `accnet`"),
            "{error}"
        );
        assert!(toml::from_str::<Config>("[theme.sizes]\ntext = 1\n").is_err());
    }

    #[test]
    fn a_key_is_found_in_its_table() {
        let text = "tokens = \"t\"\n[theme.light]\naccent = \"#000000\"\n\n[theme.dark]\nink = \"#000000\"\naccent=\"#000000\"\n";
        assert_eq!(line_of(text, "theme.light", "accent"), 3);
        assert_eq!(line_of(text, "theme.dark", "accent"), 7);
        assert_eq!(line_of(text, "theme.dark", "ground"), 5);
        assert_eq!(line_of(text, "theme.fonts", "sans"), 0);
    }
}
