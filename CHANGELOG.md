# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow semver.

## Unreleased

### Added

- `onepage lint [DIR]` and `onepage lint --fix`: Biome's lint and formatter in process over the
  page's own sources (the entry's folder, `vendor` and the `[wasm] glue` left out), its
  `recommended` rules with the classic JSX runtime under `h` and `Fragment`. One
  `error:`/`notice:` `file:line: rule: message` line per finding, exit 1 on an error; read-only,
  a file the formatter would change is a `lint/format` error. `--fix` writes the safe fixes and
  the formatter (two spaces, 100 columns, double quotes, trailing commas, semicolons) in place.
  The mapped packages are linted read-only, as notices.
- `[lint]` in `onepage.toml`: `groups = { style = "error" }` and
  `rules = { "style/noTernary" = "off" }`, levels `error`, `notice` and `off`; an unknown group
  or rule is an error that names it.
- `pagekit::lint`: the lint as a library (`page`, `lint_text`, `Rules`, `Report`).
- `tokens = "tokens.toml"`: a design-token file (`[light]` and `[dark]` palettes of nine
  colours, `[fonts]`, `[sizes]`, `[series]`, `[radii]`, `[space]`) written as CSS variables -
  the light palette on `:root`, the dark one under `prefers-color-scheme: dark` and both under
  `[data-theme=light|dark]` - and its colour names taught to the utilities (`bg-surface`,
  `text-muted`, `border-border`, `bg-series-1`, with `/opacity`). The preflight's faces follow
  `--font-sans` and `--font-mono`.
- `[imports]`: an import map from a bare name to a package folder (its `package.json` says the
  entry and the subpaths) or one file. A mapped package's modules are checked and scanned for
  class names, and its `style` sheet comes with it, between the preflight and the utilities.
- `vendor = "vendor"`: bare imports from a folder of unpacked npm packages, by `exports`
  (subpaths, `*` patterns, the `browser`/`import`/`module`/`default` conditions), `module` or
  `main`, including the packages' imports of each other. Vendored code is linked, not checked.
- `preact/jsx-runtime` from the inlined runtime, and namespace imports of `preact`: every
  package shares the page's one Preact.
- `onepage dev` also watches the mapped packages and the token file.
- `[theme.light]`, `[theme.dark]` and `[theme.fonts]` in `onepage.toml`: single tokens changed
  per page, applied before the variables are written, so they hold in both schemes and under
  `[data-theme]`. Only known names; `[theme]` needs `tokens`. The changed palettes are checked
  for WCAG AA (text 4.5:1 on ground and surface, ground 4.5:1 on accent, series 3:1); a
  failing pair fails the build at its line in `onepage.toml`.
- `pagekit::tokens::Theme`, `Tokens::with` and `Tokens::pairs`, the contrast duties as data;
  `Page::theme`.
- `packages/panelkit`: the token file and a Preact component set (controls, cards, alerts,
  facts, tabs, figures, a bar, split panes, a preview, Tabler glyphs) with an MCP Apps client,
  `panelkit/mcp`. `examples/kit` shows every component.
- A package sheet's relative `url(...)` to a `.woff2` or `.woff` file is inlined as a `data:`
  URI: the page stays one file and carries its faces.
- panelkit embeds its token faces: Inter (with its display cut for headings) and Fira Code,
  latin subsets without contextual alternates or ligatures, about 113 KB in the page. Mono
  (`code`, `kbd`, `samp`, `pre`, `font-mono`) has ligatures off whatever the face.

### Changed

- `onepage build` and `onepage dev` fix and format the page's sources before they bundle; a
  lint error fails the build. `onepage::build` returns the lint's report with the path.
- The examples and `onepage new` are lint-clean: `type="button"`, no `!` on the root element.
- Not published to crates.io: Biome's crates are git dependencies (`publish = false`).
- `pagekit::bundle::bundle` returns the script and the modules it loaded;
  `pagekit::css::generate` and `pagekit::test::run` take the page's style and page.
- With a token file, the `font-sans` and `font-mono` utilities are `var(--font-sans)` and
  `var(--font-mono)`, not encre-css's fixed system stacks.

## 0.1.0 - unreleased

The first release.

### Added

- `onepage build`: a Preact TSX page into one self-contained, minified HTML file. swc strips
  TypeScript, compiles JSX to `h`/`Fragment`, links the relative imports and minifies; `preact`
  and `preact/hooks` come from the inlined Preact 10.29.7 runtime.
- encre-css utilities for the class names the source and the shell use, then the page's own
  `style` sheet.
- A strict Content-Security-Policy as the first `<meta>` in `<head>`: the inline script and
  style pinned by their sha256 hashes, `default-src 'none'`, extra sources in `[csp]`.
- `[wasm]`: `WebAssembly` and `Worker` allowed, `'wasm-unsafe-eval'` in the policy, `files`
  copied next to the page, `glue` exempt from the checks.
- The checks, each a `file:line: message` that fails the build: JSX without `h` (or `<>`
  without `Fragment`), an undeclared global, `import.meta`, an import outside the page folder
  or from a package.
- `onepage dev`: rebuild on change, unminified; `--port` serves the output on 127.0.0.1.
- `onepage new`: a page folder that builds on the first try.
- `onepage test` (the `test` feature): `src/**/*.test.ts` in boa, an embedded JavaScript engine.
- `onepage.toml`: `entry`, `shell`, `out`, `style`, `favicon`, `title`, `[csp]`, `[wasm]`.
