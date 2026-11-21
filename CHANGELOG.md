# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow semver.

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
