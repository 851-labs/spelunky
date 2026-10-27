// The wiki's pages in /sitemap.xml (src/seo/sitemap.ts): /wiki, every page, last modified when it was last
// verified, and the hubs that have one written (the rest are noindex: src/wiki/hub-route.ts), undated, since a
// hub changes with wiki-data's entities too. And the media each page shows, for the video and image
// sitemaps.
import { pages } from "virtual:wiki/pages";
import type { PageMedia } from "../seo/media";
import type { SitemapEntry } from "../seo/sitemap";
import { HUBS, inHub } from "./build/categories.ts";
import type { PageSummary } from "./pages";

export function wikiSitemap(): SitemapEntry[] {
  const summaries: PageSummary[] = pages;
  const hubs = HUBS.filter((hub) => summaries.some((page) => inHub(hub, page.categories)));
  return [
    { path: "/wiki" },
    ...hubs.map((hub) => ({ path: hub.path })),
    ...pages.map((page) => ({ path: `/wiki/${page.slug}`, lastmod: page.verified.date })),
  ];
}

// Each page's clips (its `clips`, placed in the body or in the Gallery) and its infobox's sprite, as shown.
export function wikiPageMedia(): PageMedia[] {
  return pages.map((page) => ({
    path: `/wiki/${page.slug}`,
    clips: page.clips,
    sprites: page.sprite ? [page.sprite] : [],
  }));
}
