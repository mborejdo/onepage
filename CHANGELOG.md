# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow semver.

## Unreleased

### Added

- `onepage lint [DIR]`: Biome's `recommended` lint and formatter over the page's own sources
  (not `vendor` or `[wasm] glue`), JSX under `h`/`Fragment`. One
  `file:line: error|note: rule: message` line per finding on stdout, then `N errors, 0 warnings`;
  exit 1 on an error, unformatted files are `lint/format` errors. Mapped packages are linted
  read-only, as notes.
- `onepage lint --fix`: safe fixes and formatter (two spaces, 100 columns, double quotes,
  trailing commas, semicolons) written in place.
- `[lint]`: `groups` and `rules` at `error`, `notice` or `off`; unknown names are errors.
- `pagekit::lint` as a library (`page`, `lint_text`, `Rules`, `Report`).
- `tokens = "tokens.toml"`: `[light]`/`[dark]` nine-colour palettes, `[fonts]`, `[sizes]`,
  `[series]`, `[radii]`, `[space]` as CSS variables (light on `:root`, dark under
  `prefers-color-scheme: dark`, both under `[data-theme=light|dark]`); colour names as
  utilities (`bg-surface`, `bg-series-1`, `/opacity`). Preflight faces follow `--font-sans` and
  `--font-mono`.
- `[imports]`: import map from a bare name to a package folder (by its `package.json`) or a
  file. Mapped packages are checked, scanned for class names, and bring their `style` sheet
  (between preflight and utilities).
- `vendor = "vendor"`: bare imports from unpacked npm packages by `exports` (subpaths, `*`,
  `browser`/`import`/`module`/`default`), `module` or `main`, between packages too. Linked, not
  checked.
- `preact/jsx-runtime` and namespace imports of `preact` from the inlined runtime, so all
  packages share one Preact.
- `onepage dev` also watches mapped packages and the token file.
- `[theme.light]`, `[theme.dark]`, `[theme.fonts]`: per-page token overrides that hold in both
  schemes and under `[data-theme]`. Known names only, needs `tokens`. A pair below WCAG AA (text
  4.5:1 on ground and surface, ground 4.5:1 on accent, series 3:1) fails the build at its line.
- `pagekit::tokens::Theme`, `Tokens::with`, `Tokens::pairs` (contrast duties as data),
  `Page::theme`.
- `packages/panelkit`: token file, Preact components (controls, cards, alerts, facts, tabs,
  figures, bar, split panes, preview, Tabler glyphs) and an MCP Apps client, `panelkit/mcp`.
  `examples/kit` shows them all.
- Relative `url(...)` to `.woff2`/`.woff` in a package sheet is inlined as `data:`.
- panelkit embeds Inter (display cut for headings) and Fira Code, latin subsets without
  contextual alternates or ligatures, about 113 KB in the page. Mono (`code`, `kbd`, `samp`,
  `pre`, `font-mono`) has ligatures off whatever the face.
- `cargo xtask assets`: Preact, the Tabler glyphs and the panelkit fonts rebuilt byte for byte
  from pinned, sha256-checked upstream releases in `assets.toml`; `--check` in CI.

### Changed

- `onepage build` and `dev` fix and format before bundling; a lint error fails the build.
  `onepage::build` returns the lint report with the path.
- Examples and `onepage new` are lint-clean (`type="button"`, no `!` on the root element).
- Not on crates.io: Biome's crates are git dependencies (`publish = false`).
- `pagekit::bundle::bundle` returns the script and loaded modules; `pagekit::css::generate` and
  `pagekit::test::run` take the page's style and page.
- With a token file, `font-sans`/`font-mono` are `var(--font-sans)`/`var(--font-mono)`, not
  encre-css's system stacks.
- A check that fails the build prints `file:line: error: message`.
- Failure reasons say what was done: `read {path}`, `write {path}`, `parse {path}`; a bad
  colour is `expected a #rrggbb colour, got "x"`.
- Every subcommand's `--help` has examples and exit codes; `--out FILE`, `--port PORT`.
- panelkit's font licences sit next to the fonts, in `fonts/`.

## 0.1.0 - unreleased

The first release.

### Added

- `onepage build`: a Preact TSX page into one self-contained, minified HTML file. swc strips
  TypeScript, compiles JSX to `h`/`Fragment`, links relative imports and minifies; `preact` and
  `preact/hooks` come from the inlined Preact 10.29.7 runtime.
- encre-css utilities for the class names in the source and shell, then the page's `style`.
- A strict CSP as the first `<meta>` in `<head>`: inline script and style pinned by sha256,
  `default-src 'none'`, extra sources in `[csp]`.
- `[wasm]`: allows `WebAssembly` and `Worker`, adds `'wasm-unsafe-eval'`, copies `files` next to
  the page, exempts `glue` from the checks.
- Checks that fail the build, each a `file:line: message`: JSX without `h` (or `<>` without
  `Fragment`), an undeclared global, `import.meta`, an import outside the page folder or from a
  package.
- `onepage dev`: rebuild on change, unminified; `--port` serves on 127.0.0.1.
- `onepage new`: a page folder that builds on the first try.
- `onepage test` (`test` feature): `src/**/*.test.ts` in boa, an embedded JavaScript engine.
- `onepage.toml`: `entry`, `shell`, `out`, `style`, `favicon`, `title`, `[csp]`, `[wasm]`.
