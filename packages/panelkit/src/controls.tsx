// The controls: a button, the text fields, a select, a switch, a tag and inline code. Native
// elements under the set's classes; every value in, every change out as the value (not the
// event). A tooltip is the native `title`.

import { type ComponentChildren, Fragment, h } from "preact";
import { type Glyph, Icon } from "./icon";
import { checkIcon, chevronDownIcon } from "./glyphs";
import { LABEL, MUTE, TONE_BG, TONE_GLYPH, TONE_TEXT, type Tone } from "./tone";

/* Every button: the pointer, the face, the disabled look. `aria-disabled` is a shut button
 * that keeps its focus and its tooltip. */
const BTN =
  "cursor-pointer whitespace-nowrap font-[inherit] disabled:cursor-default disabled:opacity-45 " +
  "aria-disabled:cursor-default aria-disabled:opacity-45";
const FRAME =
  "inline-flex items-center justify-center gap-1.5 rounded-[var(--radius-md)] border no-underline";
const PAD = "px-3 py-[5px] text-[13px]";
/* Level with an input; the glyph is the whole label. */
const SQUARE = "h-8 w-8 flex-none p-0 text-[16px]";
/* A row's own action: the glyph in a 20px box, no frame. */
const ICON_ONLY =
  "inline-flex h-5 w-5 flex-none items-center justify-center rounded-[var(--radius-sm)] " +
  "border-0 bg-transparent p-0 [&:hover:not(:disabled):not([aria-disabled=true])]:bg-muted/15";
/* A button that is a word: no frame, always underlined (a touch has no hover). */
const TEXT =
  "border-0 bg-transparent p-0 text-left text-[13px] underline decoration-muted/50 " +
  "underline-offset-[3px] [&:hover:not(:disabled):not([aria-disabled=true])]:decoration-current";

/* One text colour, edge, ground and hover per tone, so no property gets two rules. Only
 * `accent` is filled; the states are a coloured word on a quiet button. The hover guard is
 * written out (and in full, for the class scan): `hover:` alone would light up a disabled
 * button. */
const QUIET = "border-border [&:hover:not(:disabled):not([aria-disabled=true])]:border-muted";
const FILL: Record<Tone, string> = {
  neutral: `${QUIET} bg-surface text-ink`,
  accent:
    "border-accent bg-accent text-ground " +
    "[&:hover:not(:disabled):not([aria-disabled=true])]:bg-[color:color-mix(in_oklab,var(--accent)_85%,var(--ink))]",
  ok: `${QUIET} bg-transparent text-ok`,
  warn: `${QUIET} bg-transparent text-warn`,
  bad: `${QUIET} bg-transparent text-stop`,
  stop: `${QUIET} bg-transparent text-stop`,
};
/* An unframed button takes its tone as ink. */
const INK: Record<Tone, string> = {
  neutral: "text-ink",
  accent: "text-accent",
  ok: "text-ok",
  warn: "text-warn",
  bad: "text-stop",
  stop: "text-stop",
};

/** A button: framed with padding, `square` (a glyph, 32px, level with an input), `icon` (a
 *  glyph in a row, no frame) or `text` (a word, underlined).
 *
 *  `tone="accent"` fills it: the one action a screen is for. `ok`, `warn` and `bad` colour
 *  the word: close in good order, go on against a warning, discard. `icon` puts a glyph
 *  before the label, or is the whole label when there are no children; then `tooltip` is
 *  its name. `shut` is disabled but still focusable. `href` makes it a link. */
export function Button(props: {
  tone?: Tone;
  shape?: "square" | "text" | "icon";
  /** A glyph before the label, or the whole label without children. */
  icon?: Glyph;
  disabled?: boolean;
  shut?: boolean;
  tooltip?: string;
  /** `tooltip` under its older name. */
  title?: string;
  type?: "button" | "submit";
  href?: string;
  /** Open `href` in a new tab. */
  external?: boolean;
  onClick?: (e: MouseEvent) => void;
  children?: ComponentChildren;
}) {
  const tone = props.tone ?? "neutral";
  const tooltip = props.tooltip ?? props.title;
  const bare = props.children === undefined || props.children === null;
  const shape = props.shape ?? (props.icon && bare ? "square" : undefined);
  const named = shape === "square" || shape === "icon" || bare ? tooltip : undefined;
  const look =
    shape === "text"
      ? `${TEXT} ${INK[tone]}`
      : shape === "icon"
        ? `${ICON_ONLY} ${INK[tone]}`
        : `${FRAME} ${shape === "square" ? SQUARE : PAD} ${FILL[tone]}`;
  const body = (
    <>
      {props.icon ? <Icon glyph={props.icon} /> : null}
      {props.children}
    </>
  );
  if (props.href !== undefined) {
    return (
      <a
        class={`${BTN} ${look}`}
        href={props.href}
        title={tooltip}
        aria-label={named}
        onClick={props.onClick}
        {...(props.external ? { target: "_blank", rel: "noopener noreferrer" } : {})}
      >
        {body}
      </a>
    );
  }
  return (
    <button
      type={props.type ?? "button"}
      class={`${BTN} ${look}`}
      disabled={props.disabled}
      aria-disabled={props.shut ? "true" : undefined}
      title={tooltip}
      aria-label={named}
      onClick={props.shut ? (e: Event) => e.preventDefault() : props.onClick}
    >
      {body}
    </button>
  );
}

/** The box of every text control. */
export const INPUT =
  "pk-input rounded-[var(--radius-md)] border border-border bg-ground px-2.5 py-1.5 " +
  "font-[inherit] text-[13px] text-ink placeholder:text-muted " +
  "focus:outline-2 focus:outline-accent focus:-outline-offset-1 disabled:opacity-60";

/** A label over its control: a real `<label>`, so a click on the words focuses the control. */
export function Field(props: { label: string; hint?: string; children?: ComponentChildren }) {
  return (
    <label class="pk-field flex min-w-0 flex-col gap-1">
      <span class={LABEL}>{props.label}</span>
      {props.children}
      {props.hint ? <span class={`text-[11px] ${MUTE}`}>{props.hint}</span> : null}
    </label>
  );
}

/** One line of text. `numeric` right-aligns it with tabular figures. */
export function Input(props: {
  value?: string;
  placeholder?: string;
  type?: "text" | "search" | "email" | "url" | "number" | "password";
  numeric?: boolean;
  readOnly?: boolean;
  disabled?: boolean;
  tooltip?: string;
  onInput?: (value: string) => void;
}) {
  return (
    <input
      class={`${INPUT} ${props.numeric ? "text-right tabular-nums" : ""}`}
      type={props.type ?? "text"}
      value={props.value ?? ""}
      placeholder={props.placeholder}
      readOnly={props.readOnly}
      disabled={props.disabled}
      title={props.tooltip}
      onInput={(e) => props.onInput?.(e.currentTarget.value)}
    />
  );
}

/** Free text, `rows` high (4 by default). `mono` sets it in the code face. */
export function Textarea(props: {
  value?: string;
  placeholder?: string;
  rows?: number;
  mono?: boolean;
  readOnly?: boolean;
  disabled?: boolean;
  tooltip?: string;
  onInput?: (value: string) => void;
}) {
  return (
    <textarea
      class={`${INPUT} resize-y leading-normal ${props.mono ? "font-mono" : ""}`}
      rows={props.rows ?? 4}
      value={props.value ?? ""}
      placeholder={props.placeholder}
      readOnly={props.readOnly}
      disabled={props.disabled}
      title={props.tooltip}
      spellcheck={!props.mono}
      onInput={(e) => props.onInput?.(e.currentTarget.value)}
    />
  );
}

/** One choice of a `Select`. */
export interface Option {
  value: string;
  label: string;
  tooltip?: string;
}

/** A native `<select>` with the set's chevron. `options` are `Option`s or plain values (shown
 *  as written, unless `labels` names them). With no options it shows `value` as its one. */
export function Select(props: {
  value?: string;
  options: (Option | string)[];
  labels?: Record<string, string>;
  groups?: { label: string; options: Option[] }[];
  tooltip?: string;
  disabled?: boolean;
  onChange: (value: string) => void;
}) {
  const all = props.options.map((o) =>
    typeof o === "string" ? { value: o, label: props.labels?.[o] ?? o } : o,
  );
  const list = all.length || props.groups?.length || !props.value ? all : [
    { value: props.value, label: props.value },
  ];
  const option = (o: Option) => (
    <option key={o.value} value={o.value} title={o.tooltip}>
      {o.label}
    </option>
  );
  return (
    <span class="pk-select relative inline-block w-full">
      <select
        class={`${INPUT} w-full cursor-pointer appearance-none pr-7`}
        value={props.value ?? ""}
        title={props.tooltip}
        disabled={props.disabled}
        onChange={(e) => props.onChange(e.currentTarget.value)}
      >
        {list.map(option)}
        {(props.groups ?? []).map((g) => (
          <optgroup key={g.label} label={g.label}>
            {g.options.map(option)}
          </optgroup>
        ))}
      </select>
      <span
        class="pointer-events-none absolute inset-y-0 right-2 flex items-center text-muted"
        aria-hidden="true"
      >
        <Icon glyph={chevronDownIcon} size={14} />
      </span>
    </span>
  );
}

/** A box beside a sentence: on or off. */
export function Check(props: {
  checked: boolean;
  label: ComponentChildren;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label class="inline-flex cursor-pointer items-center gap-2 text-[13px]">
      <span class="relative inline-flex">
        <input
          type="checkbox"
          class="peer h-4 w-4 cursor-pointer appearance-none rounded-[var(--radius-sm)] border border-muted bg-ground checked:border-accent checked:bg-accent"
          checked={props.checked}
          disabled={props.disabled}
          onChange={(e) => props.onChange(e.currentTarget.checked)}
        />
        <span class="pointer-events-none absolute inset-0 hidden items-center justify-center text-ground peer-checked:flex">
          <Icon glyph={checkIcon} size={12} />
        </span>
      </span>
      {props.label}
    </label>
  );
}

/** A switch for a state a person flips: a real button with `role="switch"`. `label` follows
 *  it; without one, `tooltip` is its name. On is the ok colour. */
export function Toggle(props: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  tooltip?: string;
  title?: string;
  disabled?: boolean;
}) {
  const tooltip = props.tooltip ?? props.title;
  return (
    <label
      class={`inline-flex items-center gap-1.5 text-[13px] ${props.disabled ? "opacity-50" : "cursor-pointer"}`}
      title={tooltip}
    >
      <button
        type="button"
        role="switch"
        aria-checked={props.checked}
        aria-label={props.label ? undefined : tooltip}
        disabled={props.disabled}
        class={`relative inline-block h-4 w-7 flex-none cursor-pointer rounded-full border-0 p-0 transition-colors duration-150 disabled:cursor-default ${
          props.checked ? TONE_BG.ok : "bg-muted/45"
        }`}
        onClick={() => props.onChange(!props.checked)}
      >
        <span
          class={`absolute top-0.5 left-0.5 h-3 w-3 rounded-full bg-ground shadow-sm transition-transform duration-150 ${
            props.checked ? "translate-x-3" : ""
          }`}
        />
      </button>
      {props.label ? <span>{props.label}</span> : null}
    </label>
  );
}

/* The pill's frame, all but its radius and colour. */
const PILL =
  "inline-flex max-w-xs min-w-0 items-center gap-1.5 border px-2 py-0.5 align-middle text-xs whitespace-nowrap";

/** The pill: a status, a count, a category.
 *
 *  `tone` colours the word and the hairline. `dot` keeps the word in the ink and puts the
 *  tone's glyph before it, for a live state. `label` makes it a fact (`<Tag label="CPU">42
 *  %</Tag>`): the label muted before the value, never toned. `square` gives it a chip's
 *  corners. The text truncates; `tooltip` holds the whole. */
export function Tag(props: {
  tone?: Tone;
  dot?: boolean;
  square?: boolean;
  label?: string;
  tooltip?: string;
  title?: string;
  children?: ComponentChildren;
}) {
  const tooltip = props.tooltip ?? props.title;
  if (props.label !== undefined) {
    return (
      <span class={`${PILL} rounded-[var(--radius-sm)] border-border bg-muted/5`} title={tooltip}>
        <span class={MUTE}>{props.label}</span>
        <span class="truncate font-medium">{props.children}</span>
      </span>
    );
  }
  const tone = props.tone ?? "neutral";
  const ink = props.dot || tone === "neutral" ? "border-border" : `border-current ${TONE_TEXT[tone]}`;
  return (
    <span
      class={`${PILL} ${props.square ? "rounded-[var(--radius-sm)]" : "rounded-full"} ${ink}`}
      title={tooltip}
    >
      {props.dot ? (
        tone === "neutral" ? (
          <span class={`h-2 w-2 flex-none rounded-full ${TONE_BG.neutral}`} />
        ) : (
          <span class={`flex-none ${TONE_TEXT[tone]}`}>
            <Icon glyph={TONE_GLYPH[tone]} size={12} />
          </span>
        )
      ) : null}
      <span class="truncate">{props.children}</span>
    </span>
  );
}

/** A value as a value, on a faint ground: a key, a field's content, what a person types.
 *  Mono by default; `sans` keeps the ground and drops the mono; `muted` is the value that was. */
export function Code(props: { children?: ComponentChildren; muted?: boolean; sans?: boolean }) {
  const face = props.sans ? "font-[inherit] text-[12px]" : "font-mono text-[11.5px]";
  return (
    <code
      class={`rounded-[var(--radius-sm)] bg-muted/15 px-1 py-px leading-none whitespace-nowrap ${face} ${
        props.muted ? MUTE : ""
      }`}
    >
      {props.children}
    </code>
  );
}
