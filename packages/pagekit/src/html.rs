//! The shell: fill the slots, set the title and icon, and add the Content-Security-Policy.

use std::fmt::Write as _;

use anyhow::{Context, Result};
use base64::Engine as _;
use sha2::{Digest, Sha256};

use crate::{CSS_SLOT, Csp, JS_SLOT};

/// The shell with the sheet in its css slot and the script in its js slot.
#[must_use]
pub fn fill(shell: &str, sheet: &str, script: &str) -> String {
    shell
        .replace(CSS_SLOT, &format!("<style>{sheet}</style>"))
        .replace(JS_SLOT, &format!("<script>{script}</script>"))
}

/// Replace the `<title>` (or add one), and the icon links with `favicon` as a data URI.
pub fn head(html: &str, title: Option<&str>, favicon: Option<&str>) -> Result<String> {
    let mut html = html.to_owned();
    if let Some(title) = title {
        html = cut(&html, "<title", "</title>");
        let title = title.replace('&', "&amp;").replace('<', "&lt;");
        html = before_head_end(&html, &format!("<title>{title}</title>"))?;
    }
    if let Some(svg) = favicon {
        while html.contains("<link rel=\"icon\"") {
            html = cut(&html, "<link rel=\"icon\"", ">");
        }
        let uri = svg_data_uri(svg);
        html = before_head_end(&html, &format!("<link rel=\"icon\" href=\"{uri}\">"))?;
    }
    Ok(html)
}

/// Add the policy as the first element of `<head>`, so it governs everything after it.
///
/// Scripts are allowed by the hash of each inline `<script>`; styles by the hash of each
/// `<style>`, or `'unsafe-inline'` when the markup has `style` attributes, which no hash covers.
pub fn secure(html: &str, csp: &Csp, wasm: bool) -> Result<String> {
    let hashes = |tag: &str| {
        bodies(html, tag).fold(String::new(), |mut out, body| {
            let _ = write!(out, " '{}'", hash(body));
            out
        })
    };
    let wasm = if wasm { " 'wasm-unsafe-eval'" } else { "" };
    let script = format!("{wasm}{}", hashes("script"));
    let style = if html.contains(" style=\"") {
        " 'unsafe-inline'".to_owned()
    } else {
        hashes("style")
    };
    let list = |defaults: &str, extra: &[String]| {
        std::iter::once(defaults.to_owned())
            .chain(extra.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let policy = format!(
        "default-src 'none'; script-src{script}; style-src{style}; img-src {}; font-src {}; \
         connect-src {}; frame-src {}; media-src {}; worker-src 'self' blob:; \
         manifest-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'",
        list("'self' data: blob:", &csp.img),
        list("'self' data:", &csp.font),
        list("'self'", &csp.connect),
        list("'self' blob:", &csp.frame),
        list("'self' blob:", &csp.media),
    );
    into_head(
        html,
        &format!("<meta http-equiv=\"Content-Security-Policy\" content=\"{policy}\">"),
    )
}

/// `sha256-<base64>`: how a policy names an inline script or style.
#[must_use]
pub fn hash(body: &str) -> String {
    let digest = Sha256::digest(body.as_bytes());
    format!(
        "sha256-{}",
        base64::engine::general_purpose::STANDARD.encode(digest)
    )
}

/// The text of every `<tag>` without a `src`, exactly as the browser hashes it.
pub fn bodies<'a>(html: &'a str, tag: &str) -> impl Iterator<Item = &'a str> {
    let open = format!("<{tag}");
    let close = format!("</{tag}");
    let mut rest = html;
    std::iter::from_fn(move || {
        loop {
            let start = rest.find(&open)?;
            let attrs_end = start + rest[start..].find('>')?;
            let body_end = attrs_end + rest[attrs_end..].find(&close)?;
            let attrs = &rest[start..attrs_end];
            let body = &rest[attrs_end + 1..body_end];
            rest = &rest[body_end..];
            if !attrs.contains(" src=") {
                return Some(body);
            }
        }
    })
}

/// `html` with `text` right after the opening `<head>` tag.
fn into_head(html: &str, text: &str) -> Result<String> {
    let open = html.find("<head").context("the shell has no <head>")?;
    let at = open
        + html[open..]
            .find('>')
            .context("the <head> tag does not close")?
        + 1;
    Ok(format!("{}\n{text}{}", &html[..at], &html[at..]))
}

/// `html` with `text` on its own line before `</head>`.
fn before_head_end(html: &str, text: &str) -> Result<String> {
    let at = html.find("</head>").context("the shell has no </head>")?;
    Ok(format!("{}{text}\n{}", &html[..at], &html[at..]))
}

/// `html` without the first span from `start` to the end of `end`.
fn cut(html: &str, start: &str, end: &str) -> String {
    let Some(from) = html.find(start) else {
        return html.to_owned();
    };
    let to = html[from..]
        .find(end)
        .map_or(html.len(), |i| from + i + end.len());
    format!("{}{}", &html[..from], &html[to..])
}

/// An SVG as a `data:` URI without base64: a third smaller.
fn svg_data_uri(svg: &str) -> String {
    let mut out = String::from("data:image/svg+xml,");
    for c in svg.trim().chars() {
        match c {
            '"' => out.push('\''),
            '#' => out.push_str("%23"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_policy_pins_each_inline_script_by_hash() {
        let html = "<html><head></head><body><script>a()</script><script src=x></script></body>";
        let out = secure(html, &Csp::default(), false).unwrap();
        assert!(out.contains(&format!("script-src '{}';", hash("a()"))));
        assert!(!out.contains("wasm-unsafe-eval"));
        assert!(
            secure(html, &Csp::default(), true)
                .unwrap()
                .contains("'wasm-unsafe-eval'")
        );
    }

    #[test]
    fn a_style_attribute_needs_unsafe_inline() {
        let html = "<head></head><style>p{}</style><p style=\"color:red\"></p>";
        let out = secure(html, &Csp::default(), false).unwrap();
        assert!(out.contains("style-src 'unsafe-inline';"));
    }

    #[test]
    fn the_title_and_icon_are_replaced() {
        let html = "<head><title>old</title><link rel=\"icon\" href=\"/x.svg\"></head>";
        let out = head(html, Some("New"), Some("<svg fill=\"#000\"/>")).unwrap();
        assert!(out.contains("<title>New</title>") && !out.contains("old"));
        assert!(out.contains("href=\"data:image/svg+xml,%3Csvg fill='%23000'/%3E\""));
        assert!(!out.contains("/x.svg"));
    }
}
