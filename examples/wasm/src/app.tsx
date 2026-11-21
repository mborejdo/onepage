import { h, render } from "preact";
import { useEffect, useState } from "preact/hooks";

type Add = (a: number, b: number) => number;

function App() {
  const [sum, setSum] = useState<string>("loading");
  useEffect(() => {
    fetch("add.wasm")
      .then((r) => r.arrayBuffer())
      .then((bytes) => WebAssembly.instantiate(bytes))
      .then(({ instance }) => setSum(String((instance.exports.add as Add)(2, 3))))
      .catch((e: Error) => setSum(`failed: ${e.message}`));
  }, []);
  return (
    <p id="sum" class="p-4 font-mono">
      2 + 3 = {sum}
    </p>
  );
}

render(<App />, document.getElementById("root") as HTMLElement);
