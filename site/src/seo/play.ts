// /play/'s head. The page is the web build's own (crates/app/web/index.html, which a release ships and build.sh
// unpacks), not a route's, so no seo() reaches it: build.sh adds this to it after its <title> (play-head.mjs), so
// that a shared link to the game has a preview, search results a description, and the page a canonical URL (which
// the wiki's live clips' /play/?replay=...&embed frames have too). A release needn't change.
//
// The title stays the release's: it's the copyright notice the Spelunky User License asks a modified version to
// show (crates/app/src/notice.rs's TITLE), the page's only one once the game's overlay of it fades. So Open Graph's
// and Twitter's titles are it too.
import { seo } from "./seo.ts";
import type { SeoHead } from "./seo.ts";

export const PLAY_PATH = "/play/";

export function playHead(title: string): SeoHead {
  return seo({
    title,
    description:
      "Play Spelunky Classic 1.1 in your browser: Derek Yu's 2008 roguelike platformer, in an unofficial, " +
      "tick-for-tick Rust port.",
    path: PLAY_PATH,
  });
}

// The head as HTML, as the router renders a route's, but for the <title>, which the page has: the meta tags and
// the links, and the viewport, which every route's page has from the root route and the release's page hasn't (a
// phone would lay it out 980 pixels wide). The links are site-relative or absolute, so the page's <base> (its
// v/HASH/) doesn't move them.
export function playHeadHtml(head: SeoHead): string {
  const tags = ['<meta name="viewport" content="width=device-width, initial-scale=1">'];
  for (const entry of head.meta) {
    if ("title" in entry) continue;
    if ("script:ld+json" in entry) throw new Error("/play/'s head has no JSON-LD to render");
    tags.push(`<meta${attributes(entry)}>`);
  }
  for (const link of head.links) tags.push(`<link${attributes(link)}>`);
  return tags.join("\n");
}

// name="..." content="..." in the entry's own order, but href last (rel and type first, as the router writes them
// and test-seo.mjs reads them).
function attributes(entry: Record<string, unknown>): string {
  const names = Object.keys(entry).sort((a, b) => Number(a === "href") - Number(b === "href"));
  return names.map((name) => ` ${name}="${escapeHtml(String(entry[name]))}"`).join("");
}

function escapeHtml(text: string): string {
  return text.replaceAll("&", "&amp;").replaceAll('"', "&quot;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

export function unescapeHtml(text: string): string {
  return text
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", '"')
    .replaceAll("&#39;", "'")
    .replaceAll("&amp;", "&");
}
