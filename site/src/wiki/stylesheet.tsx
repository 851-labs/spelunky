import { useMatches } from "@tanstack/react-router";
import href from "./wiki.css?url";

// The wiki's CSS, a content-hashed stylesheet the browser caches across the wiki, for the root's <head>
// while a wiki route is matched. Inlined from the wiki routes' heads, it was 22 KB of every page's HTML and
// of the route tree, which every page's entry chunk carries, the landing page's too (851-3428).
//
// Not one of the heads' `links`: the router gives a stylesheet link a precedence, which makes it React's
// resource, and React never removes one, so after a client-side navigation away from the wiki its rules
// (html, body, a, ...) would stay on the landing page. Without one, the link is an element of the head,
// there only while a wiki route is. On a cold load it blocks rendering, as any stylesheet in the head does;
// a client-side navigation into the wiki doesn't wait for it as it would for a resource, but the site's
// links are plain <a>s, which the router doesn't follow.
export function WikiStylesheet() {
  const wiki = useMatches({ select: (matches) => matches.some((match) => match.routeId.startsWith("/wiki/")) });
  return wiki ? <link rel="stylesheet" href={href} /> : null;
}
