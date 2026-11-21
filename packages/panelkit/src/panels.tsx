// The surfaces: a card, an alert, a table of facts and a row of tabs.

import { type ComponentChildren, Fragment, h } from "preact";
import { Icon } from "./icon";
import { MUTE, MUTE_SM, TONE_BG, TONE_GLYPH, TONE_TEXT, type Tone } from "./tone";

/** A card's inner padding, all round: for the parts of a card drawn with `pad={false}`. */
export const CARD_PAD = "p-[var(--space-gap)]";

const CARD = "pk-card rounded-[var(--radius-lg)] border border-border bg-surface text-ink";
/* An acting card's edge takes the accent under the pointer and on focus. */
const CARD_ACT =
  "cursor-pointer no-underline text-[inherit] hover:border-accent focus-visible:border-accent";

/** A bordered surface with an optional head: `title` (one line, never wraps), `sub` beside it
 *  (cut with an ellipsis), `icon` before it and `actions` at its end.
 *
 *  `pad={false}` leaves the content to the edge (a table, a picture). `fill` makes the card
 *  its cell's height, as a column. `onClick` makes the whole card a `role="button"` (Enter
 *  and Space act on it); `href` makes it a link to a new tab. Neither should hold a control. */
export function Card(props: {
  title?: string;
  sub?: string;
  icon?: ComponentChildren;
  actions?: ComponentChildren;
  pad?: boolean;
  fill?: boolean;
  href?: string;
  onClick?: (e: Event) => void;
  tooltip?: string;
  /** More classes on the frame: a margin, a width. */
  class?: string;
  children?: ComponentChildren;
}) {
  const head = props.title || props.sub || props.actions || props.icon;
  const pad = props.pad === false ? "" : CARD_PAD;
  const acts = props.onClick || props.href ? CARD_ACT : "";
  const layout = props.fill ? "flex h-full flex-col overflow-hidden" : acts ? "block" : "";
  const cls = `${CARD} ${pad} ${layout} ${acts} ${props.class ?? ""}`;
  const body = (
    <>
      {head ? (
        <div
          class={`mb-2.5 flex items-center gap-2 ${props.pad === false ? "px-[var(--space-gap)] pt-[var(--space-gap)]" : ""}`}
        >
          <div class="flex min-w-0 flex-1 items-baseline gap-2">
            {props.icon ? <span class="flex-none self-center">{props.icon}</span> : null}
            {props.title ? (
              <strong class="flex-none font-semibold whitespace-nowrap">{props.title}</strong>
            ) : null}
            {props.sub ? (
              <span class={`min-w-0 truncate ${MUTE_SM}`} title={props.sub}>
                {props.sub}
              </span>
            ) : null}
          </div>
          {props.actions}
        </div>
      ) : null}
      {props.children}
    </>
  );
  const act = props.onClick;
  if (act) {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        act(e);
      }
    };
    return (
      <div
        role="button"
        tabIndex={0}
        class={`${cls} w-full text-left`}
        title={props.tooltip}
        onClick={act}
        onKeyDown={key}
      >
        {body}
      </div>
    );
  }
  if (props.href) {
    return (
      <a
        href={props.href}
        target="_blank"
        rel="noopener noreferrer"
        class={cls}
        title={props.tooltip}
      >
        {body}
      </a>
    );
  }
  return (
    <div class={cls} title={props.tooltip}>
      {body}
    </div>
  );
}

/** A message about the whole page that stays until its condition does: a failure, a warning,
 *  a note. `tone` is `warn` by default, the glyph is the tone's; `accent` is a note. `rail`
 *  draws the tone down the left edge too. `bad` is an `alert` to a screen reader, the rest a
 *  `status`. */
export function Alert(props: {
  tone?: Tone;
  title?: string;
  actions?: ComponentChildren;
  rail?: boolean;
  children?: ComponentChildren;
}) {
  const tone = props.tone ?? "warn";
  return (
    <div
      class={`relative flex items-start gap-2.5 border border-border bg-surface px-3 py-[var(--space-gap)] ${
        props.rail ? "rounded-r-[var(--radius-md)]" : "rounded-[var(--radius-md)]"
      }`}
      role={tone === "bad" || tone === "stop" ? "alert" : "status"}
    >
      {props.rail ? (
        <span
          class={`absolute -top-px -bottom-px -left-px w-[3px] ${TONE_BG[tone]}`}
          aria-hidden="true"
        />
      ) : null}
      <span class={`flex h-[1.5em] flex-none items-center ${TONE_TEXT[tone]}`}>
        <Icon glyph={TONE_GLYPH[tone]} />
      </span>
      <div class="flex min-w-0 flex-1 flex-col gap-0.5">
        {props.title ? <strong class="font-semibold">{props.title}</strong> : null}
        <div>{props.children}</div>
      </div>
      {props.actions}
    </div>
  );
}

/** One row of `Facts`: a label, its value and an optional aside (what the value is read
 *  against). The object form adds a `tone` (colours the value) and an `icon` before the label. */
export type FactRow =
  | [label: string, value: ComponentChildren, aside?: ComponentChildren]
  | {
      label: string;
      value: ComponentChildren;
      aside?: ComponentChildren;
      tone?: Tone;
      icon?: ComponentChildren;
    };

/** Labelled rows, a record's facts: a table, so the values line up, with tabular figures and
 *  the label column as wide as its longest label. */
export function Facts(props: { rows: FactRow[] }) {
  const rows = props.rows.map((r) =>
    Array.isArray(r) ? { label: r[0], value: r[1], aside: r[2], tone: undefined, icon: undefined } : r,
  );
  const glyphs = rows.some((r) => r.icon !== undefined && r.icon !== null);
  return (
    <table class="w-full border-collapse text-[13px] tabular-nums">
      <tbody>
        {rows.map((r) => (
          <tr key={r.label} class="border-b border-border last:border-b-0">
            <th class={`w-px py-2 pr-3 text-left align-top font-normal whitespace-nowrap ${MUTE}`}>
              {glyphs ? (
                <span class="flex items-center gap-1.5">
                  <span class="flex h-4 w-4 flex-none items-center justify-center">
                    {r.icon ?? null}
                  </span>
                  {r.label}
                </span>
              ) : (
                r.label
              )}
            </th>
            <td class="py-2 align-top">
              <div class="flex flex-wrap items-baseline gap-x-3">
                <div class={r.tone ? `font-semibold ${TONE_TEXT[r.tone]}` : ""}>{r.value}</div>
                {r.aside === undefined || r.aside === null ? null : (
                  <span class={`whitespace-nowrap ${MUTE_SM}`}>{r.aside}</span>
                )}
              </div>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** One tab: `count` after the label, `disabled` a view that is coming. */
export interface Tab {
  value: string;
  label: string;
  count?: number;
  disabled?: boolean;
}

/* Every tab has a transparent bottom edge and its label sits on an invisible bold twin, so
 * selecting changes a colour and a weight, never a size. */
const TAB =
  "-mb-px flex cursor-pointer items-center gap-1.5 border-0 border-b-2 border-transparent " +
  "bg-transparent px-3 py-[7px] font-[inherit] text-[13px] text-[inherit] hover:bg-muted/10 " +
  "aria-selected:border-accent aria-selected:font-semibold " +
  "aria-disabled:cursor-not-allowed aria-disabled:opacity-45";

/** A row of tabs over one pane: tabs as data, the selection as `value` and `onChange`.
 *  Arrow keys move between the tabs that are not disabled. */
export function Tabs(props: {
  tabs: Tab[];
  value?: string;
  onChange?: (value: string) => void;
}) {
  const value = props.value ?? props.tabs[0]?.value ?? "";
  const move = (delta: number) => {
    const at = props.tabs.findIndex((t) => t.value === value);
    const n = props.tabs.length;
    for (let step = 1; step <= n; step += 1) {
      const next = props.tabs[(((at + delta * step) % n) + n) % n];
      if (next && !next.disabled) {
        props.onChange?.(next.value);
        return;
      }
    }
  };
  return (
    <div class="flex gap-0.5 overflow-x-auto border-b border-border" role="tablist">
      {props.tabs.map((t) => (
        <button
          key={t.value}
          type="button"
          role="tab"
          aria-selected={t.value === value}
          aria-disabled={t.disabled ? "true" : undefined}
          tabIndex={t.value === value ? 0 : -1}
          class={TAB}
          onClick={() => {
            if (!t.disabled) props.onChange?.(t.value);
          }}
          onKeyDown={(e: KeyboardEvent) => {
            if (e.key === "ArrowRight") move(1);
            if (e.key === "ArrowLeft") move(-1);
          }}
        >
          <span class="grid">
            <span class="col-start-1 row-start-1">{t.label}</span>
            <span class="invisible col-start-1 row-start-1 font-semibold" aria-hidden="true">
              {t.label}
            </span>
          </span>
          {t.count === undefined ? null : (
            <span class="rounded-full bg-muted/15 px-1.5 text-[11px] tabular-nums">{t.count}</span>
          )}
        </button>
      ))}
    </div>
  );
}
