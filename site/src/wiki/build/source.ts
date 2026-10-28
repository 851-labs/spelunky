// The source viewer's build (docs/wiki-media.md, "Source"): the game's GML, a page per script and object event
// at /wiki/source/<path> (src/routes/wiki/source/), from the gml/ tree in the wiki's media package, which
// scripts/wiki-media-package copies from scripts/extract's. The site's build can't extract it (Workers Builds
// has no Rust), as it can't render the clips. A package from before the viewer has no gml/, and then nor
// does the site: no /wiki/source pages, as without the media.
//
// The Vite plugin highlights every file once (gml-highlight.ts) and serves them as virtual modules, which the
// routes import when they load, so no other page carries them:
//
// - `virtual:wiki-source/files`: `sourceFile(path)`, the file at PATH as its page shows it (SourceFile), or
//   null. Every file, in the Worker's build, which pre-renders the pages; a page carries its own file in its
//   loader's data, which the browser hydrates it with. None in the browser's, which never runs the loader, as
//   the site's links are plain <a>s the router doesn't follow (src/wiki/stylesheet.tsx): a chunk a file was
//   1,140 more static files, of the 20,000 Workers takes (site/build.sh).
// - `virtual:wiki-source/index`: what /wiki/source lists (SourceIndex).
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import type { Plugin } from "vite";
import { highlightGml } from "./gml-highlight.ts";

const FILES = "virtual:wiki-source/files";
const INDEX = "virtual:wiki-source/index";

// A file as its page shows it.
export interface SourceFile {
  // In the gml/ tree, as pages cite it: objects/oSnake/step.gml.
  path: string;
  // Each line's HTML.
  lines: string[];
  // An object event's object's other events, by file name (create.gml), with this one, in the index's order.
  siblings: string[];
}

// By file name (scrEntityGen.gml, step.gml).
export interface SourceIndex {
  scripts: string[];
  objects: { name: string; events: string[] }[];
}

// The tree's files, as paths in it, in the index's order: the scripts by name, then each object's events, the
// objects by name and their events by file name, in ASCII's order (so alarm_10 before alarm_2), whatever the
// locale.
export function sourcePaths(dir: string | null): string[] {
  if (dir === null || !existsSync(dir)) return [];
  const sorted = (names: string[]) => names.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  const list = (sub: string) => (existsSync(join(dir, sub)) ? readdirSync(join(dir, sub)) : []);
  const gml = (names: string[]) => sorted(names.filter((name) => name.endsWith(".gml")));
  const scripts = gml(list("scripts")).map((file) => `scripts/${file}`);
  const objects = sorted(list("objects")).flatMap((object) =>
    gml(readdirSync(join(dir, "objects", object))).map((file) => `objects/${object}/${file}`),
  );
  return [...scripts, ...objects];
}

export function wikiSource(dir: string | null): Plugin {
  let files: Map<string, string> | null = null;
  // Read once, for both of the build's environments.
  const load = () => (files ??= new Map(sourcePaths(dir).map((path) => [path, readFileSync(join(dir!, path), "utf8")])));
  // A file as its page shows it, as JSON.
  const json = (index: SourceIndex, path: string, text: string) => {
    const [kind, object] = path.split("/");
    const siblings = kind === "objects" ? index.objects.find(({ name }) => name === object)!.events : [];
    const file: SourceFile = { path, lines: highlightGml(text), siblings };
    return JSON.stringify(file);
  };

  return {
    name: "spelunky-wiki-source",
    resolveId(id) {
      if (id === FILES || id === INDEX) return `\0${id}`;
    },
    load(id) {
      if (!id.startsWith("\0virtual:wiki-source/")) return;
      const name = id.slice(1);
      const all = load();
      const index = sourceIndex([...all.keys()]);
      if (name === INDEX) return `export default ${JSON.stringify(index)};`;
      if (this.environment.name === "client") return "export const sourceFile = () => null;";
      // Each file's JSON as a string, parsed only when its page is rendered.
      const entries = [...all].map(([path, text]) => `  ${JSON.stringify(path)}: ${JSON.stringify(json(index, path, text))},`);
      return `const files = {\n${entries.join("\n")}\n};\nexport const sourceFile = (path) => (Object.hasOwn(files, path) ? JSON.parse(files[path]) : null);`;
    },
  };
}

// The paths, in sourcePaths' order, as the index lists them.
function sourceIndex(paths: string[]): SourceIndex {
  const index: SourceIndex = { scripts: [], objects: [] };
  for (const path of paths) {
    const [kind, name, file] = path.split("/");
    if (kind === "scripts") {
      index.scripts.push(name);
      continue;
    }
    if (index.objects.at(-1)?.name !== name) index.objects.push({ name, events: [] });
    index.objects.at(-1)!.events.push(file);
  }
  return index;
}
