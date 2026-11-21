//! The checks for mistakes that would otherwise build clean and fail silently in the
//! browser. Each runs on one module's syntax tree while it is bundled.

use std::{collections::HashSet, sync::LazyLock};

use swc_core::common::{Mark, Span};
use swc_core::ecma::ast::{
    Constructor, Function, GetterProp, Ident, ImportDecl, ImportSpecifier, JSXElement, JSXFragment,
    MetaPropExpr, MetaPropKind, Module, NamedExport, SetterProp,
};
use swc_core::ecma::visit::{Visit, VisitWith};

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
pub fn jsx_factory(module: &Module) -> Option<(Span, String)> {
    #[derive(Default)]
    struct Jsx {
        element: Option<Span>,
        fragment: Option<Span>,
        imported: HashSet<String>,
    }
    impl Visit for Jsx {
        fn visit_import_decl(&mut self, n: &ImportDecl) {
            for s in n.specifiers.iter().filter(|_| !n.type_only) {
                match s {
                    ImportSpecifier::Named(s) if !s.is_type_only => {
                        self.imported.insert(s.local.sym.to_string());
                    },
                    _ => {},
                }
            }
        }
        fn visit_jsx_element(&mut self, n: &JSXElement) {
            self.element.get_or_insert(n.span);
            n.visit_children_with(self);
        }
        fn visit_jsx_fragment(&mut self, n: &JSXFragment) {
            self.fragment.get_or_insert(n.span);
            n.visit_children_with(self);
        }
    }
    let mut v = Jsx::default();
    module.visit_with(&mut v);
    let first = v.element.or(v.fragment)?;
    if !v.imported.contains("h") {
        return Some((
            first,
            "uses JSX but does not import `h`; add `import { h } from \"preact\";`".into(),
        ));
    }
    match v.fragment {
        Some(span) if !v.imported.contains("Fragment") => Some((
            span,
            "uses `<>` but does not import `Fragment`; add it to the `preact` import".into(),
        )),
        _ => None,
    }
}

/// `import.meta` exists in ES modules only; the page is one classic script.
#[must_use]
pub fn import_meta(module: &Module) -> Option<Span> {
    struct Meta(Option<Span>);
    impl Visit for Meta {
        fn visit_meta_prop_expr(&mut self, n: &MetaPropExpr) {
            if n.kind == MetaPropKind::ImportMeta {
                self.0.get_or_insert(n.span);
            }
        }
    }
    let mut v = Meta(None);
    module.visit_with(&mut v);
    v.0
}

/// The first name read but never declared, imported or a known global. Runs after the
/// JSX and TypeScript transforms, so types are gone and JSX is `h(...)`.
#[must_use]
pub fn undeclared(
    module: &Module,
    unresolved: Mark,
    wasm: bool,
    extra: &[&str],
) -> Option<(Span, String)> {
    struct Undef<'a> {
        unresolved: Mark,
        wasm: bool,
        extra: &'a [&'a str],
        depth: usize,
        found: Option<(Span, String)>,
    }
    impl Undef<'_> {
        fn nested<N: VisitWith<Self>>(&mut self, n: &N) {
            self.depth += 1;
            n.visit_children_with(self);
            self.depth -= 1;
        }
    }
    impl Visit for Undef<'_> {
        // `export { a as b }`: `b` is a name in the export map, not a reference.
        fn visit_named_export(&mut self, n: &NamedExport) {
            if n.src.is_none() {
                for spec in &n.specifiers {
                    if let swc_core::ecma::ast::ExportSpecifier::Named(s) = spec {
                        s.orig.visit_with(self);
                    }
                }
            }
        }
        // Inside a non-arrow function, `arguments` is bound by the language.
        fn visit_function(&mut self, n: &Function) {
            self.nested(n);
        }
        fn visit_constructor(&mut self, n: &Constructor) {
            self.nested(n);
        }
        fn visit_getter_prop(&mut self, n: &GetterProp) {
            self.nested(n);
        }
        fn visit_setter_prop(&mut self, n: &SetterProp) {
            self.nested(n);
        }
        fn visit_ident(&mut self, id: &Ident) {
            let name = id.sym.as_str();
            if self.found.is_none()
                && id.ctxt.outer() == self.unresolved
                && !(name == "arguments" && self.depth > 0)
                && !is_global(name, self.wasm, self.extra)
            {
                let hint = if WASM_ONLY.contains(&name) || WASM_EXTRA.contains(&name) {
                    "; add a [wasm] table to page.toml to allow it"
                } else {
                    "; declare or import it"
                };
                self.found = Some((id.span, format!("`{name}` is not defined{hint}")));
            }
        }
    }
    let mut v = Undef {
        unresolved,
        wasm,
        extra,
        depth: 0,
        found: None,
    };
    module.visit_with(&mut v);
    v.found
}
