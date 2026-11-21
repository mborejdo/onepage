// The pure half of `panelkit/mcp`: the shapes of the MCP Apps protocol and the decisions the
// hooks make, with no `window` and no Preact, so `onepage test` can run them.

/** A `tools/call` result, as the host proxies it from the server. */
export interface CallToolResult {
  structuredContent?: unknown;
  content?: { type: string; text?: string }[];
  isError?: boolean;
  _meta?: Record<string, unknown>;
}

/** How the host shows the page: in the conversation, over it, or floating. */
export type DisplayMode = "inline" | "fullscreen" | "pip";

/** What the host says about itself (`ui/initialize`, then each
 *  `ui/notifications/host-context-changed`, a partial update merged in). Every field optional. */
export interface HostContext {
  theme?: "light" | "dark" | string;
  displayMode?: DisplayMode | string;
  availableDisplayModes?: string[];
  /** The frame, in CSS pixels: a fixed `width`/`height` or a `maxWidth`/`maxHeight`. */
  containerDimensions?: { width?: number; maxWidth?: number; height?: number; maxHeight?: number };
  locale?: string;
  timeZone?: string;
  platform?: string;
  styles?: { variables?: Record<string, string> };
  [key: string]: unknown;
}

/** A stand-in for a server tool where no host answers: `window.__panelkitMocks[name](args)`.
 *  Returns a `CallToolResult`, or any other value (taken as its `structuredContent`); may
 *  return a promise and may throw. */
export type ToolMock = (args: Record<string, unknown>) => unknown;

/** The host as `useHost` hands it over. */
export interface HostState {
  /** A host answered the handshake. */
  connected: boolean;
  /** `light` / `dark` when the host says, else null. */
  theme: "light" | "dark" | null;
  displayMode: DisplayMode | string;
  /** The host takes display-mode requests and does not rule fullscreen out. */
  canFullscreen: boolean;
  container: HostContext["containerDimensions"] | null;
  context: HostContext | null;
}

/** No host at all. */
export const DISCONNECTED: HostState = {
  connected: false,
  theme: null,
  displayMode: "inline",
  canFullscreen: false,
  container: null,
  context: null,
};

/** The sentence a call gets with neither a host nor a mock. */
export const NO_HOST = "no host";

/** The state of a host from what it said in the handshake and since. */
export function hostStateOf(connected: boolean, context: HostContext | null): HostState {
  if (!connected) return DISCONNECTED;
  const theme = context?.theme === "light" || context?.theme === "dark" ? context.theme : null;
  const modes = context?.availableDisplayModes;
  return {
    connected,
    theme,
    displayMode: context?.displayMode ?? "inline",
    canFullscreen: !modes || modes.indexOf("fullscreen") !== -1,
    container: context?.containerDimensions ?? null,
    context,
  };
}

/** Two host states that draw the same. */
export function sameHostState(a: HostState, b: HostState): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** What a tool answered, read once. */
export interface ToolAnswer<T = unknown> {
  data: T | null;
  text: string;
  error: string | null;
}

/** A result's text parts, joined by a blank line. */
export function textOf(result: CallToolResult | null | undefined): string {
  return (result?.content ?? [])
    .filter((c) => c && c.type === "text" && typeof c.text === "string")
    .map((c) => c.text as string)
    .join("\n\n");
}

/** A result as a page reads it: `structuredContent` as data, the text, and the error a
 *  failed tool (`isError`) states in its text. */
export function answerOf<T = unknown>(result: CallToolResult | null | undefined): ToolAnswer<T> {
  const text = textOf(result);
  const data = (result?.structuredContent ?? null) as T | null;
  if (result?.isError) return { data, text, error: text || "the tool reported an error" };
  return { data, text, error: null };
}

/** A mock's return as a result: one that already is stays, anything else is its
 *  `structuredContent`. */
export function asResult(value: unknown): CallToolResult {
  if (
    value &&
    typeof value === "object" &&
    ("structuredContent" in value || "content" in value || "isError" in value)
  ) {
    return value as CallToolResult;
  }
  return { structuredContent: value };
}

/** What a failure says. */
export function reasonOf(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** The state `useTool` hands a component. */
export interface ToolState<T = unknown> {
  data: T | null;
  text: string;
  error: string | null;
  /** A call is out; the data that stands stays meanwhile. */
  loading: boolean;
}

/** Nothing read yet. */
export const IDLE: ToolState<never> = { data: null, text: "", error: null, loading: false };

/** The state after `answer` arrives over `was`: a failed call keeps the data that stood. */
export function settle<T>(was: ToolState<T>, result: CallToolResult): ToolState<T> {
  const a = answerOf<T>(result);
  return a.error
    ? { data: a.data ?? was.data, text: a.text, error: a.error, loading: false }
    : { data: a.data, text: a.text, error: null, loading: false };
}

/** A size worth reporting (`ui/notifications/size-changed`): whole pixels, rounded up, and
 *  never the one reported last; null otherwise. */
export function nextSize(
  last: { width: number; height: number } | null,
  width: number,
  height: number,
): { width: number; height: number } | null {
  const next = { width: Math.ceil(width), height: Math.ceil(height) };
  if (!(next.width >= 0 && next.height >= 0)) return null;
  if (last && last.width === next.width && last.height === next.height) return null;
  return next;
}
