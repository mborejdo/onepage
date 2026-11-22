//! Bare imports: `import { Card } from "panelkit"`, `import { scaleLinear } from "d3-scale"`.
//!
//! A bare specifier is a package name (`name` or `@scope/name`) and an optional subpath
//! (`panelkit/mcp`). It resolves, in order:
//!
//! 1. through `[imports]`, the page's import map: the whole specifier or its package name to a
//!    package folder or to one file. A file is the entry itself; a folder is read like a
//!    vendored package below;
//! 2. through the vendor folder, a tree of unpacked npm packages: `<vendor>/<name>/`.
//!
//! A package folder's `package.json` says the entry: `exports` (a string, a subpath map with
//! `*` patterns, conditions `browser`, `import`, `module` and `default`, in that order of
//! preference), else `module`, else `main`, else `index`. A subpath of a package without
//! `exports` is a file in it. A path without an extension tries `.ts`, `.tsx`, `.js`, `.mjs`
//! and `.jsx`, then an `index` with each.
//!
//! No network and no install: the files are on disk already, as `npm pack` unpacks them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// The extensions tried, in order, for a path that names none.
const EXTENSIONS: &[&str] = &["ts", "tsx", "js", "mjs", "jsx"];

/// The `exports` conditions taken, in order of preference: a page is an ES module bundle in
/// a browser.
const CONDITIONS: &[&str] = &["browser", "import", "module", "default"];

/// Where bare imports resolve: the import map and the vendor folder, as absolute paths.
#[derive(Debug, Clone, Default)]
pub struct Imports {
    /// A specifier or a package name to a package folder or a file.
    pub map: BTreeMap<String, PathBuf>,
    /// A folder of unpacked npm packages.
    pub vendor: Option<PathBuf>,
}

impl Imports {
    /// The folders whose files a page may import: every mapped folder (a mapped file's
    /// folder) and the vendor folder.
    #[must_use]
    pub fn roots(&self) -> Vec<PathBuf> {
        let mapped = self.map.values().map(|p| {
            let dir = if p.is_file() {
                p.parent().unwrap_or(p)
            } else {
                p
            };
            dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf())
        });
        let vendor = self
            .vendor
            .iter()
            .map(|v| v.canonicalize().unwrap_or_else(|_| v.clone()));
        mapped.chain(vendor).collect()
    }

    /// The package folders of the import map (not the vendor folder): their modules are the
    /// page's own code, checked and scanned for class names.
    #[must_use]
    pub fn kits(&self) -> Vec<PathBuf> {
        self.map
            .values()
            .filter(|p| p.is_dir())
            .map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()))
            .filter(|p| {
                self.vendor
                    .as_ref()
                    .and_then(|v| v.canonicalize().ok())
                    .is_none_or(|v| !p.starts_with(v))
            })
            .collect()
    }

    /// The file a bare `spec` names, or `None` when no mapping and no vendored package has
    /// its name.
    pub fn resolve(&self, spec: &str) -> Result<Option<PathBuf>> {
        let (name, sub) = split(spec);
        if let Some(target) = self.map.get(spec) {
            return mapped(target, "").map(Some);
        }
        if let Some(target) = self.map.get(name) {
            if target.is_file() && !sub.is_empty() {
                bail!("`{name}` is mapped to one file, so `{spec}` names nothing");
            }
            return mapped(target, sub).map(Some);
        }
        let Some(vendor) = &self.vendor else {
            return Ok(None);
        };
        let dir = vendor.join(name);
        if !dir.join("package.json").is_file() {
            return Ok(None);
        }
        package(&dir, sub).map(Some)
    }
}

/// The sheet a package folder brings, by its package.json's `style` field (the convention of
/// CSS-aware bundlers), or `None`.
pub fn style(dir: &Path) -> Result<Option<PathBuf>> {
    let manifest = dir.join("package.json");
    if !manifest.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&manifest)
        .with_context(|| format!("read {}", manifest.display()))?;
    let json: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", manifest.display()))?;
    Ok(json
        .get("style")
        .and_then(Value::as_str)
        .map(|s| dir.join(s)))
}

/// `@scope/name/sub/path` as `("@scope/name", "sub/path")`; `name` as `("name", "")`.
fn split(spec: &str) -> (&str, &str) {
    let segments = if spec.starts_with('@') { 2 } else { 1 };
    let mut at = 0;
    for _ in 0..segments {
        match spec[at..].find('/') {
            Some(i) => at += i + 1,
            None => return (spec, ""),
        }
    }
    (&spec[..at - 1], &spec[at..])
}

/// A mapped target: a file is the entry, a folder a package.
fn mapped(target: &Path, sub: &str) -> Result<PathBuf> {
    if target.is_file() {
        return Ok(target.to_path_buf());
    }
    if !target.is_dir() {
        bail!("[imports]: {} does not exist", target.display());
    }
    package(target, sub)
}

/// The file `sub` (or the entry, for `""`) of the package in `dir`.
fn package(dir: &Path, sub: &str) -> Result<PathBuf> {
    let manifest = dir.join("package.json");
    let json: Value = if manifest.is_file() {
        let text = std::fs::read_to_string(&manifest)
            .with_context(|| format!("read {}", manifest.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parse {}", manifest.display()))?
    } else {
        Value::Null
    };
    let key = if sub.is_empty() {
        ".".to_owned()
    } else {
        format!("./{sub}")
    };
    let target = if let Some(exports) = json.get("exports") {
        export(exports, &key).with_context(|| {
            format!(
                "{} does not export `{key}` for a browser module",
                manifest.display()
            )
        })?
    } else if sub.is_empty() {
        ["module", "main"]
            .iter()
            .find_map(|k| json.get(k).and_then(Value::as_str))
            .unwrap_or("index")
            .to_owned()
    } else {
        sub.to_owned()
    };
    let path = dir.join(target.trim_start_matches("./"));
    probe(&path).with_context(|| format!("{} names no file", path.display()))
}

/// The target of `key` (`.` or `./sub`) in an `exports` value.
fn export(exports: &Value, key: &str) -> Option<String> {
    match exports {
        Value::Object(map) if map.keys().any(|k| k.starts_with('.')) => {
            if let Some(value) = map.get(key) {
                return condition(value);
            }
            map.iter().find_map(|(pattern, value)| {
                let (prefix, suffix) = pattern.split_once('*')?;
                let stem = key.strip_prefix(prefix)?.strip_suffix(suffix)?;
                condition(value).map(|t| t.replace('*', stem))
            })
        },
        _ if key == "." => condition(exports),
        _ => None,
    }
}

/// A conditional target, reduced to a path by [`CONDITIONS`].
fn condition(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Array(list) => list.iter().find_map(condition),
        Value::Object(map) => CONDITIONS
            .iter()
            .find_map(|c| map.get(*c).and_then(condition)),
        _ => None,
    }
}

/// `path` as a file: itself, with an extension, or its folder's `index`.
fn probe(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    let with = |base: &Path| {
        EXTENSIONS.iter().find_map(|ext| {
            let mut name = base.as_os_str().to_owned();
            name.push(".");
            name.push(ext);
            let p = PathBuf::from(name);
            p.is_file().then_some(p)
        })
    };
    with(path).or_else(|| with(&path.join("index")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn a_name_splits_from_its_subpath() {
        assert_eq!(split("preact"), ("preact", ""));
        assert_eq!(split("preact/hooks"), ("preact", "hooks"));
        assert_eq!(split("@tanstack/charts"), ("@tanstack/charts", ""));
        assert_eq!(
            split("@tanstack/charts/scales/linear"),
            ("@tanstack/charts", "scales/linear")
        );
    }

    #[test]
    fn exports_main_patterns_and_the_map_resolve() {
        let dir = std::env::temp_dir().join(format!("pagekit-packages-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let vendor = dir.join("vendor");
        write(
            &vendor.join("a/package.json"),
            r#"{"exports":{".":{"types":"./a.d.ts","require":"./a.cjs","import":"./dist/a.js"},
                "./sub":"./dist/sub.js","./parts/*":"./dist/parts/*.js"}}"#,
        );
        for f in ["dist/a.js", "dist/sub.js", "dist/parts/x.js"] {
            write(&vendor.join("a").join(f), "export {};");
        }
        write(
            &vendor.join("@s/b/package.json"),
            r#"{"main":"dist/b.cjs","module":"src/index.js"}"#,
        );
        write(&vendor.join("@s/b/src/index.js"), "export {};");
        write(
            &dir.join("kit/package.json"),
            r#"{"exports":{"./mcp":"./src/mcp"}}"#,
        );
        write(&dir.join("kit/src/mcp/index.ts"), "export {};");
        write(&dir.join("one.ts"), "export {};");
        let imports = Imports {
            map: BTreeMap::from([
                ("kit".into(), dir.join("kit")),
                ("one".into(), dir.join("one.ts")),
            ]),
            vendor: Some(vendor.clone()),
        };
        let r = |s: &str| imports.resolve(s).unwrap();
        assert_eq!(r("a"), Some(vendor.join("a/dist/a.js")));
        assert_eq!(r("a/sub"), Some(vendor.join("a/dist/sub.js")));
        assert_eq!(r("a/parts/x"), Some(vendor.join("a/dist/parts/x.js")));
        assert_eq!(r("@s/b"), Some(vendor.join("@s/b/src/index.js")));
        assert_eq!(r("kit/mcp"), Some(dir.join("kit/src/mcp/index.ts")));
        assert_eq!(r("one"), Some(dir.join("one.ts")));
        assert_eq!(r("nowhere"), None);
        assert!(imports.resolve("a/private").is_err());
        assert!(imports.resolve("one/sub").is_err());
        assert_eq!(imports.kits().len(), 1);
    }
}
