// The source viewer's pages as the routes load them (src/routes/wiki/source/): the file or the index, from
// build/source.ts's modules, and their heads. Imported by the routes' loaders when they run, so none of it is
// in the route tree, which every page's entry chunk carries (the landing page's too: site/test-bundle.mjs).
import type { Seo } from "../seo/seo";
import type { SourceFile, SourceIndex } from "./build/source";
import { sourceSubject, sourceUrl } from "./source-url.ts";

export type { SourceFile, SourceIndex };

// The page of the file at /wiki/source/SPLAT (objects/oSnake/step for gml/objects/oSnake/step.gml), or null if
// there's none. Looked up in the index first: on spelunky.ai the router renders only what isn't a file, a path
// with no page, which needn't load every file to learn it. noindex, as the index is: the game's code, there for the wiki's citations to open, not the
// wiki's own writing to rank in search (docs/wiki-media.md, "Source"), so the sitemap doesn't list them either.
export async function sourcePage(splat: string): Promise<{ file: SourceFile; head: Seo } | null> {
  const path = `${splat}.gml`;
  const index: SourceIndex = (await import("virtual:wiki-source/index")).default;
  const [kind, name, event] = path.split("/");
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
      title: `${path} – GML source – Spelunky Classic fan wiki`,
      description: `The GML of ${sourceSubject(path)} in Spelunky Classic 1.1's source code (spelunky.gmk), ${lines}, with line numbers.`,
      path: sourceUrl(path),
      noindex: true,
    },
  };
}

// /wiki/source's list of every file, or null without the media's gml/ tree.
export async function sourceIndexPage(): Promise<{ index: SourceIndex; head: Seo } | null> {
  const index = (await import("virtual:wiki-source/index")).default;
  if (index.scripts.length === 0 && index.objects.length === 0) return null;
  return {
    index,
    head: {
      title: "GML source – Spelunky Classic fan wiki",
      description:
        "Spelunky Classic 1.1's source code: every script and object event of spelunky.gmk as GML, with line " +
        "numbers, for the wiki's citations to open.",
      path: "/wiki/source",
      noindex: true,
    },
  };
}
