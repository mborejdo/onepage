// The page's frame: the bar across its top, two panes side by side, and a preview of a
// document the page made.

import { type ComponentChildren, h } from "preact";
import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { MUTE } from "./tone";

/** The bar across the top of a tool page: its name, a subtitle that gives way first, and the
 *  page's actions at the far end (`children`). `stage` puts a pill beside the name (`beta`). */
export function Bar(props: {
  name: string;
  subtitle?: string;
  stage?: string;
  /** Where the name links to; no link when absent. */
  home?: string;
  children?: ComponentChildren;
}) {
  const name = <span class="whitespace-nowrap">{props.name}</span>;
  return (
    <header class="flex flex-none flex-wrap items-center gap-3 border-b border-border bg-ground px-4 py-2.5">
      <h1 class="m-0 flex min-h-8 shrink-0 items-center gap-2 font-[var(--font-display)] text-[14px] font-semibold">
        {props.home === undefined ? (
          name
        ) : (
          <a href={props.home} class="text-[inherit] no-underline">
            {name}
          </a>
        )}
      </h1>
      {props.stage ? (
        <span class="rounded-full border border-accent px-1.5 text-[11px] text-accent">
          {props.stage}
        </span>
      ) : null}
      <span class={`min-w-[4em] flex-1 truncate text-[12px] ${MUTE}`}>{props.subtitle}</span>
      <div class="ml-auto flex flex-wrap items-center justify-end gap-3">{props.children}</div>
    </header>
  );
}

/** Two panes side by side, the divider dragged with the pointer or moved with the arrow keys;
 *  stacked on a narrow screen. `ratio` is the start pane's share at first (0.5), kept between
 *  `min` and `1 - min` (0.2). */
export function SplitPane(props: {
  start: ComponentChildren;
  end: ComponentChildren;
  ratio?: number;
  min?: number;
  /** The divider's name for a screen reader. */
  label?: string;
}) {
  const min = props.min ?? 0.2;
  const [ratio, setRatio] = useState(props.ratio ?? 0.5);
  const box = useRef<HTMLDivElement>(null);
  const clamp = (r: number) => Math.min(1 - min, Math.max(min, r));
  const drag = (e: PointerEvent) => {
    const el = box.current;
    if (!el) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => {
      const r = el.getBoundingClientRect();
      setRatio(clamp((m.clientX - r.left) / r.width));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
  const key = (e: KeyboardEvent) => {
    const step = e.key === "ArrowLeft" ? -0.05 : e.key === "ArrowRight" ? 0.05 : 0;
    if (step) {
      e.preventDefault();
      setRatio((r) => clamp(r + step));
    }
  };
  const share = `${(ratio * 100).toFixed(1)}%`;
  return (
    <div ref={box} class="flex min-h-0 flex-1 flex-col md:flex-row">
      <div class="flex min-h-0 min-w-0 flex-col md:flex-none" style={{ flexBasis: share }}>
        {props.start}
      </div>
      <div
        role="separator"
        aria-orientation="vertical"
        aria-label={props.label ?? "resize"}
        aria-valuenow={Math.round(ratio * 100)}
        aria-valuemin={Math.round(min * 100)}
        aria-valuemax={Math.round((1 - min) * 100)}
        tabIndex={0}
        class="hidden w-1 flex-none cursor-col-resize bg-border hover:bg-accent focus-visible:bg-accent md:block"
        onPointerDown={drag}
        onKeyDown={key}
      />
      <div class="flex min-h-0 min-w-0 flex-1 flex-col">{props.end}</div>
    </div>
  );
}

/** A document the page made, framed: a PDF in the browser's viewer, an HTML page in a
 *  sandbox with no scripts. The blob's URL is revoked when it is replaced. `empty` stands in
 *  while there is none. `pdf` is `blob` under its older name. */
export function Preview(props: {
  blob?: Blob | null;
  pdf?: Blob | null;
  title?: string;
  empty?: ComponentChildren;
}) {
  const blob = props.blob ?? props.pdf ?? null;
  const url = useMemo(() => (blob ? URL.createObjectURL(blob) : null), [blob]);
  useEffect(() => () => {
    if (url) URL.revokeObjectURL(url);
  }, [url]);
  if (!url || !blob) {
    return (
      <div class={`flex flex-1 items-center justify-center p-6 text-center ${MUTE}`}>
        {props.empty}
      </div>
    );
  }
  const pdf = blob.type === "application/pdf";
  return (
    <iframe
      class="min-h-0 w-full flex-1 border-0 bg-surface"
      title={props.title ?? (pdf ? "PDF" : "Preview")}
      src={pdf ? `${url}#view=FitH&toolbar=0` : url}
      sandbox={pdf ? undefined : ""}
    />
  );
}
