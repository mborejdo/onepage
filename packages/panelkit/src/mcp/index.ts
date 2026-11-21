/** `panelkit/mcp`: a page that runs as an MCP App - the HTML an MCP server lists as a `ui://`
 *  resource and a chat host shows in a sandboxed frame.
 *
 *  Importing it connects (host.ts): in a frame it shakes hands with the host and follows its
 *  theme; opened directly it stays disconnected, and every tool call goes to
 *  `window.__panelkitMocks[name](args)` instead, so the same page runs on its own.
 *
 *  - `useHost`: connected, theme, display mode, whether fullscreen is offered, the frame.
 *  - `useTool`: a tool call as state, repeated when its inputs change, polled with `every`.
 *  - `useToolResult`: what the opening tool returned, and its arguments.
 *  - `callTool`, `requestFullscreen`, `exitFullscreen`, `openLink`, `sendMessage`,
 *    `updateModelContext`, `sendLog`: for a click. Without a host each degrades (`false`, or a
 *    rejection "no host"); none throws. */

import { useEffect, useRef, useState } from "preact/hooks";
import {
  asResult,
  type CallToolResult,
  DISCONNECTED,
  type HostState,
  hostStateOf,
  IDLE,
  NO_HOST,
  reasonOf,
  sameHostState,
  settle,
  type ToolState,
} from "./core";
import {
  connect,
  hostContext,
  isConnected,
  lastToolInput,
  lastToolResult,
  mocks,
  notify,
  onHostChange,
  onToolInput,
  onToolResult,
  request,
  setDisplayMode,
} from "./host";

export {
  answerOf,
  asResult,
  DISCONNECTED,
  hostStateOf,
  nextSize,
  NO_HOST,
  textOf,
} from "./core";
export type {
  CallToolResult,
  DisplayMode,
  HostContext,
  HostState,
  ToolAnswer,
  ToolMock,
  ToolState,
} from "./core";
export {
  connect,
  hostCapabilities,
  hostContext,
  lastToolResult,
  onToolInput,
  onToolResult,
  PROTOCOL_VERSION,
} from "./host";

const ready = connect(typeof document !== "undefined" && document.title ? document.title : "panelkit");

/** The host as it stands, read once. */
function hostNow(): HostState {
  return isConnected() ? hostStateOf(true, hostContext()) : DISCONNECTED;
}

/** Whether a host answered the handshake. */
export function hasHost(): boolean {
  return isConnected();
}

/** Call a tool on the serving MCP server through the host (`tools/call`). With no host, a
 *  mock answers; with neither, it rejects "no host". A call made during the handshake waits
 *  for it. */
export function callTool(
  name: string,
  args: Record<string, unknown> = {},
): Promise<CallToolResult> {
  return ready.then((up) => {
    if (up) {
      return request("tools/call", { name, arguments: args }) as Promise<CallToolResult>;
    }
    const mock = mocks()?.[name];
    if (typeof mock !== "function") throw new Error(NO_HOST);
    return Promise.resolve(mock(args)).then(asResult);
  });
}

/** Ask for a display mode; true when asked. The host has the last word. */
function displayMode(mode: string): boolean {
  if (!isConnected()) return false;
  request("ui/request-display-mode", { mode }).then(
    (answer) => {
      const granted = (answer as { mode?: string } | null)?.mode;
      if (typeof granted === "string") setDisplayMode(granted);
    },
    (e: unknown) => console.warn("panelkit/mcp: display mode declined:", reasonOf(e)),
  );
  return true;
}

/** Ask the host to show the page fullscreen, when it offers the mode; true when asked. Ask
 *  on a person's click, never on render. */
export function requestFullscreen(): boolean {
  return hostNow().canFullscreen && displayMode("fullscreen");
}

/** Ask the host to put the page back into the conversation; true when asked. */
export function exitFullscreen(): boolean {
  return displayMode("inline");
}

/** Open a link host-side (`ui/open-link`): the sandbox blocks the page's own navigation. */
export function openLink(url: string): boolean {
  if (!isConnected() || !url) return false;
  request("ui/open-link", { url }).catch(() => {});
  return true;
}

/** Post `text` into the conversation as the user (`ui/message`); the model answers it. */
export function sendMessage(text: string): boolean {
  if (!isConnected() || !text) return false;
  request("ui/message", { role: "user", content: [{ type: "text", text }] }).catch(() => {});
  return true;
}

/** Tell the model what happened on the page, for its next turn; each call replaces the last. */
export function updateModelContext(text: string): boolean {
  if (!isConnected()) return false;
  request("ui/update-model-context", { content: [{ type: "text", text }] }).catch(() => {});
  return true;
}

/** A line in the host's log (`notifications/message`), not in the conversation. */
export function sendLog(level: "debug" | "info" | "warning" | "error", data: string): void {
  if (isConnected()) notify("notifications/message", { level, data });
}

/** The host, as state; re-renders when it connects or its context changes. */
export function useHost(): HostState {
  const [state, setState] = useState<HostState>(hostNow);
  const last = useRef(state);
  useEffect(() => {
    const look = () => {
      const next = hostNow();
      if (!sameHostState(last.current, next)) {
        last.current = next;
        setState(next);
      }
    };
    const off = onHostChange(look);
    ready.then(look);
    return off;
  }, []);
  return state;
}

/** What `useTool` hands back: the call's state and a way to ask again. */
export interface ToolRead<T> extends ToolState<T> {
  /** Call again, keeping the data that stands until the new answer lands. */
  reload: () => void;
}

/** A tool on the serving MCP server as state: `data` its `structuredContent`, `text` its text,
 *  `error` the reason a call failed, `loading` while one is out. `name` null clears it. The
 *  call repeats when `name` or `args` change (compared as JSON), and an answer that arrives
 *  after a newer call started is dropped. `every` polls: the same call that many milliseconds
 *  after each answer, keeping the data. */
export function useTool<T = unknown>(
  name: string | null,
  args: Record<string, unknown> = {},
  opts?: { every?: number },
): ToolRead<T> {
  const [state, setState] = useState<ToolState<T>>(IDLE);
  const [tick, setTick] = useState(0);
  const keep = useRef(false);
  const current = useRef(state);
  const key = JSON.stringify(args);
  const every = opts?.every ?? 0;
  // biome-ignore lint/correctness/useExhaustiveDependencies: `tick` is what `reload` bumps
  useEffect(() => {
    const set = (next: ToolState<T>) => {
      current.current = next;
      setState(next);
    };
    const hold = keep.current;
    keep.current = false;
    if (!name) {
      set(IDLE);
      return undefined;
    }
    let live = true;
    let timer: ReturnType<typeof setTimeout> | null = null;
    const run = (stay: boolean) => {
      set({ ...(stay ? current.current : IDLE), loading: true });
      callTool(name, JSON.parse(key) as Record<string, unknown>)
        .then(
          (result) => {
            if (live) set(settle(current.current, result));
          },
          (e: unknown) => {
            if (live) set({ ...current.current, error: reasonOf(e), loading: false });
          },
        )
        .then(() => {
          if (live && every > 0) timer = setTimeout(() => run(true), every);
        });
    };
    run(hold);
    return () => {
      live = false;
      if (timer) clearTimeout(timer);
    };
  }, [name, key, tick, every]);
  return {
    ...state,
    reload: () => {
      keep.current = true;
      setTick((t) => t + 1);
    },
  };
}

/** What the opening tool returned (`data`, its `structuredContent`; `{}` for a result without
 *  one; null until it arrives) and its arguments (`input`, also while they stream). */
export function useToolResult<T = unknown>(): {
  data: T | null;
  input: Record<string, unknown> | null;
} {
  const [data, setData] = useState<T | null>(() => (lastToolResult()?.data ?? null) as T | null);
  const [input, setInput] = useState<Record<string, unknown> | null>(
    () => (lastToolInput()?.args ?? null) as Record<string, unknown> | null,
  );
  useEffect(() => {
    const offResult = onToolResult((d) => setData(d as T));
    const offInput = onToolInput((a) => setInput((a ?? null) as Record<string, unknown> | null));
    return () => {
      offResult();
      offInput();
    };
  }, []);
  return { data, input };
}
