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
| an import outside the page folder or from an unknown package | only relative imports, `preact` (and its hooks and JSX runtime) and the packages in `[imports]` or `vendor` are bundled |

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
tokens = "tokens.toml"       # design tokens: CSS variables and utility colour names
vendor = "../vendor"         # a folder of unpacked npm packages, for bare imports

# The import map: a bare name to a package folder or to one file.
[imports]
panelkit = "../packages/panelkit"
"d3-scale" = "../vendor/d3-scale/src/index.js"

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

# Single tokens changed for this page; needs `tokens`. Any of the nine colours per scheme.
[theme.light]
accent = "#5b4fb3"

[theme.dark]
accent = "#b4a9f5"

[theme.fonts]                # sans, mono, display
mono = "JetBrains Mono"
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

### Tokens

`tokens` names a TOML file of design tokens, [`packages/panelkit/tokens.toml`](packages/panelkit/tokens.toml)
for one: `[light]` and `[dark]` palettes of the same nine colours (`ground`, `ink`, `muted`,
`accent`, `surface`, `border`, `ok`, `warn`, `stop`), `[fonts]` (`sans`, `mono`, `display`),
`[sizes]` (`text`, `label`), and optionally `[series]` (`light` and `dark` lists of chart
colours), `[radii]` and `[space]` (CSS lengths by name). The sheet starts with them as
variables:

```css
:root{color-scheme:light dark;--font-sans:"Inter",ui-sans-serif,...;--radius-md:8px;--ground:#fbfaf7;...}
@media (prefers-color-scheme: dark){:root:not([data-theme=light]){--ground:#17191f;...}}
[data-theme=light]{color-scheme:light;--ground:#fbfaf7;...}
[data-theme=dark]{color-scheme:dark;--ground:#17191f;...}
```

So the system picks the scheme, and `<html data-theme="dark">` (or any element's, for a
subtree) overrides it. Each colour name and `series-1` to `series-n` is also a utility colour:
`bg-surface`, `text-muted`, `border-border`, `bg-accent/15`, `fill-series-2`; a page writes no
colour classes of its own.

`[theme.light]`, `[theme.dark]` and `[theme.fonts]` change single tokens before the variables
are written, so a change holds in both schemes and under `data-theme`; a page's own
`:root{--accent:...}` would lose to the dark block. An unknown name is an error, and so is
`[theme]` without `tokens`. The changed palettes are held to the token file's contrast duties,
and a failing pair fails the build:

```text
error: onepage.toml:6: [theme.dark] accent #7a6fd0 is 4.1:1 on ground, needs 4.5:1
```

Text colours (`ink`, `muted`, `accent`, `ok`, `warn`, `stop`) need 4.5:1 on `ground` and
`surface`, `ground` needs 4.5:1 on `accent`, each series colour 3:1 on both.

### Bare imports

A bare import (`import { Card } from "panelkit"`, `"panelkit/mcp"`, `"@scope/name/sub"`)
resolves through `[imports]` first, then `vendor`; `preact`, `preact/hooks` and
`preact/jsx-runtime` always come from the inlined runtime, so the page and every package share
one Preact.

- `[imports]` is an import map, the shape of an HTML import map's `imports` and of a
  `file:` dependency in `package.json`: a specifier or a package name to a folder or a file
  (relative to the page). A file is the entry. A folder is a package: its `package.json` says
  the entry and the subpaths, as below. A mapped package is the page's own code: its modules
  run the checks, their class names count, and a `style` field in its `package.json` names a
  sheet that comes with it.
- `vendor` is a folder of npm packages as `npm pack` unpacks them (`<vendor>/<name>/` and
  `<vendor>/@scope/<name>/`), already on disk: onepage has no network and no install. A
  package's entry is its `exports` (subpaths, `*` patterns, the conditions `browser`, `import`,
  `module` and `default`, in that order), else `module`, else `main`; a package's imports of
  other packages resolve the same way. Vendored code is linked as it is, not checked. To pin an
  entry, map the name to the file in `[imports]`.

## Examples

- [`examples/hello`](examples/hello): a counter, a second module, the page's own sheet, an
  icon and a test file.
- [`examples/wasm`](examples/wasm): fetches a 41-byte WebAssembly module and calls its `add`.
  `fetch` does not work from `file://`, so run it with `onepage dev examples/wasm --port 8000`.
- [`examples/kit`](examples/kit): every [panelkit](packages/panelkit) component, with the
  token file and the kit by its bare name; `#light` or `#dark` in the URL picks a scheme.

## How it works

1. **Bundle.** swc parses each module, runs the checks, strips TypeScript and compiles JSX in
   the classic runtime (`h`, `Fragment`). The swc bundler links the relative and bare imports
   into one module; each `import { ... } from "preact"` becomes `const { ... } = window.preact`.
   The module is wrapped in a function and, for `build`, minified by the swc minifier with
   names mangled.
2. **Style.** The token variables; encre-css's preflight; the mapped packages' sheets; the
   utilities for every class name in the entry's folder, the mapped packages' modules and the
   shell; then the `style` sheet. `build` strips comments and indentation.
3. **Fill.** The sheet goes into `<!--css-->` as one `<style>`; the Preact runtime and the
   bundle into `<!--js-->` as one `<script>`; the title and the icon into `<head>`.
4. **Lock.** The policy is computed from the finished file, so the hashes match byte for byte.

`onepage dev` polls modification times every 300 ms and rebuilds; `--port` adds a small static
server over `std::net`. No live reload: refresh the browser.

## Workspace

| crate | what |
|---|---|
| [`apps/onepage`](apps/onepage) | the binary: command line, `onepage.toml`, dev loop and server, page scaffold |
| [`packages/pagekit`](packages/pagekit) | the engine: swc bundle, package resolution, tokens, encre-css sheet, shell, CSP, checks, boa tests |
| [`packages/panelkit`](packages/panelkit) | not a crate: the token file and a Preact component set, imported by pages |

## Development

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo deny check
```

CI runs the same, plus the example pages, uploaded as an artifact. `tokens`' tests check that
panelkit's token file holds WCAG AA contrast in both schemes.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option. The vendored Preact runtime in `packages/pagekit/assets` is MIT, see its
[license](packages/pagekit/assets/LICENSE-preact). panelkit's glyphs are Tabler Icons, MIT, see
[their license](packages/panelkit/LICENSE-tabler).
