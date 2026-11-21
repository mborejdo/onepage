// The kit: every panelkit component on one page, a catalogue to look at in both schemes. The
// MCP part runs on its own here: no host frames the page, so its tool calls go to the mock.

import { type ComponentChildren, h, render } from "preact";
import { useState } from "preact/hooks";
import {
  Alert,
  Bar,
  Button,
  Card,
  Check,
  Code,
  Counter,
  Facts,
  Field,
  Gauge,
  Icon,
  Input,
  KICKER,
  MUTE_SM,
  Preview,
  Select,
  SplitPane,
  Tabs,
  Tag,
  Textarea,
  Toggle,
  type Tone,
} from "panelkit";
import {
  boltIcon,
  downloadIcon,
  GLYPHS,
  maximizeIcon,
  moonIcon,
  refreshIcon,
  scaleIcon,
  settingsIcon,
  sunIcon,
  trashIcon,
} from "panelkit/glyphs";
import { callTool, requestFullscreen, useHost, useTool } from "panelkit/mcp";

// The page's own stand-in for a server: what a tool call answers without a host.
window.__panelkitMocks = {
  clock: () => ({ structuredContent: { time: new Date().toISOString().slice(11, 19) } }),
  echo: (args) => args,
};

const TONES: Tone[] = ["neutral", "accent", "ok", "warn", "bad"];

/** The scheme: the system's, or the one the URL's hash names. */
function scheme(next: string): void {
  if (next === "light" || next === "dark") {
    document.documentElement.setAttribute("data-theme", next);
  } else {
    document.documentElement.removeAttribute("data-theme");
  }
}
const initial = location.hash.slice(1);
scheme(initial);

function Section(props: { title: string; children?: ComponentChildren }) {
  return (
    <section class="flex flex-col gap-3">
      <h2 class={`m-0 ${KICKER}`}>{props.title}</h2>
      {props.children}
    </section>
  );
}

function Controls() {
  const [name, setName] = useState("Demo");
  const [size, setSize] = useState("md");
  const [note, setNote] = useState("Free text, four rows high.");
  const [on, setOn] = useState(true);
  const [checked, setChecked] = useState(false);
  return (
    <Card title="Controls" sub="Field, Input, Select, Textarea, Check, Toggle">
      <div class="grid gap-3 sm:grid-cols-2">
        <Field label="Name">
          <Input value={name} onInput={setName} placeholder="A name" />
        </Field>
        <Field label="Size" hint="Plain values, shown through labels">
          <Select
            value={size}
            options={["sm", "md", "lg"]}
            labels={{ sm: "Small", md: "Medium", lg: "Large" }}
            onChange={setSize}
          />
        </Field>
        <div class="sm:col-span-2">
          <Field label="Note">
            <Textarea value={note} onInput={setNote} rows={3} />
          </Field>
        </div>
        <Check checked={checked} label="Include drafts" onChange={setChecked} />
        <Toggle checked={on} label="Live" onChange={setOn} />
      </div>
    </Card>
  );
}

function Buttons() {
  return (
    <Card title="Buttons" sub="tones, shapes, an icon">
      <div class="flex flex-col gap-3">
        <div class="flex flex-wrap items-center gap-2">
          {TONES.map((t) => (
            <Button key={t} tone={t}>
              {t}
            </Button>
          ))}
          <Button disabled>disabled</Button>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <Button tone="accent" icon={downloadIcon}>
            Download
          </Button>
          <Button icon={refreshIcon} tooltip="Reload" />
          <Button shape="square" tooltip="Settings">
            <Icon glyph={settingsIcon} />
          </Button>
          <Button shape="icon" tone="bad" icon={trashIcon} tooltip="Delete" />
          <Button shape="text">a word</Button>
        </div>
      </div>
    </Card>
  );
}

function Marks() {
  return (
    <Card title="Tags and code">
      <div class="flex flex-col gap-3">
        <div class="flex flex-wrap gap-2">
          {TONES.map((t) => (
            <Tag key={t} tone={t}>
              {t}
            </Tag>
          ))}
        </div>
        <div class="flex flex-wrap gap-2">
          {TONES.map((t) => (
            <Tag key={t} dot tone={t}>
              {t}
            </Tag>
          ))}
          <Tag label="CPU">42 %</Tag>
          <Tag square>a lane</Tag>
        </div>
        <p class="m-0 text-[13px]">
          Type <Code>onepage build</Code>, or the value that was: <Code muted>0.0.9</Code>, in
          words: <Code sans>in progress</Code>.
        </p>
      </div>
    </Card>
  );
}

function Alerts() {
  return (
    <div class="flex flex-col gap-2">
      <Alert tone="accent" title="A note">
        Information, nothing to fix.
      </Alert>
      <Alert title="A warning" actions={<Button>Retry</Button>}>
        The default tone; it stays until its condition does.
      </Alert>
      <Alert tone="ok">Saved, in a rail-less box.</Alert>
      <Alert tone="bad" title="A failure" rail>
        With the rail down its left edge.
      </Alert>
    </div>
  );
}

function Figures() {
  const [count, setCount] = useState(1234);
  return (
    <Card
      title="Figures"
      sub="Gauge and Counter"
      icon={<Icon glyph={scaleIcon} size={18} />}
      actions={
        <Button icon={boltIcon} onClick={() => setCount(count + 7)}>
          Add 7
        </Button>
      }
    >
      <div class="grid items-end gap-4 sm:grid-cols-3">
        <Gauge label="Load" value={0.62} low={0} high={1} decimals={2} />
        <Gauge label="Out of range" value={104.2} low={0} high={100} unit="kg" />
        <Counter value={count} caption="events" unit="n" locale="en" />
      </div>
    </Card>
  );
}

function Series() {
  const swatches = [
    "bg-series-1",
    "bg-series-2",
    "bg-series-3",
    "bg-series-4",
    "bg-series-5",
    "bg-series-6",
  ];
  return (
    <Card title="Series" sub="the chart colours, in their fixed order">
      <div class="flex h-24 items-end gap-[2px]">
        {swatches.map((c, i) => (
          <div key={c} class="flex flex-1 flex-col items-center gap-1">
            <div class={`w-full rounded-t-[4px] ${c}`} style={{ height: `${30 + i * 10}px` }} />
            <span class={MUTE_SM}>{i + 1}</span>
          </div>
        ))}
      </div>
    </Card>
  );
}

function Glyphs() {
  return (
    <Card title="Glyphs" sub={`${Object.keys(GLYPHS).length} Tabler outline icons`}>
      <div class="grid grid-cols-[repeat(auto-fill,minmax(6.5rem,1fr))] gap-1">
        {Object.entries(GLYPHS).map(([name, glyph]) => (
          <div key={name} class="flex min-w-0 items-center gap-2 py-1 text-[11px] text-muted">
            <span class="text-ink">
              <Icon glyph={glyph} size={18} />
            </span>
            <span class="truncate">{name}</span>
          </div>
        ))}
      </div>
    </Card>
  );
}

function Host() {
  const host = useHost();
  const clock = useTool<{ time: string }>("clock", {}, { every: 1000 });
  const [echo, setEcho] = useState("");
  return (
    <Card
      title="MCP"
      sub="panelkit/mcp, answered by window.__panelkitMocks"
      actions={
        <Button
          shape="square"
          tooltip="Fullscreen"
          disabled={!host.canFullscreen}
          icon={maximizeIcon}
          onClick={() => requestFullscreen()}
        />
      }
    >
      <Facts
        rows={[
          ["connected", String(host.connected)],
          ["display mode", host.displayMode],
          {
            label: "clock",
            value: clock.data?.time ?? "-",
            aside: clock.loading ? "calling" : "every second",
            tone: clock.error ? "bad" : "ok",
          },
          [
            "echo",
            <Button
              key="echo"
              shape="text"
              onClick={() =>
                callTool("echo", { said: "hello" }).then((r) =>
                  setEcho(JSON.stringify(r.structuredContent)),
                )
              }
            >
              call it
            </Button>,
            echo || null,
          ],
        ]}
      />
    </Card>
  );
}

function Frames() {
  const [doc, setDoc] = useState<Blob | null>(null);
  const make = () =>
    setDoc(
      new Blob(
        [
          "<!doctype html><body style='font:14px system-ui;padding:1rem'><h1>A document</h1>" +
            `<p>Made at ${new Date().toLocaleTimeString()}, framed in a sandbox.</p></body>`,
        ],
        { type: "text/html" },
      ),
    );
  return (
    <Card title="Frames" sub="SplitPane and Preview" pad={false}>
      <div class="flex h-56 border-t border-border">
        <SplitPane
          start={
            <div class="flex flex-col items-start gap-2 p-[var(--space-gap)]">
              <span class={MUTE_SM}>Drag the divider, or focus it and use the arrows.</span>
              <Button tone="accent" onClick={make}>
                Make a document
              </Button>
            </div>
          }
          end={<Preview blob={doc} empty="No document yet" />}
        />
      </div>
    </Card>
  );
}

function App() {
  const [tab, setTab] = useState("all");
  const [theme, setTheme] = useState(
    initial === "light" || initial === "dark" ? initial : "system",
  );
  return (
    <div class="flex min-h-screen flex-col">
      <Bar name="panelkit" subtitle="every component, both schemes" stage="0.1">
        <Icon glyph={theme === "dark" ? moonIcon : sunIcon} />
        <span class="w-28">
          <Select
            value={theme}
            options={["system", "light", "dark"]}
            onChange={(t) => {
              setTheme(t);
              scheme(t);
            }}
          />
        </span>
      </Bar>
      <main class="mx-auto flex w-full max-w-5xl flex-col gap-6 p-6">
        <Tabs
          tabs={[
            { value: "all", label: "All" },
            { value: "counted", label: "Counted", count: 12 },
            { value: "later", label: "Coming", disabled: true },
          ]}
          value={tab}
          onChange={setTab}
        />
        <Section title="Controls">
          <div class="grid gap-4 md:grid-cols-2">
            <Controls />
            <div class="flex flex-col gap-4">
              <Buttons />
              <Marks />
            </div>
          </div>
        </Section>
        <Section title="Messages">
          <Alerts />
        </Section>
        <Section title="Figures and data">
          <Figures />
          <div class="grid gap-4 md:grid-cols-2">
            <Series />
            <Host />
          </div>
        </Section>
        <Section title="Frames">
          <Frames />
        </Section>
        <Section title="Glyphs">
          <Glyphs />
        </Section>
      </main>
    </div>
  );
}

render(<App />, document.getElementById("root") as HTMLElement);
