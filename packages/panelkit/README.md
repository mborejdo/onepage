# panelkit

A small Preact component set for tool pages and MCP Apps, plus the design tokens it's drawn
with. Plain Preact and hooks; the look is utility classes on the token names and one short sheet.

- `tokens.toml` - colours, faces, sizes, chart series, radii and spacing, in two schemes.
- `src/` - components (`panelkit`), glyphs (`panelkit/glyphs`), MCP Apps client (`panelkit/mcp`).
- `base.css` - what a utility can't say: faces, page ground and ink, focus, figure motion.
- `fonts/` - Inter and Fira Code, SIL OFL 1.1 ([LICENSE-inter](LICENSE-inter),
  [LICENSE-fira-code](LICENSE-fira-code)).

## Use it from a onepage page

```toml
# onepage.toml
tokens = "../../packages/panelkit/tokens.toml"

[imports]
panelkit = "../../packages/panelkit"
```

```tsx
import { h, render } from "preact";
import { Button, Card, Tag } from "panelkit";
import { downloadIcon } from "panelkit/glyphs";

render(
  <Card title="Report" actions={<Tag tone="ok">ready</Tag>}>
    <Button tone="accent" icon={downloadIcon}>Download</Button>
  </Card>,
  document.getElementById("root")!,
);
```

`tokens` writes the variables and the utility colour names; `[imports]` makes `panelkit` a bare
import, scans it for class names and brings `base.css` (its `package.json` `style`).
[`examples/kit`](../../examples/kit) shows every component.

## Tokens

| token | light | dark | for |
|---|---|---|---|
| `ground` | `#fbfaf7` | `#17191f` | the page |
| `ink` | `#1d2129` | `#e8e6e3` | body text |
| `muted` | `#606a7e` | `#8b93a7` | captions, labels, rules |
| `accent` | `#1d6fb8` | `#7cc4fa` | links, focus, the filled button, the selected tab |
| `surface` | `#eef0f4` | `#232733` | cards, alerts |
| `border` | `#d4d8e0` | `#363c4d` | the edges of surfaces |
| `ok` / `warn` / `stop` | `#2d7a38` / `#a15c00` / `#c0392b` | `#8fd694` / `#f2c46d` / `#f28b82` | the three states |
| `series-1..6` | blue, orange, aqua, violet, magenta, ochre | the same hues, stepped | chart series, in order |

Both schemes hold WCAG AA (text 4.5:1 on `ground` and `surface`, `ground` 4.5:1 on `accent`,
series 3:1), checked by pagekit's tests. Faces: Inter, its display cut for headings, Fira Code
for mono, system faces behind. Radii `--radius-sm|md|lg` (4, 8, 12px), gap `--space-gap`
(12px). Use them as `bg-surface`, `text-series-3`, `rounded-[var(--radius-md)]`,
`p-[var(--space-gap)]`, `font-mono`.

## Faces

onepage inlines the faces from `fonts/` (85 KB woff2, 113 KB in the page), so they need no
install. Latin subsets of the variable fonts, weights 300 to 700. Headings (`h1` to `h3`) use
Inter's `opsz` 32, Inter Display. No contextual alternates or ligatures (`->`, `=>`, `!=` stay
as typed); `code`, `kbd`, `samp`, `pre` and `font-mono` have ligatures off whatever the face.
Figures (`Gauge`, `Counter`, `Facts`, tab counts, numeric `Input`) use tabular digits.

Rebuild (fonttools with brotli) from `InterVariable.ttf` (Inter 4.1) and `FiraCode-VF.ttf`
(Fira Code 6.2):

```sh
U="U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,\
U+2000-206F,U+20AC,U+2122,U+2190-2195,U+2212,U+2215,U+2248,U+2260,U+2264-2265,U+FEFF,U+FFFD"
fonttools varLib.instancer InterVariable.ttf wght=300:700 -o inter.ttf
pyftsubset inter.ttf --unicodes="$U" --flavor=woff2 --output-file=fonts/inter.woff2 \
  --layout-features=ccmp,locl,mark,mkmk,kern,case,tnum,frac,numr,dnom
fonttools varLib.instancer FiraCode-VF.ttf wght=300:700 -o fira.ttf
pyftsubset fira.ttf --unicodes="$U" --flavor=woff2 --output-file=fonts/fira-code.woff2 \
  --layout-features=ccmp,locl,mark,mkmk
```

## Components

Values in, changes out as the value (not the event). `tooltip` is the native title and, for a
glyph-only control, its accessible name.

| | |
|---|---|
| `Button` | `tone` (`accent` fills it), `shape` (`square`, `icon`, `text`), `icon` (before the label, or the whole label), `disabled`, `shut` (disabled but focusable), `href`, `external`, `type` |
| `Field` | a `<label>` over its control: `label`, `hint` |
| `Input`, `Textarea` | `value`, `onInput(value)`, `placeholder`, `numeric` / `rows`, `mono` |
| `Select` | `value`, `options` (`{ value, label }` or strings with `labels`), `groups`, `onChange(value)` |
| `Check`, `Toggle` | `checked`, `onChange(checked)`, `label` |
| `Tag` | `tone`, `dot` (tone glyph before an ink word), `label` (a fact: `<Tag label="CPU">42 %</Tag>`), `square` |
| `Code` | inline code on a faint ground: `muted`, `sans` |
| `Card` | `title`, `sub`, `icon`, `actions`, `pad`, `fill`, `onClick` (the card acts), `href` |
| `Alert` | `tone` (default `warn`), `title`, `actions`, `rail` |
| `Facts` | `rows`: `[label, value, aside?]` or `{ label, value, aside, tone, icon }` |
| `Tabs` | `tabs` (`{ value, label, count, disabled }`), `value`, `onChange`; arrow keys move |
| `Gauge` | `value`, `low`, `high`, `unit`, `label`, `tone`, `decimals`, `ends`; out of range is `bad` |
| `Counter` | split-flap count: `value`, `caption`, `unit`, `digits`, `size` (`md`, `lg`), `locale` |
| `Bar` | top bar: `name`, `subtitle`, `stage`, `home`, actions as children |
| `SplitPane` | `start`, `end`, `ratio`, `min`; the divider drags and takes arrow keys |
| `Preview` | a `Blob` framed: PDF in the viewer, HTML sandboxed without scripts; `empty` |
| `Icon` | `glyph`, `size` (default 16), `spin`, `tooltip` |

`Tone` is `neutral`, `accent`, `ok`, `warn`, `bad` (`stop` = `bad`). `TONE_TEXT`, `TONE_BG` and
`TONE_GLYPH` map a tone to classes and glyph; `MUTE`, `MUTE_SM`, `LABEL` and `KICKER` are the
caption faces.

## Glyphs

`panelkit/glyphs` exports 119 outline [Tabler Icons](https://tabler.io/icons) (MIT,
[LICENSE-tabler](LICENSE-tabler)): 24 x 24, 2px round stroke in `currentColor`. Names are
Tabler's in camel case plus `Icon` (`downloadIcon`, `alertTriangleIcon`, `chartLineIcon`), so a
page carries only what it imports. `GLYPHS` maps every Tabler name to its glyph, for a catalogue
(it carries them all). To add one, copy its paths from Tabler's outline SVG into
`src/glyphs.ts`.

## MCP Apps: `panelkit/mcp`

A client for [MCP Apps](https://github.com/modelcontextprotocol/ext-apps): a page an MCP server
lists as a `ui://` resource, shown by a chat host in a sandboxed frame. JSON-RPC 2.0 over
`postMessage` with the parent window.

- On import, in a frame: `ui/initialize` (protocol version, page name, supported display
  modes), then `ui/notifications/initialized`. The host's theme becomes `<html data-theme>`.
- From the host: `ui/notifications/tool-input` and `tool-input-partial` (the opening tool's
  arguments), `ui/notifications/tool-result`, `ui/notifications/host-context-changed`; host
  requests (`ui/resource-teardown`, `ping`) are answered.
- To the host: `tools/call`, `ui/request-display-mode`, `ui/open-link`, `ui/message`,
  `ui/update-model-context`, `notifications/message`, and `ui/notifications/size-changed` on
  every content resize.

| | |
|---|---|
| `useHost()` | `{ connected, theme, displayMode, canFullscreen, container, context }` |
| `useTool(name, args, { every })` | `{ data, text, error, loading, reload }`: `data` is `structuredContent`; repeats when `name` or `args` change; `every` polls |
| `useToolResult()` | `{ data, input }`: the opening tool's result and arguments |
| `callTool(name, args)` | a promise of the `CallToolResult` |
| `requestFullscreen()`, `exitFullscreen()` | `true` when asked; the host decides |
| `openLink`, `sendMessage`, `updateModelContext`, `sendLog` | `false` without a host |
| `onToolResult`, `onToolInput`, `answerOf`, `hostStateOf` | the raw deliveries and the pure readers |

### Without a host: `window.__panelkitMocks`

An unframed page (opened directly, a preview, a test) never connects: `useTool` and `callTool`
call `window.__panelkitMocks[name](args)`, or fail with "no host". A mock returns a
`CallToolResult` or any value (taken as `structuredContent`), may be async, may throw. Set it
before the first call, in the page or in a script a server injects ahead of it:

```js
// Every tool of the server that serves the page, by name, over its JSON-RPC endpoint.
window.__panelkitMocks = new Proxy({}, {
  get: (_, name) => async (args) => {
    const res = await fetch("/mcp", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: args } }),
    });
    return (await res.json()).result;
  },
});
```

## License

MIT OR Apache-2.0, like the repository. The glyphs are Tabler Icons, MIT.
