//! The stylesheet: encre-css utilities for every class name the source and the shell use.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

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

/// The preflight reset and the utilities that `sources` and `shell` use.
#[must_use]
pub fn generate(sources: &[String], shell: &str) -> String {
    let all = sources.iter().map(String::as_str).chain([shell]);
    encre_css::generate(all, &encre_css::Config::default())
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
        let css = generate(&["<div class=\"p-4 text-sm\">".into()], "");
        assert!(css.contains(".p-4"), "{css}");
        assert!(css.contains("padding: 1rem"), "{css}");
    }

    #[test]
    fn minify_drops_comments_and_indentation() {
        let css = "/* reset */\n.a,\n.b {\n  color: red;\n}\n";
        assert_eq!(minify(css), ".a,.b {color: red;}");
    }
}
