# onepage

A Preact TSX page in, one self-contained HTML file out: bundled, styled, minified and locked
down by a strict Content-Security-Policy.

The script, the stylesheet, the Preact runtime and the icon are all inside the one file, so it
opens from disk, from any static host or as an email attachment. Pure Rust: no Node, no npm,
no `node_modules`.

## Install

You need a Rust toolchain.

```sh
cargo install --path apps/onepage
# with `onepage test` (embeds the boa JavaScript engine)
cargo install --path apps/onepage --features test
```

## Usage

```sh
onepage new hello                 # hello/onepage.toml, index.html and src/app.tsx
onepage build hello               # hello/dist/index.html, minified
onepage build hello --out site/index.html
onepage dev hello --port 8000     # rebuild on save, unminified, on http://127.0.0.1:8000/
onepage test hello                # src/**/*.test.ts (the `test` feature)
```

A page is a folder with a shell and a TSX entry:

```html
<!-- index.html -->
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>hello</title>
<!--css-->
</head>
<body>
<div id="root"></div>
<!--js-->
</body>
</html>
```

```tsx
// src/app.tsx
import { h, render } from "preact";
import { useState } from "preact/hooks";

function App() {
  const [count, setCount] = useState(0);
  return (
    <button class="p-4 text-sm" onClick={() => setCount(count + 1)}>
      Clicked {count} times
    </button>
  );
}

render(<App />, document.getElementById("root")!);
```

A mistake that would build and then show a blank page fails the build instead, with one line on
stderr per finding and exit code 1:

```text
error: src/app.tsx:3: uses JSX but does not import `h`; add `import { h } from "preact";`
```

| check | why |
|---|---|
| JSX without `h`, `<>` without `Fragment` | classic JSX compiles to `h(...)`; without the import the page throws on first render |
| an undeclared global | a typo or a missing import, otherwise a runtime `ReferenceError`; `WebAssembly` and `Worker` need `[wasm]` |
| `import.meta` | the page is one classic script, which has none |
| an import outside the page folder or from a package | only relative imports, `preact` and `preact/hooks` are bundled |

Exit codes: 0 done, 1 the page has a mistake, 2 onepage could not do what you asked (the
reason is the one line on stderr).

## Config

`onepage.toml` in the page's folder. Every key is optional; an unknown key is an error.

```toml
entry = "src/app.tsx"        # the module the bundle starts from
shell = "index.html"         # the HTML with the <!--css--> and <!--js--> slots
out = "dist/index.html"      # where the page is written
style = "src/style.css"      # a sheet appended after the utilities (default: none)
favicon = "favicon.svg"      # an SVG inlined as the icon, replacing the shell's icon links
title = "Hello"              # replaces the shell's <title>

# Extra sources, added after the defaults of each directive.
[csp]
connect = ["https://api.example.com"]
img = []
frame = []
font = []
media = []

# The WebAssembly opt-in.
[wasm]
files = ["add.wasm"]         # copied next to `out`, by file name
glue = "src/wasm"            # generated glue (wasm-bindgen): skipped by the checks
```

The policy, as the first element of `<head>`:

```text
default-src 'none'; script-src 'sha256-...'; style-src 'sha256-...';
img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self';
frame-src 'self' blob:; media-src 'self' blob:; worker-src 'self' blob:;
manifest-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'
```

`style-src` is `'unsafe-inline'` instead when the markup has `style="..."` attributes, which no
hash can cover. With `[wasm]`, `script-src` also has `'wasm-unsafe-eval'`.

## Examples

- [`examples/hello`](examples/hello): a counter, a second module, the page's own sheet, an
  icon and a test file.
- [`examples/wasm`](examples/wasm): fetches a 41-byte WebAssembly module and calls its `add`.
  `fetch` does not work from `file://`, so run it with `onepage dev examples/wasm --port 8000`.

## How it works

1. **Bundle.** swc parses each module, runs the checks, strips TypeScript and compiles JSX in
   the classic runtime (`h`, `Fragment`). The swc bundler links the relative imports into one
   module; each `import { ... } from "preact"` becomes `const { ... } = window.preact`. The
   module is wrapped in a function and, for `build`, minified by the swc minifier with names mangled.
2. **Style.** encre-css generates its preflight and the utilities for every class name in the
   entry's folder and the shell; the `style` sheet follows; `build` strips comments and
   indentation.
3. **Fill.** The sheet goes into `<!--css-->` as one `<style>`; the Preact runtime and the
   bundle into `<!--js-->` as one `<script>`; the title and the icon into `<head>`.
4. **Lock.** The policy is computed from the finished file, so the hashes match byte for byte.

`onepage dev` polls modification times every 300 ms and rebuilds; `--port` adds a small static
server over `std::net`. No live reload: refresh the browser.

## Workspace

| crate | what |
|---|---|
| [`apps/onepage`](apps/onepage) | the binary: command line, `onepage.toml`, dev loop and server, page scaffold |
| [`packages/pagekit`](packages/pagekit) | the engine: swc bundle, encre-css sheet, shell, CSP, checks, boa tests |

## Development

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo deny check
```

CI runs the same, plus the two example pages, uploaded as an artifact.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option. The vendored Preact runtime in `packages/pagekit/assets` is MIT, see its
[license](packages/pagekit/assets/LICENSE-preact).
