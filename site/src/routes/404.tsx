import { createFileRoute } from "@tanstack/react-router";
import { ICON_LINKS } from "../seo/seo";
import { NotFound } from "../site/not-found";

// The 404 page (851-4003): the site's frame round a line saying there's nothing here, and the ways on
// (src/site/not-found.tsx). Pre-rendered to 404.html, which the Worker serves for any path that isn't a page
// (src/server.ts), so it's shown at the path asked for: it has no scripts (the root leaves them out, and
// not-found-page.mjs what the pre-render adds), which would hydrate a router at that path and find another page.
// Its search box is the plain form, which needs none. Its stylesheet is the root's to link (__root.tsx).
export const Route = createFileRoute("/404")({
  head: () => ({
    meta: [{ title: "Not found - spelunky.ai" }, { name: "robots", content: "noindex" }],
    links: ICON_LINKS,
  }),
  component: NotFound,
});
