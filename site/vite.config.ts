import { cloudflare } from "@cloudflare/vite-plugin";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import viteReact from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import { pageSlugs } from "./src/wiki/build/compile.ts";
import { wiki } from "./src/wiki/build/plugin.ts";

// What build.sh adds to dist/client after this build: the release's web build, the license, the update
// manifest and the wiki's media. The router serves none of them, so pre-rendering doesn't follow the pages'
// links there.
const NOT_ROUTES = ["/play/", "/LICENSE.txt", "/releases/", "/wiki/media/"];

// The wiki's pages (docs/wiki-style.md, "The pipeline"), the stats they cite, and the clip specs, which are
// the repo's.
const here = (path: string) => fileURLToPath(new URL(path, import.meta.url));
const WIKI = {
  contentDir: here("content/wiki"),
  statsFile: here("content/wiki-data/stats.json"),
  clipsDir: here("../wiki/clips"),
  mediaPin: here("wiki-media.txt"),
  // build.sh's, after it unpacks the pinned media (docs/web-deploy.md, "Wiki media").
  media: process.env.WIKI_MEDIA_DIR
    ? { dir: process.env.WIKI_MEDIA_DIR, url: process.env.WIKI_MEDIA_URL ?? "" }
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
        ...pageSlugs(WIKI.contentDir).map((slug) => ({ path: `/wiki/${slug}` })),
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
  ],
});
