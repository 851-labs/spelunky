import { cloudflare } from "@cloudflare/vite-plugin";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import viteReact from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// What build.sh adds to dist/client after this build: the release's web build, the license, the update
// manifest and the wiki's media. The router serves none of them, so pre-rendering doesn't follow the pages'
// links there.
const NOT_ROUTES = ["/play/", "/LICENSE.txt", "/releases/", "/wiki/media/"];

export default defineConfig({
  plugins: [
    // The Worker (wrangler.jsonc, src/server.ts) runs in Vite's SSR environment, so `vite dev` runs it in
    // workerd, and the build writes dist/client (the static assets) and dist/server (the Worker, with the
    // wrangler.json that `wrangler deploy` picks up through .wrangler/deploy/config.json).
    cloudflare({ viteEnvironment: { name: "ssr" } }),
    tanstackStart({
      // Every static route is rendered to HTML at build time, and served as a static asset.
      prerender: {
        enabled: true,
        filter: ({ path }) => !NOT_ROUTES.some((prefix) => path.startsWith(prefix)),
      },
    }),
    viteReact(),
  ],
});
