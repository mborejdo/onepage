# onepage

A Preact TSX page in, one self-contained HTML file out: bundled, styled, minified and locked
by a strict Content-Security-Policy. Script, styles, Preact runtime and icon are all in that one
file, so it opens from disk, a static host or an email attachment. Pure Rust, no Node, no npm.

## Install

```sh
cargo install --path apps/onepage                          # with `onepage types` (embeds tsrs, Rust 1.100+)
cargo install --path apps/onepage --features test          # adds `onepage test` (embeds boa)
cargo install --path apps/onepage --no-default-features    # without type checks (Rust 1.97+, smaller)
```

Not on crates.io: Oxc ships its linter and formatter crates only as a git tag, which
`cargo install --path` fetches.

## Usage

```sh
onepage new hello                 # hello/onepage.toml, index.html, src/app.tsx
onepage build hello               # hello/dist/index.html, minified (--out to change)
onepage lint hello [--fix]        # Oxc lint and formatter; --fix writes safe fixes back
onepage dev hello --port 8000     # rebuild on save, unminified, on http://127.0.0.1:8000/
onepage test hello                # src/**/*.test.ts (`test` feature)
onepage types hello               # tsc errors and type-aware rules (`types` feature)
```

`dev` polls file times every 300 ms; `--port` adds a small `std::net` server. No live reload.

A page is a folder with an HTML shell (`<!--css-->` and `<!--js-->` mark where the sheet and
script go) and a TSX entry:

```tsx
// src/app.tsx
import { h, render } from "preact";
import { useState } from "preact/hooks";

function App() {
  const [count, setCount] = useState(0);
  return (
    <button type="button" class="p-4 text-sm" onClick={() => setCount(count + 1)}>
      Clicked {count} times
    </button>
  );
}

render(<App />, document.getElementById("root") as HTMLElement);
```

A mistake that would build into a blank page fails the build, one line per finding:

```text
src/app.tsx:3: error: uses JSX but does not import `h`; add `import { h } from "preact";`
```

| check | why |
|---|---|
| JSX without `h`, `<>` without `Fragment` | JSX compiles to `h(...)`, the page throws on first render |
| undeclared global | typo or missing import; `WebAssembly` and `Worker` need `[wasm]` |
| `import.meta` | the page is one classic script, it has none |
| import outside the page folder or from an unknown package | only relative imports, `preact` and `[imports]`/`vendor` packages are bundled |

Exit codes: 0 done, 1 the page has a mistake (check or lint), 2 onepage couldn't do what you
asked (reason on stderr).

## Lint

Oxc's linter (Oxlint's rules) and formatter, in process, over the page's own `.ts`/`.tsx` files
(not `vendor`, not `[wasm] glue`). The preset is onepage's own list of Oxlint rules, one per
line in [`packages/pagekit/src/lint-preset.txt`](packages/pagekit/src/lint-preset.txt): 166
rules, most of them `correctness`, from the `eslint`, `typescript`, `unicorn`, `oxc`, `react`,
`jsx-a11y`, `jest` and `next` plugins, `react/rules-of-hooks` among them. Classic JSX under
Preact's `h` and `Fragment`.

```text
src/app.tsx:10: error: react/button-has-type: `button` elements must have a valid `type` attribute.
1 errors, 0 warnings
```

- The report goes to stdout and ends with the counts. `error` exits 1, `notice` only prints (as
  `note:`). `react/exhaustive-deps` and `react/jsx-key` are notes by default; a file the
  formatter would change is a `format` error, one that does not parse a `parse` error.
- `// eslint-disable-next-line <rule> -- why` (or `oxlint-disable-next-line`) silences a line.
- `[imports]` packages are linted read-only, as notes: fix them where they live.
- `--fix` applies the errors' safe fixes (plus dropping unused imports), then formats
  (Prettier-compatible): two spaces, 100 columns, double quotes, trailing commas, semicolons.
  Twice is once. A note never rewrites the file.
- `build` and `dev` fix and format first, so what's bundled is what's on disk. A lint error
  fails `build` before anything is written; `dev` keeps the last good page. Use `onepage lint`
  as the read-only CI gate.

## Types

With the `types` feature, [tsrs](https://github.com/oxc-project/tsrs) (the TypeScript 7 checker,
ported to Rust) runs in process over the page's own sources: tsc's errors (`TS2322`) and the
type-aware rules for promises and methods that fail without a trace - `no-floating-promises`,
`no-misused-promises`, `await-thenable`, `unbound-method`, `switch-exhaustiveness-check`,
`restrict-template-expressions` (as `typescript/<rule>`). The `tsconfig.json` is made in memory
(strict, classic JSX under `h`/`Fragment`, the DOM libraries); bare imports get their types from
`[imports]` (the sources) and `vendor` (each package's `types`), Preact's typings are inlined,
and `test`/`assert` are declared for `*.test.ts`.

`onepage types` prints the findings; `lint` and `build` add them to the lint's. `[types]` in
`onepage.toml` is a ratchet, so an existing page is not blocked on day one:

```toml
[types]                      # per code, how many findings the page may have
TS7006 = 9
"typescript/no-floating-promises" = 1
```

A code over its count is an error (it fails `lint` and `build`); one under it gets a note to
lower the count. Without the table every finding is a note.

## Config

`onepage.toml` in the page folder. Every key is optional, an unknown key is an error.

```toml
entry = "src/app.tsx"        # module the bundle starts from
shell = "index.html"         # HTML with the <!--css--> and <!--js--> slots
out = "dist/index.html"      # where the page is written
style = "src/style.css"      # sheet appended after the utilities (default: none)
favicon = "favicon.svg"      # SVG inlined as the icon, replaces the shell's icon links
title = "Hello"              # replaces the shell's <title>
tokens = "tokens.toml"       # design tokens, see Tokens
vendor = "../vendor"         # unpacked npm packages, see Bare imports

[imports]                    # import map: bare name -> package folder or one file
panelkit = "../packages/panelkit"
"d3-scale" = "../vendor/d3-scale/src/index.js"

[csp]                        # extra sources, after each directive's defaults
connect = ["https://api.example.com"]
img = []
frame = []
font = []
media = []

[wasm]                       # WebAssembly opt-in
files = ["add.wasm"]         # copied next to `out`
glue = "src/wasm"            # wasm-bindgen glue, skipped by the checks

[theme.light]                # single tokens for this page (needs `tokens`), nine colours per scheme
accent = "#5b4fb3"

[theme.dark]
accent = "#b4a9f5"

[theme.fonts]                # sans, mono, display
mono = "JetBrains Mono"

[lint]                       # levels error, notice, off: per category, then per rule
categories = { suspicious = "error", perf = "error" }
rules = { "eslint/no-console" = "notice", "eslint/no-nested-ternary" = "off" }

[lint.rules]                 # the same, one per line; options as Oxlint takes them
"eslint/max-params" = ["error", { max = 4 }]
```

A rule is named as a finding names it, `<plugin>/<name>` (`eslint/no-console`,
`jsx-a11y/alt-text`); with options, the value is Oxlint's array, the level first. A category is
one of Oxlint's (`correctness`, `suspicious`, `pedantic`, `perf`, `style`, `restriction`): its
rules in the preset's plugins, not the type-aware ones. An unknown name, a `nursery` category
(turn a nursery rule on by name), a rule that needs type information, or a name from another
linter is an error at its line in `onepage.toml`.

The policy is the first element of `<head>`:

```text
default-src 'none'; script-src 'sha256-...'; style-src 'sha256-...';
img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self';
frame-src 'self' blob:; media-src 'self' blob:; worker-src 'self' blob:;
manifest-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'
```

`style="..."` attributes in the markup turn `style-src` into `'unsafe-inline'` (no hash covers
them). `[wasm]` adds `'wasm-unsafe-eval'` to `script-src`.

### Tokens

A token file ([panelkit's](packages/panelkit/tokens.toml), say) has `[light]` and `[dark]` with
the same nine colours (`ground`, `ink`, `muted`, `accent`, `surface`, `border`, `ok`, `warn`,
`stop`), `[fonts]` (`sans`, `mono`, `display`), `[sizes]` (`text`, `label`), and optionally
`[series]` (`light`/`dark` chart colour lists), `[radii]` and `[space]` (named CSS lengths).
They open the sheet as variables:

```css
:root{color-scheme:light dark;--font-sans:"Inter",ui-sans-serif,...;--radius-md:8px;--ground:#fbfaf7;...}
@media (prefers-color-scheme: dark){:root:not([data-theme=light]){--ground:#17191f;...}}
[data-theme=light]{color-scheme:light;--ground:#fbfaf7;...}
[data-theme=dark]{color-scheme:dark;--ground:#17191f;...}
```

The system picks the scheme; `data-theme` on `<html>` (or any element, for a subtree) overrides
it. Every colour and `series-1..n` is a utility colour (`bg-surface`, `text-muted`,
`bg-accent/15`, `fill-series-2`), so a page writes no colour classes.

`[theme.*]` is applied before the variables are written, so it holds in both schemes and under
`data-theme` - a page's own `:root{--accent:...}` would lose to the dark block. Unknown names are
an error, so is `[theme]` without `tokens`. The result must keep WCAG AA or the build fails at
the line (`[theme.dark] accent #7a6fd0 is 4.1:1 on ground, needs 4.5:1`): text colours (`ink`,
`muted`, `accent`, `ok`, `warn`, `stop`) 4.5:1 on `ground` and `surface`, `ground` 4.5:1 on
`accent`, series 3:1 on both.

### Bare imports

`"panelkit"`, `"panelkit/mcp"` or `"@scope/name/sub"` resolve through `[imports]`, then
`vendor`. `preact`, `preact/hooks` and `preact/jsx-runtime` always come from the inlined
runtime, so everything shares one Preact.

- `[imports]` works like an HTML import map or a `file:` dependency, paths relative to the
  page. A file is the entry; a folder is a package, its `package.json` gives entry and
  subpaths. Mapped packages count as page code: checked, scanned for class names, and their
  `package.json` `style` sheet comes along.
- `vendor` holds packages as `npm pack` unpacks them (`<vendor>/<name>/`,
  `<vendor>/@scope/<name>/`); no network, no install. Entry: `exports` (subpaths, `*` patterns,
  conditions `browser`, `import`, `module`, `default` in that order), else `module`, else
  `main`, for the packages' imports of each other too. Vendored code is linked as is, not
  checked. To pin an entry, map it in `[imports]`.

## Examples

- [`examples/hello`](examples/hello): a counter, a second module, a page sheet, an icon, a test.
- [`examples/wasm`](examples/wasm): fetches a 41-byte wasm module and calls `add`. `fetch`
  fails on `file://`, so use `onepage dev examples/wasm --port 8000`.
- [`examples/kit`](examples/kit): every [panelkit](packages/panelkit) component; `#light` or
  `#dark` in the URL picks a scheme.

## How it works

- **Lint** - Oxc fixes and formats the sources; an error stops the build.
- **Bundle** - Oxc runs the checks on each module; rolldown strips TypeScript, compiles JSX,
  links everything (tree-shaken) into one IIFE whose parameters are `window.preact`,
  `window.preactHooks` and `window.preactJsx`. `build` minifies and mangles (Oxc's minifier).
- **Style** - token variables, encre-css's preflight, package sheets, utilities for every class
  name in the sources, packages and shell, then `style`.
- **Fill** - one `<style>` into `<!--css-->`, runtime and bundle as one `<script>` into
  `<!--js-->`, title and icon into `<head>`.
- **Lock** - the policy is hashed from the finished file, so it matches byte for byte.

## Workspace

| crate | what |
|---|---|
| [`apps/onepage`](apps/onepage) | the binary: CLI, `onepage.toml`, dev loop and server, scaffold |
| [`packages/pagekit`](packages/pagekit) | the engine: bundle, packages, tokens, CSS, CSP, checks, lint, tests |
| [`packages/panelkit`](packages/panelkit) | not a crate: a token file and Preact components |
| [`xtask`](xtask) | `cargo xtask assets`, dev only |

## Assets

The Preact runtime, the Tabler glyphs and the fonts are built from pinned upstream releases
listed in [`assets.toml`](assets.toml): npm tarballs and GitHub releases, each checked against
its sha256.

```sh
cargo xtask assets           # rebuild all, byte for byte
cargo xtask assets preact    # or one: preact, tabler, inter, fira-code
cargo xtask assets --check   # the files match [out], offline (CI)
```

To update one, bump its `version`, run it, and take the new `sha256` from the error (check it
against the release). A new glyph is a name in `icons`. The fonts need
[uv](https://docs.astral.sh/uv), which runs the pinned fonttools; nothing else does.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check
cargo xtask assets --check
for p in examples/*; do cargo run -- lint "$p"; done
```

## License

[Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option. The vendored Preact runtime
([license](packages/pagekit/assets/LICENSE-preact)) and panelkit's Tabler glyphs
([license](packages/panelkit/LICENSE-tabler)) are MIT.
