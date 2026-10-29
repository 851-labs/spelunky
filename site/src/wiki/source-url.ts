// The source viewer's URLs and names (src/routes/wiki/source/), with no dependencies: vite.config.ts lists the
// pages by them.

// Where a file's page is: its path in the gml/ tree without .gml, which the asset store would serve by its
// extension rather than as a page, and the lines its link highlights (#L66, or #L66-L71).
export function sourceUrl(path: string, first?: number, last?: number): string {
  const page = `/wiki/source/${path.replace(/\.gml$/, "")}`;
  if (first === undefined) return page;
  return last === undefined || last === first ? `${page}#L${first}` : `${page}#L${first}-L${last}`;
}

// Where a room's page is (/wiki/source/rooms/rOlmec): where its creation code would be in the gml/ tree
// (rooms/rOlmec/), as the extractor writes it.
export function roomUrl(name: string): string {
  return `/wiki/source/rooms/${name}`;
}

// What a file is, in words: "oSnake's step event", "the script scrEntityGen".
export function sourceSubject(path: string): string {
  const [kind, name, file] = path.split("/");
  if (kind === "scripts") return `the script ${name.replace(/\.gml$/, "")}`;
  return `${name}'s ${file.replace(/\.gml$/, "").replaceAll("_", " ")} event`;
}
