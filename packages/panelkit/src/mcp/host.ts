// The connection to an MCP Apps host: JSON-RPC 2.0 over `postMessage` with the parent
// window, as the MCP Apps extension specifies it (SEP-1865).
//
// At load, a page inside a frame sends `ui/initialize` (its name, the protocol version and the
// display modes it supports); the host answers with its capabilities and context, and the page
// confirms with `ui/notifications/initialized`. After that the host pushes the opening tool's
// arguments (`ui/notifications/tool-input`, `tool-input-partial` while they stream) and its
// result (`ui/notifications/tool-result`), and context changes
// (`ui/notifications/host-context-changed`). The page asks through the host: `tools/call`,
// `ui/request-display-mode`, `ui/open-link`, `ui/message`, `ui/update-model-context`, and it
// reports its content's size (`ui/notifications/size-changed`). A page that is not framed
// never connects; its calls go to `window.__panelkitMocks`.

import {
  type CallToolResult,
  type HostContext,
  nextSize,
  reasonOf,
  type ToolMock,
} from "./core";

declare global {
  interface Window {
    /** Answers for tool calls where no host does: a page opened directly, a preview, a test.
     *  Set by the page (or the server that serves it) before the first call. */
    __panelkitMocks?: Record<string, ToolMock>;
  }
}

/** The MCP Apps protocol version this client speaks. */
export const PROTOCOL_VERSION = "2026-01-26";

type Listener = (value: unknown) => void;

const pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
const results = new Set<Listener>();
const inputs = new Set<Listener>();
const changes = new Set<() => void>();
let seq = 0;
let connected = false;
let connecting: Promise<boolean> | null = null;
let context: HostContext | null = null;
let capabilities: Record<string, unknown> | null = null;
let lastResult: { data: unknown } | null = null;
let lastInput: { args: unknown } | null = null;

const hasWindow = () => typeof window !== "undefined";

/** Whether the page sits in a frame, where a host may be. */
export function framed(): boolean {
  return hasWindow() && typeof window.parent !== "undefined" && window.parent !== window;
}

/** The mocks, when the page set any. */
export function mocks(): Record<string, ToolMock> | undefined {
  return hasWindow() ? window.__panelkitMocks : undefined;
}

function post(message: Record<string, unknown>): void {
  window.parent.postMessage({ jsonrpc: "2.0", ...message }, "*");
}

/** Send a request to the host; resolves to its result, rejects with its error. */
export function request(method: string, params: Record<string, unknown> = {}): Promise<unknown> {
  if (!framed()) return Promise.reject(new Error("no host"));
  seq += 1;
  const id = seq;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    post({ id, method, params });
  });
}

/** Send a notification to the host: no answer. */
export function notify(method: string, params: Record<string, unknown> = {}): void {
  if (framed()) post({ method, params });
}

function fan(set: Set<Listener>, value: unknown): void {
  for (const fn of Array.from(set)) {
    try {
      fn(value);
    } catch (e) {
      console.error("panelkit/mcp: a listener threw", e);
    }
  }
}

function changed(): void {
  applyTheme();
  for (const fn of Array.from(changes)) fn();
}

/** The host's theme as `<html data-theme>`, which the token sheet turns into the scheme. */
function applyTheme(): void {
  const theme = context?.theme;
  if ((theme === "light" || theme === "dark") && typeof document !== "undefined") {
    document.documentElement.setAttribute("data-theme", theme);
  }
}

/** A message from the host: an answer to a request, a notification, or a request of its own. */
function receive(m: Record<string, unknown>): void {
  const method = typeof m.method === "string" ? m.method : null;
  const params = (m.params ?? {}) as Record<string, unknown>;
  if (method === null) {
    const waiting = pending.get(m.id as number);
    if (!waiting) return;
    pending.delete(m.id as number);
    const error = m.error as { message?: string } | undefined;
    if (error) waiting.reject(new Error(error.message ?? "the host refused"));
    else waiting.resolve(m.result);
    return;
  }
  switch (method) {
    case "ui/notifications/tool-input":
    case "ui/notifications/tool-input-partial":
      lastInput = { args: params.arguments ?? null };
      fan(inputs, lastInput.args);
      return;
    case "ui/notifications/tool-result": {
      const result = params as CallToolResult;
      lastResult = { data: result.structuredContent ?? {} };
      fan(results, lastResult.data);
      return;
    }
    case "ui/notifications/host-context-changed":
      context = { ...(context ?? {}), ...params };
      changed();
      return;
    default:
      // A request of the host's (`ui/resource-teardown`, `ping`): answered, nothing to keep.
      if (m.id !== undefined) post({ id: m.id, result: {} });
  }
}

/** Open the connection: the handshake, then the size reports. Once; resolves to whether a
 *  host answered. A page that is not framed resolves false at once. */
export function connect(name = "panelkit"): Promise<boolean> {
  if (connecting) return connecting;
  if (!framed() || typeof window.addEventListener !== "function") {
    connecting = Promise.resolve(false);
    return connecting;
  }
  window.addEventListener("message", (e: MessageEvent) => {
    const m = e.data as Record<string, unknown> | null;
    if (e.source === window.parent && m && m.jsonrpc === "2.0") receive(m);
  });
  connecting = request("ui/initialize", {
    protocolVersion: PROTOCOL_VERSION,
    appInfo: { name, version: "0.1.0" },
    appCapabilities: { availableDisplayModes: ["inline", "fullscreen"] },
  }).then(
    (answer) => {
      const a = (answer ?? {}) as Record<string, unknown>;
      capabilities = (a.hostCapabilities ?? {}) as Record<string, unknown>;
      context = (a.hostContext ?? {}) as HostContext;
      connected = true;
      notify("ui/notifications/initialized");
      reportSizes();
      changed();
      return true;
    },
    (e: unknown) => {
      console.warn("panelkit/mcp: no host answered ui/initialize:", reasonOf(e));
      return false;
    },
  );
  return connecting;
}

/** Report the content's size on every change, never the same size twice. The height is the
 *  document's at `height: max-content`, so a page pinned to its frame reports its content. */
function reportSizes(): void {
  if (typeof ResizeObserver === "undefined" || typeof document === "undefined") return;
  let last: { width: number; height: number } | null = null;
  let queued = false;
  const measure = () => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      const root = document.documentElement;
      const was = root.style.height;
      root.style.height = "max-content";
      const height = root.getBoundingClientRect().height;
      root.style.height = was;
      const size = nextSize(last, window.innerWidth, height);
      if (size) {
        last = size;
        notify("ui/notifications/size-changed", size);
      }
    });
  };
  const ro = new ResizeObserver(measure);
  ro.observe(document.documentElement);
  if (document.body) ro.observe(document.body);
  const root = document.getElementById("root");
  if (root) ro.observe(root);
  measure();
}

/** Whether a host answered the handshake. */
export function isConnected(): boolean {
  return connected;
}

/** What the host said about itself, merged. */
export function hostContext(): HostContext | null {
  return context;
}

/** What the host can do, from the handshake. */
export function hostCapabilities(): Record<string, unknown> | null {
  return capabilities;
}

/** Hear every change of the host: the handshake, its context. Returns the unsubscribe. */
export function onHostChange(fn: () => void): () => void {
  changes.add(fn);
  return () => {
    changes.delete(fn);
  };
}

/** Hear every tool result's `structuredContent` (`{}` for a result without one). */
export function onToolResult(fn: (data: unknown) => void): () => void {
  results.add(fn);
  return () => {
    results.delete(fn);
  };
}

/** Hear the opening tool's arguments, also while they stream. */
export function onToolInput(fn: (args: unknown) => void): () => void {
  inputs.add(fn);
  return () => {
    inputs.delete(fn);
  };
}

/** The last tool result delivered, or null. */
export function lastToolResult(): { data: unknown } | null {
  return lastResult;
}

/** The last tool arguments delivered, or null. */
export function lastToolInput(): { args: unknown } | null {
  return lastInput;
}

/** Merge a change of display mode the host confirmed. */
export function setDisplayMode(mode: string): void {
  context = { ...(context ?? {}), displayMode: mode };
  changed();
}
