// The wiki's pages in /sitemap.xml (src/seo/sitemap.ts): /wiki and every page, each last modified when it
// was last verified.
import { pages } from "virtual:wiki/pages";
import type { SitemapEntry } from "../seo/sitemap";

export function wikiSitemap(): SitemapEntry[] {
  return [
    { path: "/wiki" },
    ...pages.map((page) => ({ path: `/wiki/${page.slug}`, lastmod: page.verified.date })),
  ];
}
