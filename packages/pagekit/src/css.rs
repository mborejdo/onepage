//! The stylesheet: the token variables, encre-css's preflight, the sheets of the packages the
//! page imports, and the utilities for every class name the source and the shell use - in
//! that order, so a utility wins over a package's base rule.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use encre_css::{Config, Preflight};

use crate::tokens::Tokens;

/// What the sheet holds besides the utilities.
#[derive(Debug, Default)]
pub struct Style<'a> {
    /// The token file: its variables first, its colour names known to the utilities, and the
    /// preflight's faces set to `--font-sans` and `--font-mono`.
    pub tokens: Option<&'a Tokens>,
    /// Sheets between the preflight and the utilities (the imported packages').
    pub base: &'a [String],
}

/// Every `.ts` and `.tsx` file under `dir`, sorted.
pub fn files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in
            std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))?
        {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "ts" || e == "tsx") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The text of every source file under `dir`, for the class scan.
pub fn sources(dir: &Path) -> Result<Vec<String>> {
    files(dir)?
        .iter()
        .map(|p| std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display())))
        .collect()
}

/// The preflight reset and the utilities that `sources` and `shell` use, with `style` around
/// them.
#[must_use]
pub fn generate(sources: &[String], shell: &str, style: &Style<'_>) -> String {
    let mut config = Config::default();
    let mut out = String::new();
    if let Some(tokens) = style.tokens {
        out.push_str(&tokens.css());
        for (name, value) in tokens.colors() {
            config.theme.colors.add(name, value);
        }
        config.preflight = Preflight::new_full()
            .font_family_sans("var(--font-sans)")
            .font_family_mono("var(--font-mono)");
    }
    // With no class names, encre-css writes the preflight alone.
    out.push_str(&encre_css::generate([], &config));
    for sheet in style.base {
        out.push('\n');
        out.push_str(sheet);
    }
    config.preflight = Preflight::new_none();
    let all = sources.iter().map(String::as_str).chain([shell]);
    out.push('\n');
    let utilities = encre_css::generate(all, &config);
    if style.tokens.is_some() {
        // encre-css writes `font-sans` and `font-mono` with fixed system stacks; with a token
        // file they are its faces, like the preflight's.
        out.push_str(
            &utilities
                .replace(SANS_STACK, "font-family: var(--font-sans);")
                .replace(MONO_STACK, "font-family: var(--font-mono);"),
        );
    } else {
        out.push_str(&utilities);
    }
    out
}

/// encre-css's `font-sans` declaration.
const SANS_STACK: &str = r#"font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Noto Sans", sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji";"#;
/// encre-css's `font-mono` declaration.
const MONO_STACK: &str =
    r#"font-family: Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;"#;

/// `sheet` with every relative `url(...)` to a font file (`.woff2`, `.woff`) as a `data:` URI,
/// read from `dir`: the page stays one file and the faces come with it. Other URLs stay.
pub fn inline_fonts(sheet: &str, dir: &Path) -> Result<String> {
    use base64::Engine as _;
    use std::fmt::Write as _;
    let mut out = String::with_capacity(sheet.len());
    let mut rest = sheet;
    while let Some(at) = rest.find("url(") {
        let open = at + "url(".len();
        out.push_str(&rest[..open]);
        rest = &rest[open..];
        let Some(close) = rest.find(')') else { break };
        let target = rest[..close].trim().trim_matches(['"', '\'']);
        let mime = match Path::new(target).extension().and_then(|e| e.to_str()) {
            Some("woff2") => "font/woff2",
            Some("woff") => "font/woff",
            _ => continue,
        };
        if target.contains(':') || target.starts_with(['/', '#']) {
            continue;
        }
        let path = dir.join(target);
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        let _ = write!(out, "\"data:{mime};base64,{data}\"");
        rest = &rest[close..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The sheet without comments and indentation: a strip, not a rewrite.
#[must_use]
pub fn minify(css: &str) -> String {
    let mut text = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(at) = rest.find("/*") {
        text.push_str(&rest[..at]);
        rest = rest[at..]
            .find("*/")
            .map_or("", |end| &rest[at + end + 2..]);
    }
    text.push_str(rest);
    let mut out = String::with_capacity(text.len());
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let joined = out.ends_with(['{', '}', ';', ',']) || line.starts_with(['{', '}']);
        if !out.is_empty() && !joined {
            out.push(' ');
        }
        out.push_str(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_used_class_gets_its_rule() {
        let css = generate(
            &["<div class=\"p-4 text-sm\">".into()],
            "",
            &Style::default(),
        );
        assert!(css.contains(".p-4"), "{css}");
        assert!(css.contains("padding: 1rem"), "{css}");
    }

    #[test]
    fn token_colours_are_utilities_and_base_sheets_precede_them() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../panelkit/tokens.toml");
        let tokens = Tokens::load(&path).unwrap();
        let base = [".pk-x { color: red; }".to_owned()];
        let style = Style {
            tokens: Some(&tokens),
            base: &base,
        };
        let html = "<p class=\"bg-surface text-muted border-border bg-accent/15 text-series-2 \
                    font-sans font-mono\">";
        let css = generate(&[html.into()], "", &style);
        assert!(
            !css.contains(MONO_STACK) && !css.contains(SANS_STACK),
            "{css}"
        );
        assert!(
            css.contains(".font-mono {\n  font-family: var(--font-mono);"),
            "{css}"
        );
        assert!(css.starts_with(":root{"), "{css}");
        for rule in [
            "background-color: var(--surface)",
            "color: var(--muted)",
            "border-color: var(--border)",
            "color-mix(in oklab, var(--accent) 15%, transparent)",
            "color: var(--series-2)",
            "var(--font-sans)",
        ] {
            assert!(css.contains(rule), "{rule}: {css}");
        }
        assert!(css.find(".pk-x").unwrap() < css.find(".bg-surface").unwrap());
    }

    #[test]
    fn a_relative_font_url_is_inlined_and_the_rest_stays() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../panelkit");
        let sheet = "@font-face{src:url(\"fonts/inter.woff2\") format(\"woff2\")}\n\
                     .a{background:url(x.png)}.b{src:url(https://x.test/f.woff2)}";
        let css = inline_fonts(sheet, &dir).unwrap();
        assert!(
            css.starts_with("@font-face{src:url(\"data:font/woff2;base64,d09GMg"),
            "{}",
            &css[..80]
        );
        assert!(css.ends_with(
            ") format(\"woff2\")}\n.a{background:url(x.png)}.b{src:url(https://x.test/f.woff2)}"
        ));
        assert!(inline_fonts("a{src:url(missing.woff2)}", &dir).is_err());
    }

    #[test]
    fn minify_drops_comments_and_indentation() {
        let css = "/* reset */\n.a,\n.b {\n  color: red;\n}\n";
        assert_eq!(minify(css), ".a,.b {color: red;}");
    }
}
