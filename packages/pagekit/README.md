# pagekit

A Preact TSX module graph to one self-contained HTML page, in process: no Node, no npm.

```rust
let page = pagekit::Page::new("examples/hello");
let built = pagekit::build(&page)?;
std::fs::write("index.html", built.html)?;
```

- `bundle` links the imports from the entry into one classic script with
  [swc](https://swc.rs): TypeScript stripped, JSX compiled to `h`/`Fragment`, `preact`,
  `preact/hooks` and `preact/jsx-runtime` read from the inlined UMD runtime, minified when
  `Page::minify` is set.
- `packages` resolves bare imports: the page's import map (a name to a package folder or a
  file) and a vendor folder of unpacked npm packages, by their `package.json`.
- `css` scans the source, the mapped packages' modules and the shell for class names and writes
  the utilities with [encre-css](https://crates.io/crates/encre-css), after the token variables
  and the packages' own sheets.
- `tokens` reads a design-token file (two palettes, fonts, sizes, a chart series, radii,
  spacing) into CSS variables and utility colour names, and computes WCAG contrast.
- `html` fills the shell's `<!--css-->` and `<!--js-->` slots, sets the title and the icon, and
  adds a Content-Security-Policy that pins the inline script and style by their sha256 hashes.
- `check` fails the build on what would otherwise be a blank page: JSX without `h`, an
  undeclared global, `import.meta`, an import outside the page. Each is a `Finding` with a file
  and a line.
- `test` (feature `test`) runs `*.test.ts` files in [boa](https://boajs.dev).

The vendored Preact runtime is MIT, see `assets/LICENSE-preact`.

License: MIT OR Apache-2.0.
