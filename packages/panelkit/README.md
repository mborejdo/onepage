# panelkit

A small Preact component set for tool pages and MCP Apps, and the design tokens it is drawn
with. Plain Preact and hooks, no framework; the look is utility classes on the token names plus
one short sheet.

- `tokens.toml` - the one source of truth for colours, faces, sizes, the chart series, radii
  and spacing, in two schemes.
- `src/` - the components (`panelkit`), the glyphs (`panelkit/glyphs`) and the MCP Apps client
  (`panelkit/mcp`).
- `base.css` - what a utility cannot say: the faces, the page's ground and ink, focus, the
  figures' motion.
- `fonts/` - the token faces, embedded: Inter and Fira Code, SIL OFL 1.1 (see
  [LICENSE-inter](LICENSE-inter) and [LICENSE-fira-code](LICENSE-fira-code)).

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

`tokens` writes the variables and teaches the utilities the colour names; `[imports]` makes
`panelkit` a bare import, scans its modules for class names and brings `base.css` (the
`style` field of `package.json`). [`examples/kit`](../../examples/kit) shows every component.

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

Text colours reach 4.5:1 on `ground` and `surface` in both schemes, `ground` reaches 4.5:1 on
`accent` (a filled button's label), and the series reach 3:1 (pagekit's `tokens` tests check
it). Fonts: `Inter` for text, its display cut for headings, `Fira Code` for mono, with the system
faces behind them.
`--radius-sm|md|lg` (4, 8, 12px) and `--space-gap` (12px) shape the components.

In a class: `bg-surface`, `text-muted`, `border-border`, `bg-accent/15`, `text-series-3`; in a
value: `rounded-[var(--radius-md)]`, `p-[var(--space-gap)]`. `font-sans` and `font-mono` are the
token faces.

## Faces

`base.css` declares Inter and Fira Code from `fonts/` (onepage inlines them into the page), so a
page looks the same where they are not installed: about 85 KB of woff2, 113 KB in the page.
Both are latin subsets of the variable fonts at weights 300 to 700. Inter keeps its optical
size axis: text draws at its own size, headings (`h1` to `h3`) at `opsz` 32, which is Inter
Display. Neither file has contextual alternates or ligatures, so `->`, `=>` and `!=` stay as
typed, and `base.css` turns ligatures off for `code`, `kbd`, `samp`, `pre` and `font-mono`
whatever face `--font-mono` names. Figures (`Gauge`, `Counter`, `Facts`, tab counts, a numeric
`Input`) use tabular digits.

To rebuild a file (fonttools with brotli), from `InterVariable.ttf` (Inter 4.1) and
`FiraCode-VF.ttf` (Fira Code 6.2):

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

Every value in, every change out as the value (not the event). `tooltip` is the native title
and, for a control whose glyph is its only label, its accessible name.

| | |
|---|---|
| `Button` | `tone` (`accent` fills it), `shape` (`square`, `icon`, `text`), `icon` (a glyph before the label, or the whole label), `disabled`, `shut` (disabled but focusable), `href`, `external`, `type` |
| `Field` | a `<label>` over its control: `label`, `hint` |
| `Input`, `Textarea` | `value`, `onInput(value)`, `placeholder`, `numeric` / `rows`, `mono` |
| `Select` | `value`, `options` (`{ value, label }` or plain strings with `labels`), `groups`, `onChange(value)` |
| `Check`, `Toggle` | `checked`, `onChange(checked)`, `label` |
| `Tag` | `tone`, `dot` (the tone's glyph before an ink word), `label` (a fact: `<Tag label="CPU">42 %</Tag>`), `square` |
| `Code` | inline code on a faint ground: `muted`, `sans` |
| `Card` | `title`, `sub`, `icon`, `actions`, `pad`, `fill`, `onClick` (the card acts), `href` |
| `Alert` | `tone` (`warn` by default), `title`, `actions`, `rail` |
| `Facts` | `rows`: `[label, value, aside?]` or `{ label, value, aside, tone, icon }` |
| `Tabs` | `tabs` (`{ value, label, count, disabled }`), `value`, `onChange`; arrow keys move |
| `Gauge` | `value`, `low`, `high`, `unit`, `label`, `tone`, `decimals`, `ends`; out of range is `bad` |
| `Counter` | a split-flap count: `value`, `caption`, `unit`, `digits`, `size` (`md`, `lg`), `locale` |
| `Bar` | the page's top bar: `name`, `subtitle`, `stage`, `home`, actions as children |
| `SplitPane` | `start`, `end`, `ratio`, `min`; the divider drags and takes arrow keys |
| `Preview` | a `Blob` framed: a PDF in the viewer, HTML sandboxed without scripts; `empty` |
| `Icon` | `glyph`, `size` (16 by default), `spin`, `tooltip` |

`Tone` is `neutral`, `accent`, `ok`, `warn`, `bad` (`stop` is the same as `bad`). `TONE_TEXT`,
`TONE_BG` and `TONE_GLYPH` map a tone to its classes and glyph; `MUTE`, `MUTE_SM`, `LABEL` and
`KICKER` are the caption faces.

## Glyphs

`panelkit/glyphs` exports 119 [Tabler Icons](https://tabler.io/icons) (MIT, see
[LICENSE-tabler](LICENSE-tabler)) in the outline style: a 24 x 24 grid, a 2px round stroke in
`currentColor`. Each is Tabler's name in camel case with `Icon` behind it - `downloadIcon`,
`alertTriangleIcon`, `playerPlayIcon`, `chartLineIcon` - so a page carries only those it
imports. `GLYPHS` maps every Tabler name to its glyph, for a catalogue (it carries them all).
To add one, copy its paths from Tabler's outline SVG into `src/glyphs.ts`.

## MCP Apps: `panelkit/mcp`

A client for the [MCP Apps](https://github.com/modelcontextprotocol/ext-apps) extension: a page
an MCP server lists as a `ui://` resource, shown by a chat host in a sandboxed frame. It speaks
JSON-RPC 2.0 over `postMessage` with the parent window.

- On import, in a frame: `ui/initialize` (the protocol version, the page's name, the display
  modes it supports), then `ui/notifications/initialized`. The host's theme becomes
  `<html data-theme>`, which the token sheet follows.
- From the host: `ui/notifications/tool-input` and `tool-input-partial` (the opening tool's
  arguments), `ui/notifications/tool-result`, `ui/notifications/host-context-changed`; a
  host request (`ui/resource-teardown`, `ping`) is answered.
- To the host: `tools/call`, `ui/request-display-mode`, `ui/open-link`, `ui/message`,
  `ui/update-model-context`, `notifications/message`, and `ui/notifications/size-changed`
  whenever the content's size changes.

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

A page that is not framed (opened directly, a preview, a test) never connects. Its tool calls -
`useTool` and `callTool` - go to `window.__panelkitMocks[name](args)` instead; with no mock
they fail with "no host". A mock returns a `CallToolResult`, or any other value, which is taken
as its `structuredContent`; it may return a promise and may throw. Set it before the first
call - in the page, or in a script a server injects ahead of the page's when it serves the page
outside a host:

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
