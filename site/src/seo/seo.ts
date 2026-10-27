// The head every page's route returns: `head: () => seo({ ... })`. One call sets the title, the description,
// the canonical URL, Open Graph and Twitter's tags and the page's JSON-LD, so no page can forget one of them
// or let two disagree. site/test-seo.mjs checks the pre-rendered pages for them (docs/web-deploy.md, "SEO").
import type { JsonLd } from "./jsonld";

export const SITE_URL = "https://spelunky.ai";
export const SITE_NAME = "spelunky.ai";

// Until 851-3340 renders an image per page: the landing page's gameplay still.
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
  // fan wiki").
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
}

// What a route's head() returns: TanStack Router renders `meta` (and each "script:ld+json" as a
// <script type="application/ld+json">, its JSON escaped for a script element) and `links` into <head>.
export interface SeoHead {
  meta: Array<Record<string, unknown>>;
  links: Array<{ rel: string; href: string }>;
}

export function seo({ title, description, path, image = DEFAULT_IMAGE, type = "website", jsonLd = [] }: Seo): SeoHead {
  const url = absoluteUrl(path);
  const imageUrl = absoluteUrl(image.path);
  return {
    meta: [
      { title },
      { name: "description", content: description },
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
    links: [{ rel: "canonical", href: url }],
  };
}

// https://spelunky.ai + a site-relative path; an absolute URL as it is.
export function absoluteUrl(path: string): string {
  if (/^https?:\/\//.test(path)) return path;
  if (!path.startsWith("/")) throw new Error(`not a site-relative path: ${path}`);
  return SITE_URL + path;
}
