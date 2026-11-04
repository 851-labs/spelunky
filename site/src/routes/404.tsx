import { createFileRoute } from "@tanstack/react-router";
import { FAVICON_PATH, ICON_SVG_PATH, TOUCH_ICON_PATH } from "../seo/seo";
import { SiteFrame, SiteSearch } from "../site/frame";
import href from "../site/not-found.css?url";

// The 404 page (851-4003): the site's frame (src/site/frame.tsx) round a line saying there's nothing here, and the
// ways on. Pre-rendered to 404.html, which the Worker serves for any path that isn't a page (src/server.ts), so it's
// shown at the path asked for: it has no scripts (the root leaves them out, __root.tsx), which would hydrate a
// router at that path and find another page. Its search box is the plain form, which needs none. Its CSS is a
// stylesheet of its own, so the route tree, in every page's entry chunk, carries only its URL.
export const Route = createFileRoute("/404")({
  head: () => ({
    meta: [{ title: "Not found - spelunky.ai" }, { name: "robots", content: "noindex" }],
    links: [
      { rel: "stylesheet", href },
      { rel: "icon", href: FAVICON_PATH, sizes: "32x32" },
      { rel: "icon", type: "image/svg+xml", href: ICON_SVG_PATH },
      { rel: "apple-touch-icon", href: TOUCH_ICON_PATH },
    ],
  }),
  component: NotFound,
});

function NotFound() {
  return (
    <SiteFrame
      section={null}
      search={<SiteSearch />}
      footer={
        <p>
          spelunky.ai &middot; <a href="/wiki">Wiki</a> &middot; <a href="https://github.com/851-labs/spelunky">GitHub</a>
        </p>
      }
    >
      <h1>Nothing down here.</h1>
      <p>
        There's no page at this address. <a href="/">Back to the surface</a>, <a href="/play/">play</a>, or search the{" "}
        <a href="/wiki">wiki</a>.
      </p>
    </SiteFrame>
  );
}
