// What /sitemap.xml lists (src/routes/sitemap[.]xml.ts, pre-rendered to a static file): every page a search
// engine should index, by its canonical path. Each source below is a list of pages: the site's own, and the
// wiki's (src/wiki/sitemap.ts). site/test-seo.mjs fails if a pre-rendered page is missing or the
// sitemap lists a page that isn't served.
import { wikiSitemap } from "../wiki/sitemap";
import { absoluteUrl } from "./seo";

export interface SitemapEntry {
  // The canonical path, as seo({ path }) has it.
  path: string;
  // ISO 8601 date of the last real change (a wiki page's verified date); left out rather than guessed.
  lastmod?: string;
}

// The pages outside the wiki: the landing page and the game (build.sh's /play/, not a route).
const SITE_PAGES: SitemapEntry[] = [{ path: "/" }, { path: "/play/" }];

export const SITEMAP_SOURCES: Array<() => SitemapEntry[]> = [() => SITE_PAGES, wikiSitemap];

export function sitemapXml(sources = SITEMAP_SOURCES): string {
  const entries = sources.flatMap((source) => source());
  const seen = new Set<string>();
  const urls = entries.map(({ path, lastmod }) => {
    if (seen.has(path)) throw new Error(`the sitemap has ${path} twice`);
    seen.add(path);
    const loc = `    <loc>${escapeXml(absoluteUrl(path))}</loc>\n`;
    const mod = lastmod === undefined ? "" : `    <lastmod>${escapeXml(lastmod)}</lastmod>\n`;
    return `  <url>\n${loc}${mod}  </url>\n`;
  });
  return (
    '<?xml version="1.0" encoding="UTF-8"?>\n' +
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n' +
    urls.join("") +
    "</urlset>\n"
  );
}

function escapeXml(text: string): string {
  return text.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}
