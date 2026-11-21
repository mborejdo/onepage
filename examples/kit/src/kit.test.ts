// The kit's pure parts, in `onepage test`: the counter's flips and the MCP client's reading
// of results and host states.

import { flaps } from "panelkit";
import { answerOf, asResult, hostStateOf, nextSize } from "panelkit/mcp";

test("a counter flips only the digits that changed, from the right", () => {
  const first = flaps(null, 66, "66");
  assert(
    first.cells.every((c) => c.flip === null),
    "the first render flips nothing",
  );
  const next = flaps(first.memory, 67, "67");
  assert(next.cells[0]?.flip === null && next.cells[1]?.flip === "up", "66 -> 67 flips one");
  const fall = flaps(next.memory, 100, "100");
  assert(
    fall.cells.every((c) => c.flip === "up"),
    "99 -> 100 flips every cell",
  );
});

test("a tool result reads as data, text and error", () => {
  const ok = answerOf({ structuredContent: { a: 1 }, content: [{ type: "text", text: "hi" }] });
  assert(ok.error === null && ok.text === "hi", "a plain result");
  const bad = answerOf({ isError: true, content: [{ type: "text", text: "no such row" }] });
  assert(bad.error === "no such row", "a failed tool says why");
  assert(asResult(5).structuredContent === 5, "a mock's bare value is the content");
});

test("the host state follows the context", () => {
  assert(!hostStateOf(false, null).connected, "no host");
  const s = hostStateOf(true, { theme: "dark", availableDisplayModes: ["inline"] });
  assert(s.theme === "dark" && !s.canFullscreen, "dark, inline only");
  assert(nextSize({ width: 10, height: 20 }, 10, 19.2) === null, "the same size is not sent");
});
