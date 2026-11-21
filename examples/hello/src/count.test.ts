import { label } from "./count";

test("one click reads as once", () => {
  assert(label(1) === "Clicked once");
});

test("more clicks are counted", () => {
  assert(label(3) === "Clicked 3 times", label(3));
});
