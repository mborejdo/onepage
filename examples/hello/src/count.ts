/** How the count reads. */
export function label(count: number): string {
  return count === 1 ? "Clicked once" : `Clicked ${count} times`;
}
