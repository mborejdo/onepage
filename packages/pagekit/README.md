# pagekit

The engine behind onepage: a Preact TSX module graph to one self-contained HTML page, in
process. No Node, no npm.

```rust
let page = pagekit::Page::new("examples/hello");
let built = pagekit::build(&page)?;
std::fs::write("index.html", built.html)?;
```

| module | does |
|---|---|
| `bundle` | links the imports into one classic script with [rolldown](https://rolldown.rs) and [Oxc](https://oxc.rs): TypeScript stripped, JSX to `h`/`Fragment`, `preact*` from the inlined runtime, tree-shaken, minified with `Page::minify` |
| `packages` | resolves bare imports via the import map and a vendor folder, by `package.json` |
| `css` | [encre-css](https://crates.io/crates/encre-css) utilities for the class names in the sources, packages and shell, after the token variables and package sheets |
| `tokens` | a design-token file to CSS variables and utility colour names, plus WCAG contrast |
| `html` | fills the `<!--css-->`/`<!--js-->` slots, title and icon, and adds a CSP pinning the inline script and style by sha256 |
| `check` | fails on what would be a blank page; each `Finding` has a file and line |
| `lint` | [Oxc](https://oxc.rs)'s linter (Oxlint's rules) and formatter over the page's sources (fixes on request) and, read-only, its packages; `Rules` reads `[lint]`: Oxlint categories and rule names |
| `types` | (feature `types`) [tsrs](https://github.com/oxc-project/tsrs), the TypeScript 7 checker in Rust, over the page's sources: tsc's errors and six type-aware rules, under a per-code ratchet |
| `test` | (feature `test`) runs `*.test.ts` in [boa](https://boajs.dev) |

Details live in the [onepage README](../../README.md) and the rustdoc. The vendored Preact
runtime is MIT, see `assets/LICENSE-preact`.

License: MIT OR Apache-2.0.
