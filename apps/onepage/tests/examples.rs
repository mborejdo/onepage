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
fn the_kit_page_gets_the_tokens_the_kit_and_its_sheet() {
    let (page, _) = build(&examples().join("kit"), "kit");
    let styles: Vec<_> = html::bodies(&page, "style").collect();
    let sheet = styles[0];
    assert!(
        sheet.starts_with(":root{color-scheme:light dark;"),
        "tokens first"
    );
    assert!(sheet.contains("[data-theme=dark]{color-scheme:dark;--ground:#17191f;"));
    assert!(sheet.contains(".pk-flap::after"), "the kit's base.css");
    for rule in [
        ".bg-surface {",
        ".text-muted {",
        ".bg-series-6 {",
        ".border-border {",
    ] {
        assert!(sheet.contains(rule), "{rule}");
    }
    let base = sheet.find(".pk-flap::after").unwrap();
    assert!(
        base < sheet.find(".bg-surface {").unwrap(),
        "utilities after the base"
    );
    assert!(page.contains("window.preactJsx"));
}

/// A page folder in the test's temporary folder, with `files` written into it.
fn page(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("pages")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    let shell = "<html><head><!--css--></head><body><!--js--></body></html>";
    for (path, text) in std::iter::once(&("index.html", shell)).chain(files) {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

/// Vendored packages: an `exports` map, a `module` field, a package importing another by
/// its bare name and itself relatively, the automatic JSX runtime and a namespace import.
const VENDORED: &[(&str, &str)] = &[
    ("onepage.toml", "vendor = \"vendor\"\n"),
    (
        "vendor/alpha/package.json",
        r#"{"type":"module","exports":{".":{"types":"./a.d.ts","import":"./dist/index.js"}}}"#,
    ),
    (
        "vendor/alpha/dist/index.js",
        "import { twice } from \"@scope/beta\";\nimport * as P from \"preact\";\n\
         import { jsx } from \"preact/jsx-runtime\";\n\
         export const four = twice(2);\nexport const preact = P;\n\
         export const node = () => jsx(\"b\", { children: \"x\" }, \"k\");\n",
    ),
    (
        "vendor/@scope/beta/package.json",
        r#"{"main":"dist/beta.cjs","module":"src/index.js"}"#,
    ),
    (
        "vendor/@scope/beta/src/index.js",
        "export { twice } from \"./lib/twice.js\";\n",
    ),
    (
        "vendor/@scope/beta/src/lib/twice.js",
        "export const twice = (n) => n * 2;\n",
    ),
    (
        "src/app.tsx",
        "import { h, render } from \"preact\";\nimport { four } from \"alpha\";\n\
         render(<p class=\"p-4\">{four}</p>, document.body);\n",
    ),
    (
        "src/one.test.ts",
        "import { h } from \"preact\";\nimport { four, node, preact } from \"alpha\";\n\
         test(\"one preact\", () => {\n  assert(four === 4, \"bare and relative imports\");\n\
           assert(preact.h === h, \"the package shares the page's preact\");\n\
           assert(node().key === \"k\" && node().props.children === \"x\", \"jsx\");\n});\n",
    ),
];

#[test]
fn vendored_packages_resolve_and_share_one_preact() {
    let dir = page("vendored", VENDORED);
    let out = onepage::build(&dir, None, true).unwrap();
    let html = std::fs::read_to_string(out).unwrap();
    assert!(html.contains(".p-4 {"), "the page's own classes");
    #[cfg(feature = "test")]
    {
        let config = onepage::config::Config::load(&dir).unwrap();
        let outcomes = pagekit::test::run(&config.page(&dir, false)).unwrap();
        assert!(outcomes[0].passed(), "{:?}", outcomes[0].lines);
    }
}

#[test]
fn an_import_map_names_a_kit_and_a_missing_package_fails() {
    let dir = page(
        "mapped",
        &[
            ("onepage.toml", "[imports]\nkit = \"../kit-folder\"\n"),
            (
                "../kit-folder/package.json",
                r#"{"exports":{".":"./src/index.ts","./parts":"./src/parts.tsx"}}"#,
            ),
            (
                "../kit-folder/src/index.ts",
                "export const label = \"text-ok\";\n",
            ),
            (
                "../kit-folder/src/parts.tsx",
                "import { h } from \"preact\";\nexport const Box = () => <i class=\"m-7\" />;\n",
            ),
            (
                "src/app.tsx",
                "import { h, render } from \"preact\";\nimport { label } from \"kit\";\n\
                 import { Box } from \"kit/parts\";\nrender(<Box />, document.body);\nexport { label };\n",
            ),
        ],
    );
    let out = onepage::build(&dir, None, true).unwrap();
    let html = std::fs::read_to_string(out).unwrap();
    assert!(
        html.contains(".m-7 {"),
        "a kit module's classes are scanned"
    );
    let f = finding(
        "unknown-package",
        "[imports]\nkit = \"../kit-folder\"\n",
        "import { x } from \"kit/nothing\";\nexport const y = x;\n",
    );
    assert!(
        f.message.contains("does not export `./nothing`"),
        "{}",
        f.message
    );
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
