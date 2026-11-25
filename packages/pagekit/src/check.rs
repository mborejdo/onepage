//! The checks for mistakes that would otherwise build clean and fail silently in the
//! browser. Each runs on one module's semantic model (Oxc) before it is bundled.

use std::{collections::HashSet, sync::LazyLock};

use oxc_ast::AstKind;
use oxc_ast::ast::{ImportDeclarationSpecifier, ImportOrExportKind};
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};

/// Browser and ECMAScript globals a page may read without declaring them.
static GLOBALS: LazyLock<HashSet<&str>> =
    LazyLock::new(|| include_str!("globals.txt").lines().collect());

/// Real globals that only a `[wasm]` page may read.
const WASM_ONLY: &[&str] = &["WebAssembly", "Worker", "SharedWorker"];

/// Names wasm-bindgen glue probes with `typeof`, allowed with `[wasm]` only.
const WASM_EXTRA: &[&str] = &["WorkerGlobalScope", "global"];

/// Whether `name` may be read undeclared.
fn is_global(name: &str, wasm: bool, extra: &[&str]) -> bool {
    extra.contains(&name)
        || (wasm && WASM_EXTRA.contains(&name))
        || (GLOBALS.contains(name) && (wasm || !WASM_ONLY.contains(&name)))
}

/// A JSX file that does not import its factory: classic JSX compiles `<div/>` to
/// `h("div")`, so the page throws on first render and paints nothing.
#[must_use]
pub fn jsx_factory(semantic: &Semantic<'_>) -> Option<(Span, String)> {
    let mut element: Option<Span> = None;
    let mut fragment: Option<Span> = None;
    let mut imported = HashSet::new();
    let earliest = |slot: &mut Option<Span>, span: Span| {
        if slot.is_none_or(|s| span.start < s.start) {
            *slot = Some(span);
        }
    };
    for node in semantic.nodes() {
        match node.kind() {
            AstKind::ImportDeclaration(decl) if decl.import_kind == ImportOrExportKind::Value => {
                for s in decl.specifiers.iter().flatten() {
                    if let ImportDeclarationSpecifier::ImportSpecifier(s) = s
                        && s.import_kind == ImportOrExportKind::Value
                    {
                        imported.insert(s.local.name.as_str());
                    }
                }
            },
            AstKind::JSXElement(e) => earliest(&mut element, e.span),
            AstKind::JSXFragment(f) => earliest(&mut fragment, f.span),
            _ => {},
        }
    }
    let first = match (element, fragment) {
        (Some(e), Some(f)) => Some(if f.start < e.start { f } else { e }),
        (e, f) => e.or(f),
    }?;
    if !imported.contains("h") {
        return Some((
            first,
            "uses JSX but does not import `h`; add `import { h } from \"preact\";`".into(),
        ));
    }
    match fragment {
        Some(span) if !imported.contains("Fragment") => Some((
            span,
            "uses `<>` but does not import `Fragment`; add it to the `preact` import".into(),
        )),
        _ => None,
    }
}

/// `import.meta` exists in ES modules only; the page is one classic script.
#[must_use]
pub fn import_meta(semantic: &Semantic<'_>) -> Option<Span> {
    semantic
        .nodes()
        .iter()
        .filter_map(|n| match n.kind() {
            AstKind::ImportMeta(m) => Some(m.span),
            _ => None,
        })
        .min_by_key(|s| s.start)
}

/// The first name read but never declared, imported or a known global. Types do not count:
/// they are stripped, and so is a `typeof x` in a type.
#[must_use]
pub fn undeclared(semantic: &Semantic<'_>, wasm: bool, extra: &[&str]) -> Option<(Span, String)> {
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();
    let mut found: Option<(Span, &str)> = None;
    for (name, references) in scoping.root_unresolved_references() {
        let name: &str = name;
        if is_global(name, wasm, extra) {
            continue;
        }
        for &id in references {
            let reference = scoping.get_reference(id);
            let flags = reference.flags();
            if !flags.is_value() || flags.is_value_as_type() {
                continue;
            }
            let node = reference.node_id();
            let mut ancestors = nodes.ancestor_kinds(node);
            if ancestors
                .clone()
                .any(|k| matches!(k, AstKind::TSTypeQuery(_) | AstKind::TSTypeReference(_)))
            {
                continue;
            }
            // Inside a non-arrow function, `arguments` is bound by the language.
            if name == "arguments" && ancestors.any(|k| matches!(k, AstKind::Function(_))) {
                continue;
            }
            let span = nodes.kind(node).span();
            if found.is_none_or(|(s, _)| span.start < s.start) {
                found = Some((span, name));
            }
        }
    }
    found.map(|(span, name)| {
        let hint = if WASM_ONLY.contains(&name) || WASM_EXTRA.contains(&name) {
            "; add a [wasm] table to page.toml to allow it"
        } else {
            "; declare or import it"
        };
        (span, format!("`{name}` is not defined{hint}"))
    })
}
