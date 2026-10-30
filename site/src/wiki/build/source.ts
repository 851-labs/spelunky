// The source viewer's build (docs/wiki-media.md, "Source"): the game's GML, a page per script and object event
// at /wiki/source/<path> (src/routes/wiki/source/), from the gml/ tree in the wiki's media (wiki/media), and its
// rooms, a page each at /wiki/source/rooms/<name>, from the media's rooms/, which scripts/wiki-media copies from
// scripts/extract's. The site's build can't extract them (Workers Builds has no Rust), as it can't render the
// clips. Media without a gml/ or a rooms/ (a stand-in's) give the site none of those pages, as no media do.
//
// The Vite plugin highlights every file once (gml-highlight.ts) and serves them as virtual modules, which the
// routes import when they load, so no other page carries them:
//
// - `virtual:wiki-source/files`: `sourceFile(path)`, the file at PATH as its page shows it (SourceFile), or
//   null, and `sourceRoom(name)`, the room NAME (SourceRoom), or null. Every file and room, in the Worker's
//   build, which pre-renders the pages; a page carries its own file or room in its loader's data, which the
//   browser hydrates it with. None in the browser's, which never runs the loader, as the site's links are plain
//   <a>s the router doesn't follow (src/wiki/stylesheet.tsx): a chunk a file was 1,140 more static files, of
//   the 20,000 Workers takes (site/build.sh).
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

// A room as its page shows it: the extractor's rooms/<name>.json (crates/tools' extract, `RoomFile`), objects,
// backgrounds and views by name.
export interface SourceRoom {
  name: string;
  caption: string;
  width: number;
  height: number;
  speed: number;
  persistent: boolean;
  // #rrggbb.
  background_color: string;
  draw_background_color: boolean;
  creation_code: string;
  // The slots that show a background, or are visible without one.
  backgrounds: {
    slot: number;
    visible: boolean;
    foreground: boolean;
    background: string | null;
    x: number;
    y: number;
    tile_h: boolean;
    tile_v: boolean;
    hspeed: number;
    vspeed: number;
    stretch: boolean;
  }[];
  views_enabled: boolean;
  // The visible views.
  views: {
    slot: number;
    view_x: number;
    view_y: number;
    view_w: number;
    view_h: number;
    port_x: number;
    port_y: number;
    port_w: number;
    port_h: number;
    hborder: number;
    vborder: number;
    hspeed: number;
    vspeed: number;
    follow: string | null;
  }[];
  tiles: { id: number; background: string; x: number; y: number; bg_x: number; bg_y: number; width: number; height: number; depth: number }[];
  // In creation order.
  instances: { id: number; object: string; x: number; y: number; creation_code?: string }[];
  // Its picture (scripts/wiki-media's wiki-rooms: rooms/<name>.png), where the media have one: its URL and size
  // in game pixels, and the box each instance's sprite covers in it (x, y, width, height), in the instances'
  // order, null for one whose object has no sprite. The build adds it; the media's JSON hasn't got it.
  picture?: { src: string; width: number; height: number; boxes: ([number, number, number, number] | null)[] };
}

// What a room's picture needs from the media: where they're served, and their sprites/ (index.json's objects'
// sprites, each sprite.json's size and origin).
export interface RoomPictures {
  url: string;
  spritesDir: string;
}

// By file name (scrEntityGen.gml, step.gml), and the rooms by name.
export interface SourceIndex {
  scripts: string[];
  objects: { name: string; events: string[] }[];
  rooms: string[];
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

// The rooms in the media's rooms/ (DIR), by name, in ASCII's order; null without one, when the room citations
// can't be checked or linked (compile.ts).
export function roomNames(dir: string | null): string[] | null {
  if (dir === null || !existsSync(dir)) return null;
  const names = readdirSync(dir).filter((file) => file.endsWith(".json")).map((file) => file.slice(0, -".json".length));
  return names.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
}

// A PNG's width and height, from its IHDR chunk.
function pngSize(bytes: Buffer): [number, number] {
  return [bytes.readUInt32BE(16), bytes.readUInt32BE(20)];
}

// The room's JSON with its picture, where rooms/ has one: each instance's box is its object's sprite, placed by
// its origin, as wiki-rooms draws it.
export function withPicture(roomsDir: string, name: string, json: string, pictures: RoomPictures | null): string {
  const file = join(roomsDir, `${name}.png`);
  if (!pictures || !existsSync(file)) return json;
  const room = JSON.parse(json) as SourceRoom;
  const index = JSON.parse(readFileSync(join(pictures.spritesDir, "index.json"), "utf8")) as { objects: { name: string; sprite: string | null }[] };
  const spriteOf = new Map(index.objects.map((o) => [o.name, o.sprite]));
  const sprites = new Map<string, { width: number; height: number; origin?: { x: number; y: number } } | null>();
  const sprite = (name: string) => {
    if (!sprites.has(name)) {
      const meta = join(pictures.spritesDir, name, "sprite.json");
      sprites.set(name, existsSync(meta) ? JSON.parse(readFileSync(meta, "utf8")) : null);
    }
    return sprites.get(name)!;
  };
  const [width, height] = pngSize(readFileSync(file));
  room.picture = {
    src: `${pictures.url}/rooms/${encodeURIComponent(name)}.png`,
    width,
    height,
    boxes: room.instances.map((instance) => {
      const name = spriteOf.get(instance.object);
      const s = name ? sprite(name) : null;
      return s?.origin ? [instance.x - s.origin.x, instance.y - s.origin.y, s.width, s.height] : null;
    }),
  };
  return JSON.stringify(room);
}

// `dir` is the media's gml/, `roomsDir` their rooms/, `pictures` what the rooms' pictures need of the media.
export function wikiSource(dir: string | null, roomsDir: string | null, pictures: RoomPictures | null = null): Plugin {
  let files: Map<string, string> | null = null;
  // Read once, for both of the build's environments; the rooms' files only for the Worker's.
  const load = () => (files ??= new Map(sourcePaths(dir).map((path) => [path, readFileSync(join(dir!, path), "utf8")])));
  const rooms = roomNames(roomsDir) ?? [];
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
      const index = sourceIndex([...all.keys()], rooms);
      if (name === INDEX) return `export default ${JSON.stringify(index)};`;
      if (this.environment.name === "client") return "export const sourceFile = () => null;\nexport const sourceRoom = () => null;";
      // Each file's and room's JSON as a string, parsed only when its page is rendered. A room's is the media's
      // file, with its picture.
      const entries = [...all].map(([path, text]) => `  ${JSON.stringify(path)}: ${JSON.stringify(json(index, path, text))},`);
      const roomEntries = rooms.map((room) => `  ${JSON.stringify(room)}: ${JSON.stringify(withPicture(roomsDir!, room, readFileSync(join(roomsDir!, `${room}.json`), "utf8"), pictures))},`);
      return [
        `const files = {\n${entries.join("\n")}\n};`,
        `const rooms = {\n${roomEntries.join("\n")}\n};`,
        "export const sourceFile = (path) => (Object.hasOwn(files, path) ? JSON.parse(files[path]) : null);",
        "export const sourceRoom = (name) => (Object.hasOwn(rooms, name) ? JSON.parse(rooms[name]) : null);",
      ].join("\n");
    },
  };
}

// The paths, in sourcePaths' order, and the rooms, in roomNames', as the index lists them.
function sourceIndex(paths: string[], rooms: string[]): SourceIndex {
  const index: SourceIndex = { scripts: [], objects: [], rooms };
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
