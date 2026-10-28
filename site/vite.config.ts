import { cloudflare } from "@cloudflare/vite-plugin";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import viteReact from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import { HUBS } from "./src/wiki/build/categories.ts";
import { pageSlugs } from "./src/wiki/build/compile.ts";
import { wiki } from "./src/wiki/build/plugin.ts";
import { wikiMediaPlugin } from "./src/seo/media-package.ts";

// What build.sh adds to dist/client after this build: the release's web build, the license, the update
// manifest and the wiki's media; and the wiki's Open Graph images, which the wiki plugin writes. The router
// serves none of them, so pre-rendering doesn't follow the pages' links there.
const NOT_ROUTES = ["/play/", "/LICENSE.txt", "/releases/", "/wiki/media/", "/wiki/og/"];

// The wiki's pages (docs/wiki-style.md, "The pipeline"), the stats they cite and the relations they're
// checked against, the pages planned but not written, and the clip and relations specs, which are the
// repo's.
const here = (path: string) => fileURLToPath(new URL(path, import.meta.url));
const WIKI = {
  contentDir: here("content/wiki"),
  statsFile: here("content/wiki-data/stats.json"),
  relationsFile: here("content/wiki-data/relations.json"),
  plannedFile: here("content/wiki-planned.yaml"),
  clipsDir: here("../wiki/clips"),
  relationsSpec: here("../wiki/data/relations.toml"),
  // build.sh's, after it unpacks the pinned media (docs/web-deploy.md, "Wiki media").
  media: process.env.WIKI_MEDIA_DIR
    ? { dir: process.env.WIKI_MEDIA_DIR, url: process.env.WIKI_MEDIA_URL ?? "", date: process.env.WIKI_MEDIA_DATE ?? "" }
    : null,
};

export default defineConfig({
  plugins: [
    wiki(WIKI),
    // The Worker (wrangler.jsonc, src/server.ts) runs in Vite's SSR environment, so `vite dev` runs it in
    // workerd, and the build writes dist/client (the static assets) and dist/server (the Worker, with the
    // wrangler.json that `wrangler deploy` picks up through .wrangler/deploy/config.json).
    cloudflare({ viteEnvironment: { name: "ssr" } }),
    tanstackStart({
      // Pre-rendering finds the pages itself, but not a route that's only a server handler. (No `sitemap`
      // option: TanStack's own sitemap stays off, as src/seo/sitemap.ts makes ours.)
      pages: [
        { path: "/" },
        { path: "/sitemap.xml" },
        { path: "/wiki" },
        ...HUBS.map((hub) => ({ path: hub.path })),
        ...pageSlugs(WIKI.contentDir).map((slug) => ({ path: `/wiki/${slug}` })),
        { path: "/sitemap-video.xml" },
        { path: "/sitemap-image.xml" },
        { path: "/robots.txt" },
        { path: "/opensearch.xml" },
      ],
      // Every static route is rendered to HTML at build time, and served as a static asset.
      prerender: {
        enabled: true,
        // /wiki/snake as wiki/snake.html, which the asset store serves at /wiki/snake, with no trailing
        // slash (wrangler.jsonc's auto-trailing-slash), rather than wiki/snake/index.html at /wiki/snake/.
        autoSubfolderIndex: false,
        // An index route's path ends in a slash (/wiki/), and pre-rendering finds it: its page is the path
        // without one, listed above, and another copy at /wiki/ would be a second page with the same canonical.
        filter: ({ path }) =>
          !NOT_ROUTES.some((prefix) => path.startsWith(prefix)) && (path === "/" || !path.endsWith("/")),
      },
    }),
    viteReact(),
    // "virtual:wiki-media": the pinned media's indexes, for the pages' JSON-LD and the sitemaps.
    wikiMediaPlugin(),
  ],
  // Sourcemaps for the Worker, for its stack traces (the Cloudflare plugin's default), and none for the
  // browser, as before. Said at the top level, not only as the Worker's environment's: Vite 8.3.5's
  // vite:css-post asks the top-level build.sourcemap whether to map the URL it puts in for
  // `./wiki.css?url` (src/wiki/stylesheet.tsx), and while that was off it returned no map and Rolldown
  // warned SOURCEMAP_BROKEN. (It's asked when bundling a `?worker` too, which would then ship the browser a
  // map: the site has none.) Fixed after 8.3.5 (vitejs/vite#23879), when this can go back to the defaults
  // (851-3516).
  build: { sourcemap: true },
  environments: { client: { build: { sourcemap: false } } },
});
