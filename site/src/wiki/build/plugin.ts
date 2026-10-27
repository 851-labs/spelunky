// The Vite plugin that puts the wiki's pages into the app (docs/wiki-style.md, "The pipeline"). It compiles
// every page once, when the build starts (compile.ts), fails the build on any error, and serves the results
// as virtual modules:
//
// - `virtual:wiki/pages`: every page's summary (slug, title, lede, categories, verified, and the clips and
//   sprite it shows), for the /wiki index and the sitemaps.
// - `virtual:wiki/loaders`: each slug's `() => import("virtual:wiki/page/<slug>")`, so a page's code is its
//   own chunk, loaded only where it's shown.
// - `virtual:wiki/page/<slug>`: the page's module (compile.ts's CompiledPage.code).
import { relative } from "node:path";
import type { Plugin, ViteDevServer } from "vite";
import { loadWiki } from "./compile.ts";
import type { CompiledPage, WikiOptions } from "./compile.ts";

const PAGES = "virtual:wiki/pages";
const LOADERS = "virtual:wiki/loaders";
const PAGE = "virtual:wiki/page/";

export function wiki(options: WikiOptions): Plugin {
  // Compiled once for both of the build's environments (the Worker's and the browser's).
  let compiled: Promise<Map<string, CompiledPage>> | null = null;
  const compileAll = () =>
    (compiled ??= loadWiki(options).then(({ pages, errors, warnings }) => {
      for (const warning of warnings) console.warn(`wiki: ${warning}`);
      if (errors.length > 0) {
        throw new Error(`the wiki has ${errors.length} error(s):\n${errors.map((e) => `  ${e}`).join("\n")}`);
      }
      return new Map(pages.map((page) => [page.meta.slug, page]));
    }));
  let server: ViteDevServer | undefined;

  return {
    name: "spelunky-wiki",
    configureServer(s) {
      server = s;
      s.watcher.add([options.contentDir, options.statsFile, options.clipsDir]);
      // A page added or removed changes the list of pages, which no module's edit does.
      const changed = (file: string) => void (isInput(file) && recompile());
      s.watcher.on("add", changed);
      s.watcher.on("unlink", changed);
    },
    async buildStart() {
      await compileAll();
    },
    resolveId(id) {
      if (id === PAGES || id === LOADERS || id.startsWith(PAGE)) return `\0${id}`;
    },
    async load(id) {
      if (!id.startsWith("\0virtual:wiki/")) return;
      const pages = await compileAll();
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
      const page = pages.get(name.slice(PAGE.length));
      if (!page) throw new Error(`no wiki page ${name.slice(PAGE.length)}`);
      return page.code;
    },
    // Under `vite dev`, an edited page, stats snapshot or clip spec recompiles every page (links and checks
    // span pages) and reloads.
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
