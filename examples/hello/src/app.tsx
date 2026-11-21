import { h, render } from "preact";
import { useState } from "preact/hooks";
import { label } from "./count";

function App() {
  const [count, setCount] = useState(0);
  return (
    <main class="p-4 text-sm">
      <h1 class="text-2xl font-bold">Hello</h1>
      <button
        type="button"
        id="count"
        class="mt-4 rounded bg-blue-600 px-4 py-2 text-white"
        onClick={() => setCount(count + 1)}
      >
        {label(count)}
      </button>
    </main>
  );
}

render(<App />, document.getElementById("root") as HTMLElement);
