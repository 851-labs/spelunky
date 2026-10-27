// The Vite plugin that puts the wiki's pages into the app (docs/wiki-style.md, "The pipeline"). It compiles
// every page once, when the build starts (compile.ts), fails the build on any error, and serves the results
// as virtual modules:
//
// - `virtual:wiki/pages`: every page's summary (slug, title, lede, categories, verified, and the clips and
//   sprite it shows), for the sitemaps.
// - `virtual:wiki/loaders`: each slug's `() => import("virtual:wiki/page/<slug>")`, so a page's code is its
//   own chunk, loaded only where it's shown.
// - `virtual:wiki/page/<slug>`: the page's module (compile.ts's CompiledPage.code).
// - `virtual:wiki/hubs`: `() => import(...)`s of /wiki's contents (`home`) and each hub's (`hubs`, by id), so
//   a hub's route loads only its own; pre-rendered, the page carries the data it shows, so the browser
//   needn't load it (hubs.ts).
// - `virtual:wiki/home`, `virtual:wiki/hub/<id>` (`virtual:wiki/hub/items/weapons`): those, as JSON.
// - `virtual:wiki/search-index`: the search index of every page (src/wiki/search.ts), which the search box
//   imports the first time it's used: its own chunk, kept out of the pages' bundle.
import { relative } from "node:path";
import type { Plugin, ViteDevServer } from "vite";
import { buildSearchIndex } from "../search.ts";
import { CATEGORY_NAMES } from "./categories.ts";
import { loadWiki } from "./compile.ts";
import type { CompiledPage, WikiOptions } from "./compile.ts";
import type { Hubs } from "./hubs.ts";

const PAGES = "virtual:wiki/pages";
const LOADERS = "virtual:wiki/loaders";
const PAGE = "virtual:wiki/page/";
const HUBS = "virtual:wiki/hubs";
const HOME = "virtual:wiki/home";
const HUB = "virtual:wiki/hub/";
const SEARCH_INDEX = "virtual:wiki/search-index";

export function wiki(options: WikiOptions): Plugin {
  // Compiled once for both of the build's environments (the Worker's and the browser's).
  let compiled: Promise<{ pages: Map<string, CompiledPage>; hubs: Hubs }> | null = null;
  const compileAll = () =>
    (compiled ??= loadWiki(options).then(({ pages, hubs, errors, warnings, wanted }) => {
      for (const warning of warnings) console.warn(`wiki: ${warning}`);
      // The red links: the pages not written yet that pages link to, most linked first (docs/wiki-style.md,
      // "Red links").
      if (wanted.length > 0) {
        const list = wanted.map((w) => `${w.slug} (${w.from.join(", ")})`).join(", ");
        console.info(`wiki: red links to ${wanted.length} page(s) not written yet: ${list}`);
      }
      if (errors.length > 0) {
        throw new Error(`the wiki has ${errors.length} error(s):\n${errors.map((e) => `  ${e}`).join("\n")}`);
      }
      return { pages: new Map(pages.map((page) => [page.meta.slug, page])), hubs };
    }));
  let server: ViteDevServer | undefined;

  return {
    name: "spelunky-wiki",
    configureServer(s) {
      server = s;
      s.watcher.add([
        options.contentDir,
        options.statsFile,
        options.relationsFile,
        options.plannedFile,
        options.clipsDir,
        options.relationsSpec,
      ]);
      // A page added or removed changes the list of pages, which no module's edit does.
      const changed = (file: string) => void (isInput(file) && recompile());
      s.watcher.on("add", changed);
      s.watcher.on("unlink", changed);
    },
    async buildStart() {
      await compileAll();
    },
    resolveId(id) {
      if ([PAGES, LOADERS, HUBS, HOME, SEARCH_INDEX].includes(id) || id.startsWith(PAGE) || id.startsWith(HUB)) return `\0${id}`;
    },
    async load(id) {
      if (!id.startsWith("\0virtual:wiki/")) return;
      const { pages, hubs } = await compileAll();
      const name = id.slice(1);
      if (name === PAGES) {
        const summaries = [...pages.values()].map(({ meta }) => ({
          slug: meta.slug,
          title: meta.title,
          type: meta.type,
          categories: meta.categories,
          lede: meta.lede,
          verified: meta.verified,
          clips: meta.clips.map((clip) => clip.id),
          sprite: meta.image ? { name: meta.image.sprite, anim: meta.image.anim } : null,
        }));
        return `export const pages = ${JSON.stringify(summaries)};`;
      }
      if (name === LOADERS) {
        const entries = [...pages.keys()].map((slug) => `  ${JSON.stringify(slug)}: () => import(${JSON.stringify(PAGE + slug)}),`);
        return `export const loaders = {\n${entries.join("\n")}\n};`;
      }
      if (name === HUBS) {
        const entries = hubs.hubs.map(({ id }) => `  ${JSON.stringify(id)}: () => import(${JSON.stringify(HUB + id)}),`);
        return `export const home = () => import(${JSON.stringify(HOME)});\nexport const hubs = {\n${entries.join("\n")}\n};`;
      }
      if (name === HOME) return `export default ${JSON.stringify(hubs.home)};`;
      if (name.startsWith(HUB)) {
        const hub = hubs.hubs.find(({ id }) => id === name.slice(HUB.length));
        if (!hub) throw new Error(`no wiki hub ${name.slice(HUB.length)}`);
        return `export default ${JSON.stringify(hub)};`;
      }
      if (name === SEARCH_INDEX) {
        const index = buildSearchIndex(
          [...pages.values()].map(({ meta, text }) => ({
            slug: meta.slug,
            title: meta.title,
            aliases: meta.aliases,
            lede: meta.lede,
            categories: meta.categories.map((category) => CATEGORY_NAMES[category]),
            sections: meta.sections,
            text,
          })),
        );
        // JSON.parse of a string literal: quicker for the browser to load than the same object as code.
        return `export const index = JSON.parse(${JSON.stringify(JSON.stringify(index))});`;
      }
      const page = pages.get(name.slice(PAGE.length));
      if (!page) throw new Error(`no wiki page ${name.slice(PAGE.length)}`);
      return page.code;
    },
    // Under `vite dev`, an edited page, snapshot, planned pages' list, clip spec or relations spec
    // recompiles every page (links, checks and navboxes span pages) and reloads.
    async handleHotUpdate({ file }) {
      if (!isInput(file)) return;
      await recompile();
      return [];
    },
  };

  function isInput(file: string) {
    const under = (dir: string) => !relative(dir, file).startsWith("..");
    return (
      (file.endsWith(".mdx") && under(options.contentDir)) ||
      file === options.statsFile ||
      file === options.relationsFile ||
      file === options.plannedFile ||
      file === options.relationsSpec ||
      (file.endsWith(".toml") && under(options.clipsDir))
    );
  }

  async function recompile() {
    compiled = null;
    try {
      await compileAll();
    } catch (e) {
      server?.config.logger.error((e as Error).message);
      return;
    }
    for (const environment of Object.values(server?.environments ?? {})) {
      for (const [id, mod] of environment.moduleGraph.idToModuleMap) {
        if (id.startsWith("\0virtual:wiki/")) environment.moduleGraph.invalidateModule(mod);
      }
    }
    server?.ws.send({ type: "full-reload" });
  }
}
