// The wiki's pages in /sitemap.xml (src/seo/sitemap.ts): /wiki and every page, each last modified when it
// was last verified. And the media each shows, for the video and image sitemaps.
import { pages } from "virtual:wiki/pages";
import type { PageMedia } from "../seo/media";
import type { SitemapEntry } from "../seo/sitemap";

export function wikiSitemap(): SitemapEntry[] {
  return [
    { path: "/wiki" },
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
