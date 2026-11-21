// The figures: where a value sits in its range (`Gauge`) and a count that goes on (`Counter`).

import { Component, h } from "preact";
import { MUTE_SM, TONE_BG, TONE_TEXT, type Tone } from "./tone";

/** The decimals `value` carries, at most 6; `decimals` when given. */
function decimalsOf(value: number, decimals?: number): number {
  if (decimals !== undefined) return decimals;
  const text = String(value);
  const dot = text.indexOf(".");
  return dot < 0 ? 0 : Math.min(6, text.length - dot - 1);
}

/** Where a value sits in its range: a bar filled to it, the number beside it, the ends under
 *  it. Out of range is `bad` and clamps the fill; the number is never clamped. */
export function Gauge(props: {
  value: number;
  low: number;
  high: number;
  unit?: string;
  label?: string;
  tone?: Tone;
  decimals?: number;
  /** The range's ends under the bar; `false` for a share. */
  ends?: boolean;
}) {
  const span = props.high - props.low || 1;
  const at = Math.min(1, Math.max(0, (props.value - props.low) / span));
  const out = props.value < props.low || props.value > props.high;
  const tone: Tone = out ? "bad" : (props.tone ?? "accent");
  const number = props.value.toFixed(decimalsOf(props.value, props.decimals));
  const text = props.unit ? `${number} ${props.unit}` : number;
  return (
    <div class="flex flex-col gap-1">
      {props.label ? <span class={MUTE_SM}>{props.label}</span> : null}
      <div class="flex items-center gap-3">
        <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-muted/20" role="img" aria-label={text}>
          <div
            class={`pk-grow-x h-full rounded-full ${TONE_BG[tone]}`}
            style={{ width: `${(at * 100).toFixed(1)}%` }}
          />
        </div>
        <span class={`font-[var(--font-mono)] text-[13px] tabular-nums ${TONE_TEXT[tone]}`}>
          {text}
        </span>
      </div>
      {props.ends === false ? null : (
        <div class="flex justify-between text-[11px] text-muted tabular-nums">
          <span>{props.low}</span>
          <span>{props.high}</span>
        </div>
      )}
    </div>
  );
}

/** One cell of a counter's board: its key (place from the right and character), whether it
 *  is a digit, and whether it flips in (`up`, `down`) or stands (`null`). */
export interface Flap {
  key: string;
  ch: string;
  digit: boolean;
  flip: "up" | "down" | null;
}

/** What a counter remembers between renders. */
export interface Flaps {
  text: string;
  value: number;
  flips: ReadonlyMap<string, "up" | "down">;
}

/** Which cells flip: the digits that differ from the render before, compared place by place
 *  from the right (66 -> 67 flips one, 99 -> 100 three). A rise flips `up`, a fall `down`;
 *  the first render (`before` null) flips nothing. Pure, for a test. */
export function flaps(
  before: Flaps | null,
  value: number,
  text: string,
): { cells: Flap[]; memory: Flaps } {
  const cells: Flap[] = [];
  const flips = new Map<string, "up" | "down">();
  const way = before && value < before.value ? "down" : "up";
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[text.length - 1 - i] ?? "";
    const key = `${i}:${ch}`;
    const digit = ch >= "0" && ch <= "9";
    let flip: "up" | "down" | null = null;
    if (before && digit) {
      const was = before.text[before.text.length - 1 - i];
      flip = was === ch ? (before.flips.get(key) ?? null) : way;
    }
    if (flip) flips.set(key, flip);
    cells.unshift({ key, ch, digit, flip });
  }
  return { cells, memory: { text, value, flips } };
}

/** What a `Counter` takes. */
export interface CounterProps {
  value: number | null;
  /** What it counts, under the number. */
  caption?: string;
  /** After the number, standing. */
  unit?: string;
  /** Decimals; the value's own (up to 3) when absent. */
  digits?: number;
  /** `md` in a grid of facts (the default), `lg` as a block's headline. */
  size?: "md" | "lg";
  /** The number's locale (grouping, decimal mark); the browser's when absent. */
  locale?: string;
}

const BOARD = {
  md: { digits: "text-[20px] leading-7", unit: MUTE_SM },
  lg: { digits: "text-[32px] leading-10", unit: "text-[18px] text-muted" },
} as const;
const FLAP = "pk-flap relative inline-block rounded-[3px] bg-muted/15 px-[3px]";
const FLAP_CELL = { none: FLAP, up: `${FLAP} pk-flip`, down: `${FLAP} pk-flip-back` } as const;

/** A count that goes on, as a split-flap board: the digits that changed flip in, the way the
 *  value went; nothing flips on the first render or under reduced motion. Anchored at the
 *  right, the unit standing after it and the caption under it; a screen reader hears the
 *  value once. A `null` value is a dash. */
export class Counter extends Component<CounterProps> {
  memory: Flaps | null = null;

  render() {
    const p = this.props;
    const digits = p.digits ?? Math.min(3, decimalsOf(p.value ?? 0));
    const text =
      p.value === null || !Number.isFinite(p.value)
        ? "-"
        : p.value.toLocaleString(p.locale, {
            minimumFractionDigits: digits,
            maximumFractionDigits: digits,
          });
    const size = BOARD[p.size ?? "md"];
    const { cells, memory } = flaps(this.memory, p.value ?? Number.NaN, text);
    this.memory = memory;
    return (
      <div class="flex flex-col items-end">
        <span class="flex items-baseline gap-1 whitespace-nowrap">
          <span class="sr-only">{p.unit ? `${text} ${p.unit}` : text}</span>
          <span class={`flex gap-[2px] font-bold tabular-nums ${size.digits}`} aria-hidden="true">
            {cells.map((c) => (
              <span key={c.key} class={c.digit ? FLAP_CELL[c.flip ?? "none"] : "inline-block"}>
                {c.ch}
              </span>
            ))}
          </span>
          {p.unit ? (
            <span class={size.unit} aria-hidden="true">
              {p.unit}
            </span>
          ) : null}
        </span>
        {p.caption ? (
          <span class={`${MUTE_SM} max-w-full min-w-0 truncate`} title={p.caption}>
            {p.caption}
          </span>
        ) : null}
      </div>
    );
  }
}
