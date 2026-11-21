//! `*.test.ts` files run in boa, an embedded JavaScript engine: no browser, no Node.
//!
//! A test file calls `test(name, fn)` and `assert(condition, message)`; both are globals. The
//! Preact runtime is loaded first, so a test may import from `preact` and `preact/hooks`.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use boa_engine::{Context, Source};

use crate::{RUNTIME, bundle};

/// The globals a test file may read: what [`HARNESS`] defines.
const GLOBALS: &[&str] = &["test", "assert"];

/// Defines `test` and `assert` before the test file runs.
const HARNESS: &str = "globalThis.self = globalThis; globalThis.window = globalThis;
var __tests = [];
function test(name, fn) { __tests.push([name, fn]); }
function assert(ok, message) { if (!ok) throw new Error(message || 'assertion failed'); }";

/// Runs the registered tests, one result line each.
const RUN: &str = "__tests.map(function (t) {
  try { t[1](); return 'ok   ' + t[0]; } catch (e) { return 'FAIL ' + t[0] + ': ' + e.message; }
}).join('\\n')";

/// One test file's result.
#[derive(Debug)]
pub struct Outcome {
    /// The test file, relative to the page.
    pub file: PathBuf,
    /// One line per test: `ok   name` or `FAIL name: message`.
    pub lines: Vec<String>,
}

impl Outcome {
    /// Whether every test passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.lines.iter().all(|l| l.starts_with("ok"))
    }
}

/// Every `*.test.ts` and `*.test.tsx` under `root/src`, each bundled and run on its own.
pub fn run(root: &Path) -> Result<Vec<Outcome>> {
    let mut out = vec![];
    for path in crate::css::files(&root.join("src"))? {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !(name.ends_with(".test.ts") || name.ends_with(".test.tsx")) {
            continue;
        }
        let file = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        let options = bundle::Options {
            minify: false,
            host: "globalThis",
            wasm: false,
            glue: None,
            globals: GLOBALS,
        };
        let script = bundle::bundle(root, &file, &options)?;
        let mut context = Context::default();
        let mut eval = |code: &str| {
            context
                .eval(Source::from_bytes(code))
                .map_err(|e| anyhow!("{}: {e}", file.display()))
        };
        for code in [HARNESS, RUNTIME, &script] {
            eval(code)?;
        }
        let result = eval(RUN)?;
        let text = result
            .as_string()
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        let lines = text.lines().map(str::to_owned).collect();
        out.push(Outcome { file, lines });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hello_example_tests_pass() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
        let outcomes = run(&root).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert!(outcomes[0].passed(), "{:?}", outcomes[0].lines);
        assert_eq!(outcomes[0].lines.len(), 2);
    }
}
