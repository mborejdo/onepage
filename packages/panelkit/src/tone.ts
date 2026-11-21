// The set's one tone vocabulary and the class lists every component shares. Utilities, not a
// stylesheet: the colour names (`text-muted`, `bg-ok`, `border-border`) come from the token
// file, which onepage teaches the utility generator.

import type { Glyph } from "./icon";
import {
  alertTriangleIcon,
  circleCheckIcon,
  circleMinusIcon,
  circleXIcon,
  infoCircleIcon,
} from "./glyphs";

/** What a mark says: `neutral` nothing, `accent` information, `ok` / `warn` / `bad` the three
 *  states. `stop` is `bad` by the token's name. */
export type Tone = "neutral" | "accent" | "ok" | "warn" | "bad" | "stop";

/** Text in a tone: a state's word, a count that warns. Each holds 4.5:1 on ground and surface. */
export const TONE_TEXT: Record<Tone, string> = {
  neutral: "text-muted",
  accent: "text-accent",
  ok: "text-ok",
  warn: "text-warn",
  bad: "text-stop",
  stop: "text-stop",
};

/** A fill in a tone: a dot, a bar, a switch that is on. */
export const TONE_BG: Record<Tone, string> = {
  neutral: "bg-muted/60",
  accent: "bg-accent",
  ok: "bg-ok",
  warn: "bg-warn",
  bad: "bg-stop",
  stop: "bg-stop",
};

/** The glyph a tone wears wherever a glyph says it, so colour is never the only cue. */
export const TONE_GLYPH: Record<Tone, Glyph> = {
  neutral: circleMinusIcon,
  accent: infoCircleIcon,
  ok: circleCheckIcon,
  warn: alertTriangleIcon,
  bad: circleXIcon,
  stop: circleXIcon,
};

/** The caption grey. */
export const MUTE = "text-muted";
/** The caption grey at 12px: a second line, a caption under a figure. */
export const MUTE_SM = "text-[12px] text-muted";
/** A field's label over its control. */
export const LABEL = "text-[11px] text-muted";
/** Small tracked capitals over a figure or across a divider. */
export const KICKER = "text-[11px] font-bold uppercase tracking-widest text-muted";
