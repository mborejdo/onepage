//! Type checks with tsrs (a Rust port of the TypeScript 7 checker), in process: tsc's own
//! errors and the type-aware rules that catch a page that builds clean and fails silently.
//!
//! The program is the page's own sources (as the lint takes them), its mapped packages and
//! vendored packages' typings (by their bare names), Preact's typings (inlined, from the
//! runtime's npm package) and the `test`/`assert` globals. The `tsconfig.json` is made in
//! memory, so nothing is written to the page: strict, classic JSX under `h` and `Fragment`,
//! the DOM libraries.
//!
//! A [`Backlog`] (`[types]` in `onepage.toml`) is a ratchet: per code, how many findings the
//! page may have. A code over its count fails; without a backlog, every finding is a note.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::{Value, json};

use crate::Page;
use crate::lint::{Finding, Report};

/// Per code (`TS2322`, `typescript/no-floating-promises`), the findings a page may have.
pub type Backlog = BTreeMap<String, usize>;

/// The type-aware rules that run, as tsrs names them: each one is a way a page fails at
/// runtime with nothing in the console that points at it.
pub const RULES: &[&str] = &[
    "no-floating-promises",
    "no-misused-promises",
    "await-thenable",
    "unbound-method",
    "switch-exhaustiveness-check",
    "restrict-template-expressions",
];

/// Preact's typings, by their path in the package.
const PREACT: &[(&str, &str)] = &[
    (
        "src/index.d.ts",
        include_str!("../assets/types/preact/src/index.d.ts"),
    ),
    (
        "src/jsx.d.ts",
        include_str!("../assets/types/preact/src/jsx.d.ts"),
    ),
    (
        "src/dom.d.ts",
        include_str!("../assets/types/preact/src/dom.d.ts"),
    ),
    (
        "hooks/src/index.d.ts",
        include_str!("../assets/types/preact/hooks/src/index.d.ts"),
    ),
    (
        "jsx-runtime/src/index.d.ts",
        include_str!("../assets/types/preact/jsx-runtime/src/index.d.ts"),
    ),
];

/// The bare names Preact's typings answer.
const PREACT_NAMES: &[(&str, &str)] = &[
    ("preact", "src/index.d.ts"),
    ("preact/hooks", "hooks/src/index.d.ts"),
    ("preact/jsx-runtime", "jsx-runtime/src/index.d.ts"),
    ("preact/jsx-dev-runtime", "jsx-runtime/src/index.d.ts"),
];

/// The globals `onepage test` defines.
const TEST_GLOBALS: &str = "declare function test(name: string, fn: () => void): void;
declare function assert(ok: unknown, message?: string): asserts ok;
";

/// Where the in-memory files live, under the page's root.
const VIRTUAL: &str = ".onepage-types";

/// Type-check `page`: every finding of its own files, each a note unless `backlog` says its
/// code is over its count.
pub fn page(page: &Page, backlog: Option<&Backlog>) -> Result<Report> {
    let root = page
        .root
        .canonicalize()
        .with_context(|| format!("resolve {}", page.root.display()))?;
    let own = own_files(page, &root)?;
    let virtual_dir = root.join(VIRTUAL);
    let mut overrides = serde_json::Map::new();
    let mut paths = serde_json::Map::new();
    for (path, text) in PREACT {
        let file = virtual_dir.join("preact").join(path);
        overrides.insert(file.to_string_lossy().into_owned(), json!(text));
    }
    for (name, path) in PREACT_NAMES {
        let file = virtual_dir.join("preact").join(path);
        paths.insert((*name).to_owned(), json!([file.to_string_lossy()]));
    }
    let imports = page.resolver();
    // The page's bare imports, then every vendored package by name: a package's typings
    // import its dependencies by their bare names too.
    let vendored = imports.vendor.as_deref().map_or_default(packages_in);
    for spec in bare_imports(&own)?.into_iter().chain(vendored) {
        if paths.contains_key(&spec) {
            continue;
        }
        // tsconfig `paths` must be absolute or `./`-relative.
        if let Some(types) = imports.types(&spec).and_then(|t| t.canonicalize().ok()) {
            paths.insert(spec, json!([types.to_string_lossy()]));
        }
    }
    let globals = virtual_dir.join("onepage.d.ts");
    overrides.insert(globals.to_string_lossy().into_owned(), json!(TEST_GLOBALS));
    let mut files: Vec<String> = own
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    files.push(globals.to_string_lossy().into_owned());
    let tsconfig = json!({
        "compilerOptions": {
            "strict": true,
            "noEmit": true,
            "target": "es2022",
            "module": "esnext",
            "moduleResolution": "bundler",
            "lib": ["es2023", "dom", "dom.iterable"],
            "jsx": "react",
            "jsxFactory": "h",
            "jsxFragmentFactory": "Fragment",
            "skipLibCheck": true,
            "allowImportingTsExtensions": true,
            "types": [],
            "paths": paths,
        },
        "files": files,
    });
    overrides.insert(
        root.join("tsconfig.json").to_string_lossy().into_owned(),
        json!(tsconfig.to_string()),
    );
    let diagnostics = run(&root, &own, &overrides)?;
    let own: BTreeSet<PathBuf> = own.into_iter().collect();
    let mut findings = diagnostics
        .into_iter()
        .filter_map(|d| finding(&root, &own, &d))
        .collect::<Vec<_>>();
    findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(ratchet(findings, backlog))
}

/// tsrs's engine over `own`, with `overrides` as files in memory: its diagnostics.
fn run(
    root: &Path,
    own: &[PathBuf],
    overrides: &serde_json::Map<String, Value>,
) -> Result<Vec<Value>> {
    let rules: Vec<Value> = RULES.iter().map(|r| json!({ "name": r })).collect();
    let own_paths: Vec<String> = own
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let payload = json!({
        "version": 2,
        "configs": [{ "file_paths": own_paths, "rules": rules }],
        "source_overrides": overrides,
        "report_syntactic": true,
        "report_semantic": true,
    });
    let payload = tsrs_lint::payload::deserialize_payload(&payload.to_string())
        .map_err(|e| anyhow::anyhow!("tsrs: {e}"))?;
    let mut out = Vec::new();
    tsrs_lint::engine::run(
        &payload,
        tsrs_lint::engine::Options::default(),
        &root.to_string_lossy(),
        &mut out,
    )
    .map_err(|e| anyhow::anyhow!("tsrs: {e}"))?;
    let diagnostics = frames(&out)?;
    // The tsconfig is onepage's: a mistake in it is onepage's, and the program never ran.
    let tsconfig = root.join("tsconfig.json");
    if let Some(d) = diagnostics
        .iter()
        .find(|d| d.get("file_path").and_then(Value::as_str) == tsconfig.to_str())
    {
        bail!(
            "tsrs: the generated tsconfig.json: {}",
            d.pointer("/message/description")
                .and_then(Value::as_str)
                .unwrap_or_default()
        );
    }
    Ok(diagnostics)
}

/// The `.ts` and `.tsx` files the lint takes as the page's own: the entry's folder, without
/// `vendor` and the `[wasm] glue`.
fn own_files(page: &Page, root: &Path) -> Result<Vec<PathBuf>> {
    let skip: Vec<PathBuf> = page
        .vendor
        .iter()
        .chain(page.wasm.as_ref().and_then(|w| w.glue.as_ref()))
        .map(|p| root.join(p))
        .collect();
    let own_dir = root.join(page.entry.parent().unwrap_or_else(|| Path::new("")));
    Ok(crate::css::files(&own_dir)?
        .into_iter()
        .filter(|p| !skip.iter().any(|s| p.starts_with(s)))
        .filter(|p| !p.to_string_lossy().ends_with(".d.ts"))
        .collect())
}

/// The package names in a vendor folder: `name` and `@scope/name`.
fn packages_in(vendor: &Path) -> Vec<String> {
    let mut names = vec![];
    let entries = |dir: &Path| {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    };
    for name in entries(vendor) {
        if name.starts_with('@') {
            for inner in entries(&vendor.join(&name)) {
                names.push(format!("{name}/{inner}"));
            }
        } else {
            names.push(name);
        }
    }
    names.sort();
    names
}

/// Every bare specifier the files import or re-export from.
fn bare_imports(files: &[PathBuf]) -> Result<BTreeSet<String>> {
    let mut specs = BTreeSet::new();
    for path in files {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let allocator = Allocator::default();
        let source = SourceType::from_path(path).unwrap_or_else(|_| SourceType::tsx());
        let parsed = Parser::new(&allocator, &text, source).parse();
        for spec in parsed.module_record.requested_modules.keys() {
            let spec = spec.as_str();
            if !spec.starts_with('.') && !spec.starts_with('/') {
                specs.insert(spec.to_owned());
            }
        }
    }
    Ok(specs)
}

/// The engine's output: frames of a little-endian `u32` length, a type byte (0 an error,
/// 1 a diagnostic, 2 timings) and that many bytes of JSON. Returns the diagnostics.
fn frames(mut out: &[u8]) -> Result<Vec<Value>> {
    let mut diagnostics = vec![];
    while !out.is_empty() {
        let Some((head, rest)) = out.split_at_checked(5) else {
            bail!("tsrs: a frame is cut short");
        };
        let len = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
        let len = usize::try_from(len).context("tsrs: a frame's length")?;
        let Some((body, rest)) = rest.split_at_checked(len) else {
            bail!("tsrs: a frame is cut short");
        };
        let value: Value = serde_json::from_slice(body).context("tsrs: a frame's JSON")?;
        match head[4] {
            0 => bail!(
                "tsrs: {}",
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("failed")
            ),
            1 => diagnostics.push(value),
            _ => {},
        }
        out = rest;
    }
    Ok(diagnostics)
}

/// A diagnostic of one of the page's own files as a finding (an error until [`ratchet`]).
fn finding(root: &Path, own: &BTreeSet<PathBuf>, d: &Value) -> Option<Finding> {
    let path = PathBuf::from(d.get("file_path")?.as_str()?);
    if !own.contains(&path) {
        return None;
    }
    let rule = match d.get("rule").and_then(Value::as_str) {
        Some(rule) => format!("typescript/{rule}"),
        None => d.pointer("/message/id")?.as_str()?.to_owned(),
    };
    let message = d
        .pointer("/message/description")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let at = d
        .pointer("/range/pos")
        .and_then(Value::as_u64)
        .and_then(|p| usize::try_from(p).ok());
    let line = at.map_or(0, |at| {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let at = at.min(text.len());
        text.get(..at).map_or(0, |t| t.matches('\n').count()) + 1
    });
    Some(Finding {
        file: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
        line,
        rule,
        message: message.split_whitespace().collect::<Vec<_>>().join(" "),
        notice: false,
    })
}

/// Each code's findings are errors when there are more than `backlog` allows, else notes;
/// without a backlog all are notes. A code under its count gets a note to lower it.
fn ratchet(mut findings: Vec<Finding>, backlog: Option<&Backlog>) -> Report {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in &findings {
        *counts.entry(f.rule.clone()).or_default() += 1;
    }
    for f in &mut findings {
        f.notice = backlog.is_none_or(|b| counts[&f.rule] <= b.get(&f.rule).copied().unwrap_or(0));
    }
    for (code, &allowed) in backlog.into_iter().flatten() {
        let found = counts.get(code).copied().unwrap_or(0);
        if found < allowed {
            findings.push(Finding {
                file: PathBuf::from("onepage.toml"),
                line: 0,
                rule: code.clone(),
                message: format!("[types] allows {allowed}, {found} are left: lower it to {found}"),
                notice: true,
            });
        }
    }
    Report {
        findings,
        formatted: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, files: &[(&str, &str)]) {
        for (path, text) in files {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
    }

    #[test]
    fn a_floating_promise_and_a_type_error_are_found_and_ratcheted() {
        let dir = std::env::temp_dir().join(format!("pagekit-types-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write(
            &dir,
            &[
                (
                    "src/app.tsx",
                    "import { h, render } from \"preact\";\nimport { useState } from \"preact/hooks\";\n\
                     const wait = async () => 1;\n// ä, a byte more than one\nwait();\n\
                     const n: number = \"x\";\n\
                     const App = () => { const [v] = useState(0); return <p>{v}{n}</p>; };\n\
                     render(<App />, document.body);\n",
                ),
                (
                    "src/app.test.ts",
                    "test(\"one\", () => assert(1 + 1 === 2, \"sum\"));\n",
                ),
            ],
        );
        let page = Page::new(&dir);
        let report = super::page(&page, None).unwrap();
        let lines: Vec<String> = report.findings.iter().map(ToString::to_string).collect();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(
            lines[0].starts_with("src/app.tsx:5: note: typescript/no-floating-promises: "),
            "{lines:?}"
        );
        assert!(
            lines[1].starts_with("src/app.tsx:6: note: TS2322: "),
            "{lines:?}"
        );
        let backlog = Backlog::from([("TS2322".into(), 1), ("TS2307".into(), 2)]);
        let report = super::page(&page, Some(&backlog)).unwrap();
        assert_eq!(report.errors(), 1, "{report}");
        assert!(
            report
                .to_string()
                .contains("onepage.toml:0: note: TS2307: [types] allows 2, 0 are left")
        );
    }

    #[test]
    fn a_relative_page_with_a_mapped_kit_type_checks_clean() {
        // The tests run in packages/pagekit: a root relative to it, as `onepage types kit` is.
        let mut page = Page::new("../../examples/kit");
        page.imports
            .insert("panelkit".into(), "../../packages/panelkit".into());
        let report = super::page(&page, None).unwrap();
        assert!(report.findings.is_empty(), "{report}");
    }
}
