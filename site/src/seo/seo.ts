// The head every page's route returns: `head: () => seo({ ... })`. One call sets the title, the description,
// the canonical URL, Open Graph and Twitter's tags and the page's JSON-LD, so no page can forget one of them
// or let two disagree. site/test-seo.mjs checks the pre-rendered pages for them (docs/web-deploy.md, "SEO").
//
// Only a page's own (leaf) route calls it: the router lets a child's meta tags replace its parents', but
// renders every matched route's links, so a layout route's canonical would make a second one.
import type { JsonLd } from "./jsonld.ts";

export const SITE_URL = "https://spelunky.ai";
export const SITE_NAME = "spelunky.ai";

// The wiki's OpenSearch description (src/seo/opensearch.ts), which every page's head names (seo()).
export const OPENSEARCH_PATH = "/opensearch.xml";
export const OPENSEARCH_TYPE = "application/opensearchdescription+xml";
// The search engine's name, in the browser's list and the link's title. At most 16 characters (OpenSearch's
// ShortName), and "Spelunky" only to say what the wiki is about (docs/wiki-license.md, "Trademark"), so the
// site's name.
export const SEARCH_NAME = "spelunky.ai wiki";

// The site's icons (site/public/, made from the game's Spelunker by scripts/site-icons): the ICO every browser
// asks /favicon.ico for, with 16- to 256-pixel images, and the SVG browsers that take one prefer, crisp at any
// size. The OpenSearch description names the ICO too (src/seo/opensearch.ts).
export const FAVICON_PATH = "/favicon.ico";
export const ICON_SVG_PATH = "/icon.svg";
// Apple's touch icon, for an iPhone's home screen and Safari's favourites: 180 pixels, opaque.
export const TOUCH_ICON_PATH = "/apple-touch-icon.png";

// The landing page's gameplay still, for a page without an image of its own (the wiki's have one, made at
// build time from the wiki's media: src/wiki/build/og.ts).
export const DEFAULT_IMAGE: SeoImage = {
  path: "/media/gameplay.png",
  width: 320,
  height: 240,
  alt: "Spelunky Classic gameplay: the player in the Mines",
};

export interface SeoImage {
  // Site-relative (/media/...) or absolute.
  path: string;
  width?: number;
  height?: number;
  alt: string;
}

export interface Seo {
  // The whole <title>, as the page wants it (docs/wiki-license.md, "Trademark": "Shotgun – Spelunky Classic
  // wiki").
  title: string;
  // The meta description, and Open Graph's and Twitter's: a wiki page's lede (docs/wiki-style.md).
  description: string;
  // The path the page is served at, which is its canonical URL: exactly that, so with the trailing slash
  // the asset store serves it at (a route pre-rendered to dir/index.html is at /dir/, and /dir redirects).
  path: string;
  image?: SeoImage;
  // Open Graph's type: "article" for a wiki page.
  type?: "website" | "article";
  jsonLd?: JsonLd[];
  // Keep it out of search engines' indexes (and the sitemap: test-seo.mjs checks), as a page with nothing
  // on it yet: a wiki hub none of whose pages is written.
  noindex?: boolean;
  // The same page in other formats, as <link rel="alternate">s: a wiki page's Markdown twin.
  alternates?: { type: string; path: string }[];
}

// What a route's head() returns: TanStack Router renders `meta` (and each "script:ld+json" as a
// <script type="application/ld+json">, its JSON escaped for a script element) and `links` into <head>.
export interface SeoHead {
  // Loosely typed: the router's React types for meta don't admit its own "script:ld+json" entries.
  meta: Array<Record<string, unknown>>;
  links: Array<{ rel: string; href: string; type?: string; title?: string; sizes?: string }>;
}

export function seo({
  title,
  description,
  path,
  image = DEFAULT_IMAGE,
  type = "website",
  jsonLd = [],
  noindex = false,
  alternates = [],
}: Seo): SeoHead {
  const url = absoluteUrl(path);
  const imageUrl = absoluteUrl(image.path);
  return {
    meta: [
      { title },
      { name: "description", content: description },
      ...(noindex ? [{ name: "robots", content: "noindex" }] : []),
      { property: "og:site_name", content: SITE_NAME },
      { property: "og:type", content: type },
      { property: "og:url", content: url },
      { property: "og:title", content: title },
      { property: "og:description", content: description },
      { property: "og:image", content: imageUrl },
      ...(image.width ? [{ property: "og:image:width", content: String(image.width) }] : []),
      ...(image.height ? [{ property: "og:image:height", content: String(image.height) }] : []),
      { property: "og:image:alt", content: image.alt },
      { property: "og:locale", content: "en_US" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: title },
      { name: "twitter:description", content: description },
      { name: "twitter:image", content: imageUrl },
      { name: "twitter:image:alt", content: image.alt },
      ...jsonLd.map((node) => ({ "script:ld+json": node })),
    ],
    links: [
      { rel: "canonical", href: url },
      ...alternates.map((alternate) => ({ rel: "alternate", type: alternate.type, href: absoluteUrl(alternate.path) })),
      // The wiki's search, for browsers: on the landing page too, as Chrome only adds a search engine a site's
      // root page names.
      { rel: "search", type: OPENSEARCH_TYPE, title: SEARCH_NAME, href: absoluteUrl(OPENSEARCH_PATH) },
      // Site-relative, as the icons are the site's own wherever it's served (a preview's too). The ICO's
      // sizes keep Chrome from taking it over the SVG, as it would one without.
      { rel: "icon", href: FAVICON_PATH, sizes: "32x32" },
      { rel: "icon", type: "image/svg+xml", href: ICON_SVG_PATH },
      { rel: "apple-touch-icon", href: TOUCH_ICON_PATH },
    ],
  };
}

// https://spelunky.ai + a site-relative path; an absolute URL as it is. The path must be a URL's already (no
// spaces or other characters to percent-encode, though it may have escapes: a sprite's 0%404x.png), so a
// page's canonical is the same text everywhere.
export function absoluteUrl(path: string): string {
  if (/^https?:\/\//.test(path)) return path;
  if (!path.startsWith("/")) throw new Error(`not a site-relative path: ${path}`);
  const unescaped = path.replace(/%[0-9A-F]{2}/gi, "");
  if (encodeURI(unescaped) !== unescaped) throw new Error(`a path with characters to percent-encode: ${path}`);
  return SITE_URL + path;
}
