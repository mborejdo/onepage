//! The example pages build, and each check fails a page with its one mistake.

#![allow(
    clippy::unwrap_used,
    reason = "test helpers, not #[test] functions, so clippy.toml's exemption misses them"
)]

use std::path::{Path, PathBuf};

use pagekit::{Finding, html};

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// Build `dir` into a fresh temporary folder; returns the HTML and the folder.
fn build(dir: &Path, name: &str) -> (String, PathBuf) {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&out);
    let file = onepage::build(dir, Some(&out.join("index.html")), true).unwrap();
    (std::fs::read_to_string(file).unwrap(), out)
}

#[test]
fn hello_builds_one_file_with_a_pinned_script_and_its_utilities() {
    let (page, _) = build(&examples().join("hello"), "hello");
    let scripts: Vec<_> = html::bodies(&page, "script").collect();
    assert_eq!(scripts.len(), 1);
    assert!(page.contains(&format!("script-src '{}';", html::hash(scripts[0]))));
    let styles: Vec<_> = html::bodies(&page, "style").collect();
    assert!(page.contains(&format!("style-src '{}';", html::hash(styles[0]))));
    assert!(styles[0].contains(".p-4 {padding: 1rem;}"), "{}", styles[0]);
    assert!(
        styles[0].contains("button:hover"),
        "the page's own sheet comes after"
    );
    assert!(page.contains("<title>Hello</title>") && !page.contains("<title>hello</title>"));
    assert!(page.contains("<link rel=\"icon\" href=\"data:image/svg+xml,"));
    assert!(!page.contains("wasm-unsafe-eval"));
}

#[test]
fn the_wasm_page_gets_wasm_unsafe_eval_and_its_file() {
    let (page, out) = build(&examples().join("wasm"), "wasm");
    assert!(page.contains("script-src 'wasm-unsafe-eval' 'sha256-"));
    assert!(out.join("add.wasm").is_file());
}

/// Build a one-file page with `app` as `src/app.tsx` and `config` as `onepage.toml`.
fn finding(name: &str, config: &str, app: &str) -> Finding {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("pages")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let shell = "<html><head><!--css--></head><body><!--js--></body></html>";
    std::fs::write(dir.join("index.html"), shell).unwrap();
    std::fs::write(dir.join("onepage.toml"), config).unwrap();
    std::fs::write(dir.join("src/app.tsx"), app).unwrap();
    std::fs::write(dir.join("../outside.ts"), "export const x = 1;\n").unwrap();
    let error = onepage::build(&dir, None, true).unwrap_err();
    assert_eq!(onepage::report(&error).1, 1, "{error:#}");
    error
        .chain()
        .find_map(|e| e.downcast_ref::<Finding>())
        .unwrap()
        .clone()
}

#[test]
fn jsx_without_h_fails_at_its_line() {
    let f = finding(
        "no-h",
        "",
        "import { render } from \"preact\";\n\nrender(<p />, document.body);\n",
    );
    assert_eq!((f.file, f.line), (PathBuf::from("src/app.tsx"), 3));
    assert!(f.message.contains("does not import `h`"), "{}", f.message);
}

#[test]
fn a_fragment_without_its_import_fails() {
    let f = finding(
        "no-fragment",
        "",
        "import { h } from \"preact\";\nexport const a = <>x</>;\n",
    );
    assert!(f.message.contains("`Fragment`"), "{}", f.message);
}

#[test]
fn an_undeclared_name_fails_and_webassembly_needs_the_opt_in() {
    let f = finding("undeclared", "", "export const a = conut + 1;\n");
    assert!(
        f.message.contains("`conut` is not defined"),
        "{}",
        f.message
    );
    let f = finding("no-wasm", "", "export const a = WebAssembly.validate;\n");
    assert!(f.message.contains("[wasm]"), "{}", f.message);
    let ok = "[wasm]\n";
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pages/no-wasm");
    std::fs::write(dir.join("onepage.toml"), ok).unwrap();
    onepage::build(&dir, None, true).unwrap();
}

#[test]
fn import_meta_fails() {
    let f = finding("meta", "", "export const u = import.meta.url;\n");
    assert!(f.message.contains("`import.meta`"), "{}", f.message);
}

#[test]
fn an_import_outside_the_page_or_from_a_package_fails() {
    let f = finding(
        "escape",
        "",
        "import { x } from \"../../outside\";\nexport const y = x;\n",
    );
    assert!(
        f.message.contains("outside the page directory"),
        "{}",
        f.message
    );
    assert_eq!(f.line, 1);
    let f = finding(
        "package",
        "",
        "import confetti from \"confetti\";\nconfetti();\n",
    );
    assert!(f.message.contains("only relative paths"), "{}", f.message);
}
