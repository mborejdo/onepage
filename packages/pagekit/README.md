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
| `bundle` | links the imports into one classic script with [swc](https://swc.rs): TypeScript stripped, JSX to `h`/`Fragment`, `preact*` from the inlined runtime, minified with `Page::minify` |
| `packages` | resolves bare imports via the import map and a vendor folder, by `package.json` |
| `css` | [encre-css](https://crates.io/crates/encre-css) utilities for the class names in the sources, packages and shell, after the token variables and package sheets |
| `tokens` | a design-token file to CSS variables and utility colour names, plus WCAG contrast |
| `html` | fills the `<!--css-->`/`<!--js-->` slots, title and icon, and adds a CSP pinning the inline script and style by sha256 |
| `check` | fails on what would be a blank page; each `Finding` has a file and line |
| `lint` | [Biome](https://biomejs.dev) lint and formatter over the page's sources (fixes on request) and, read-only, its packages; `Rules` reads `[lint]` |
| `test` | (feature `test`) runs `*.test.ts` in [boa](https://boajs.dev) |

Details live in the [onepage README](../../README.md) and the rustdoc. The vendored Preact
runtime is MIT, see `assets/LICENSE-preact`.

License: MIT OR Apache-2.0.
