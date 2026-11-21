import { h } from "preact";

/** One path of a glyph, as Tabler's outline SVG has it. */
export interface GlyphPath {
  d: string;
  fill?: string;
  stroke?: string;
  opacity?: string;
}

/** A glyph: its paths on a 24 x 24 grid. `panelkit/glyphs` exports one per icon. */
export type Glyph = GlyphPath[];

/** The icon scale in px: 14 in a dense row, 16 beside body text (the default), 18 beside a
 *  card's title, 24 on its own. Any CSS length works too. */
export type IconSize = 14 | 16 | 18 | 24;

/** An outline glyph, stroked in the current colour.
 *
 *  `import { downloadIcon } from "panelkit/glyphs"`, then `<Icon glyph={downloadIcon} />`: the
 *  page carries only the glyphs it imports. Decorative by default (`aria-hidden`); `tooltip`
 *  names it, for an icon that is the only label. */
export function Icon(props: {
  glyph: Glyph;
  /** A step of the scale (px) or a CSS length; 16 by default. */
  size?: IconSize | number | string;
  /** Turn, for a busy mark. */
  spin?: boolean;
  /** The name a screen reader reads and the native tooltip. */
  tooltip?: string;
  /** `tooltip` under its older name. */
  title?: string;
}) {
  const label = props.tooltip ?? props.title;
  const size = typeof props.size === "number" ? `${props.size}px` : (props.size ?? "16px");
  const svg = (
    <svg
      // A space before the hole: glued to it, the class scan reads `${` as part of the name.
      class={`inline-block flex-none [vertical-align:-0.125em] ${
        props.spin ? "animate-spin motion-reduce:animate-none" : ""
      }`}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      {props.glyph.map((p, i) => (
        <path key={i} d={p.d} fill={p.fill} stroke={p.stroke} opacity={p.opacity} />
      ))}
    </svg>
  );
  return label ? (
    <span role="img" aria-label={label} title={label} class="inline-flex">
      {svg}
    </span>
  ) : (
    svg
  );
}
