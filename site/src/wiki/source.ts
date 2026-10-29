// The source viewer's pages as the routes load them (src/routes/wiki/source/): the file or the index, from
// build/source.ts's modules, and their heads. Imported by the routes' loaders when they run, so none of it is
// in the route tree, which every page's entry chunk carries (the landing page's too: site/test-bundle.mjs).
import type { Seo } from "../seo/seo";
import type { PageLink } from "./build/plugin";
import type { SourceFile, SourceIndex, SourceRoom } from "./build/source";
import { roomUrl, sourceSubject, sourceUrl } from "./source-url.ts";

export type { SourceFile, SourceIndex, SourceRoom };

// The page of the file at /wiki/source/SPLAT (objects/oSnake/step for gml/objects/oSnake/step.gml), or of the
// room (rooms/rOlmec), or null if there's none. Looked up in the index first: on spelunky.ai the router renders
// only what isn't a file, a path with no page, which needn't load every file to learn it. noindex, as the index
// is: the game's code, there for the wiki's citations to open, not the wiki's own writing to rank in search
// (docs/wiki-media.md, "Source"), so the sitemap doesn't list them either.
export async function sourcePage(splat: string): Promise<(({ file: SourceFile } | RoomData) & { head: Seo }) | null> {
  const path = `${splat}.gml`;
  const index: SourceIndex = (await import("virtual:wiki-source/index")).default;
  const [kind, name, event] = path.split("/");
  if (kind === "rooms" && event === undefined) return roomPage(index, splat.slice("rooms/".length));
  const listed =
    kind === "scripts"
      ? event === undefined && index.scripts.includes(name)
      : kind === "objects" && index.objects.some((object) => object.name === name && object.events.includes(event));
  if (!listed) return null;
  const file = (await import("virtual:wiki-source/files")).sourceFile(path);
  if (!file) return null;
  const lines = `${file.lines.length} ${file.lines.length === 1 ? "line" : "lines"}`;
  return {
    file,
    head: {
      title: `${path} – GML source – Spelunky Classic wiki`,
      description: `The GML of ${sourceSubject(path)} in Spelunky Classic 1.1's source code (spelunky.gmk), ${lines}, with line numbers.`,
      path: sourceUrl(path),
      noindex: true,
    },
  };
}

// A room's page's data: its name in the URL (its file's, the extractor's: the room's, made safe and unique), the
// room, its objects that have events in the source viewer, and the wiki's pages for its objects (by name) and
// for the room (the place that lists it in its `rooms`), which it links to.
export interface RoomData {
  name: string;
  room: SourceRoom;
  linked: string[];
  pages: Record<string, PageLink>;
  place: PageLink | null;
}

async function roomPage(index: SourceIndex, name: string): Promise<(RoomData & { head: Seo }) | null> {
  if (!index.rooms.includes(name)) return null;
  const room: SourceRoom | null = (await import("virtual:wiki-source/files")).sourceRoom(name);
  if (!room) return null;
  const objects = new Set(room.instances.map((instance) => instance.object));
  const linked = index.objects.map((object) => object.name).filter((object) => objects.has(object));
  // Only this room's, so the page carries no more of the map than it shows.
  const { objectPages, roomPages } = await import("virtual:wiki/source-links");
  const pages = Object.fromEntries([...objects].filter((object) => Object.hasOwn(objectPages, object)).map((object) => [object, objectPages[object]]));
  // By the room's own name, as the place's `rooms` has it, not its file's (the same for every room Spelunky has).
  const place = Object.hasOwn(roomPages, room.name) ? roomPages[room.name] : null;
  const instances = `${room.instances.length} ${room.instances.length === 1 ? "instance" : "instances"}`;
  return {
    name,
    room,
    linked,
    pages,
    place,
    head: {
      title: `${name} – Room source – Spelunky Classic wiki`,
      description: `The room ${name} in Spelunky Classic 1.1's source code (spelunky.gmk), as the room editor placed it: its settings and its ${instances}, with their positions.`,
      path: roomUrl(name),
      noindex: true,
    },
  };
}

// /wiki/source's list of every file, or null without the media's gml/ tree.
export async function sourceIndexPage(): Promise<{ index: SourceIndex; head: Seo } | null> {
  const index = (await import("virtual:wiki-source/index")).default;
  if (index.scripts.length === 0 && index.objects.length === 0 && index.rooms.length === 0) return null;
  return {
    index,
    head: {
      title: "GML source – Spelunky Classic wiki",
      description:
        "Spelunky Classic 1.1's source code: every script and object event of spelunky.gmk as GML, with line " +
        "numbers, and every room, for the wiki's citations to open.",
      path: "/wiki/source",
      noindex: true,
    },
  };
}
