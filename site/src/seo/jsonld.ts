// Typed builders for the schema.org JSON-LD the pages embed, through seo({ jsonLd: [...] }). Each returns one
// node with its own @context, so every <script type="application/ld+json"> stands alone.
//
// Who's who (docs/wiki-license.md, "JSON-LD"): the site and its Organization are ours, the fan project; the
// VideoGame is Spelunky Classic, by Derek Yu and published by Mossmouth, and nothing here says it's
// official or that Mossmouth runs or endorses the site.
import { absoluteUrl, SITE_NAME, SITE_URL } from "./seo.ts";

export type Json = string | number | boolean | null | Json[] | { [key: string]: Json | undefined };
export type JsonObject = { [key: string]: Json | undefined };
export type JsonLd = JsonObject & { "@context": "https://schema.org"; "@type": string };

const CONTEXT = "https://schema.org";

// Stable ids, so a page's nodes can point at the site, the project and the game.
export const WEBSITE_ID = `${SITE_URL}/#website`;
export const ORGANIZATION_ID = `${SITE_URL}/#organization`;
export const GAME_ID = `${SITE_URL}/#game`;

const LICENSE_URL = `${SITE_URL}/LICENSE.txt`;
const COPYRIGHT = "Spelunky © 2008, 2009 Derek Yu and Mossmouth, LLC";
const SITE_DESCRIPTION =
  "An unofficial fan project: a tick-for-tick Rust port of Spelunky Classic 1.1, playable in the browser. " +
  "Not affiliated with or endorsed by Mossmouth.";

function node(type: string, props: JsonObject): JsonLd {
  return { "@context": CONTEXT, "@type": type, ...props };
}

// Short stand-ins for the site, the project and the game, to point at from another node. Each page carries
// its own (a crawler doesn't follow an @id to another page), with the @id tying them to the full nodes.
export const websiteRef = (): JsonObject => ({
  "@type": "WebSite",
  "@id": WEBSITE_ID,
  name: SITE_NAME,
  url: `${SITE_URL}/`,
});
export const organizationRef = (): JsonObject => ({
  "@type": "Organization",
  "@id": ORGANIZATION_ID,
  name: SITE_NAME,
  url: `${SITE_URL}/`,
});
export const gameRef = (): JsonObject => ({ "@type": "VideoGame", "@id": GAME_ID, name: "Spelunky Classic" });

// The site, with its search box. /wiki?q= is the wiki's search (src/wiki/search-box.tsx).
export function website(): JsonLd {
  return node("WebSite", {
    "@id": WEBSITE_ID,
    name: SITE_NAME,
    url: `${SITE_URL}/`,
    description: SITE_DESCRIPTION,
    inLanguage: "en",
    publisher: organizationRef(),
    potentialAction: {
      "@type": "SearchAction",
      target: { "@type": "EntryPoint", urlTemplate: `${SITE_URL}/wiki?q={search_term_string}` },
      "query-input": "required name=search_term_string",
    },
  });
}

// The fan project that makes the site and the port. Not Mossmouth, and no logo: the Spelunky logo may head
// our pages (docs/wiki-license.md, "Decisions"), but it's the game's, not the project's.
export function organization(): JsonLd {
  return node("Organization", {
    "@id": ORGANIZATION_ID,
    name: SITE_NAME,
    url: `${SITE_URL}/`,
    description: SITE_DESCRIPTION,
    sameAs: ["https://github.com/851-labs/spelunky"],
  });
}

// The original game the site is about: Derek Yu's, published by Mossmouth. The port isn't it.
export function videoGame(): JsonLd {
  return node("VideoGame", {
    "@id": GAME_ID,
    name: "Spelunky Classic",
    alternateName: "Spelunky",
    description:
      "Derek Yu's 2008 freeware roguelike platformer, made in GameMaker: the original Spelunky, since called " +
      "Spelunky Classic.",
    author: { "@type": "Person", name: "Derek Yu" },
    publisher: { "@type": "Organization", name: "Mossmouth, LLC" },
    copyrightNotice: COPYRIGHT,
    genre: ["Platform game", "Roguelike"],
    gamePlatform: "Windows",
    url: "https://spelunkyworld.com",
    sameAs: ["https://en.wikipedia.org/wiki/Spelunky"],
  });
}

export interface Crumb {
  name: string;
  path: string;
}

// A page's trail, from the top: [{ name: "Wiki", path: "/wiki" }, { name: "Monsters", ... }, ...].
export function breadcrumbs(crumbs: Crumb[]): JsonLd {
  return node("BreadcrumbList", {
    itemListElement: crumbs.map((crumb, i) => ({
      "@type": "ListItem",
      position: i + 1,
      name: crumb.name,
      item: absoluteUrl(crumb.path),
    })),
  });
}

export interface ArticleProps {
  headline: string;
  description: string;
  path: string;
  // What the page is about: a Thing for the monster or item (`{ "@type": "Thing", name: "Snake" }`), the game
  // itself by default.
  about?: JsonObject | JsonObject[];
  image?: string;
  // ISO 8601 dates: a wiki page's verified date (docs/wiki-style.md, `verified`).
  dateModified?: string;
  datePublished?: string;
}

// A wiki page.
export function article({
  headline,
  description,
  path,
  about = gameRef(),
  image,
  dateModified,
  datePublished,
}: ArticleProps): JsonLd {
  const url = absoluteUrl(path);
  return node("Article", {
    "@id": `${url}#article`,
    headline,
    description,
    url,
    mainEntityOfPage: url,
    about,
    image: image === undefined ? undefined : absoluteUrl(image),
    dateModified,
    datePublished,
    inLanguage: "en",
    author: organizationRef(),
    publisher: organizationRef(),
    isPartOf: websiteRef(),
  });
}

// A hub's list of pages, in order.
export function itemList(name: string, items: Crumb[]): JsonLd {
  return node("ItemList", {
    name,
    numberOfItems: items.length,
    itemListElement: items.map((item, i) => ({
      "@type": "ListItem",
      position: i + 1,
      name: item.name,
      url: absoluteUrl(item.path),
    })),
  });
}

// Every image and clip shows the game's assets, so each says whose they are and under what license
// (docs/wiki-license.md, "JSON-LD"), unless the caller gives its own (an image of ours, say).
export interface MediaRights {
  creditText?: string;
  copyrightNotice?: string;
  license?: string;
}

const MEDIA_RIGHTS: MediaRights = {
  creditText: "Derek Yu and Mossmouth, LLC (Spelunky Classic)",
  copyrightNotice: COPYRIGHT,
  license: LICENSE_URL,
};

export interface VideoProps extends MediaRights {
  name: string;
  description: string;
  contentUrl: string;
  thumbnailUrl: string;
  // ISO 8601: when the clip was rendered.
  uploadDate: string;
  // ISO 8601 duration, "PT6S".
  duration?: string;
  // The page it plays on.
  embedUrl?: string;
  width?: number;
  height?: number;
}

export function videoObject({ contentUrl, thumbnailUrl, embedUrl, ...rest }: VideoProps): JsonLd {
  return node("VideoObject", {
    ...MEDIA_RIGHTS,
    ...rest,
    contentUrl: absoluteUrl(contentUrl),
    thumbnailUrl: absoluteUrl(thumbnailUrl),
    embedUrl: embedUrl === undefined ? undefined : absoluteUrl(embedUrl),
  });
}

export interface ImageProps extends MediaRights {
  contentUrl: string;
  name?: string;
  caption?: string;
  width?: number;
  height?: number;
}

export function imageObject({ contentUrl, ...rest }: ImageProps): JsonLd {
  return node("ImageObject", { ...MEDIA_RIGHTS, ...rest, contentUrl: absoluteUrl(contentUrl) });
}

export interface Faq {
  question: string;
  // Plain text: a page's FAQ answers rendered without markup (docs/wiki-style.md, "FAQ blocks").
  answer: string;
}

export function faqPage(faqs: Faq[]): JsonLd {
  return node("FAQPage", {
    mainEntity: faqs.map(({ question, answer }) => ({
      "@type": "Question",
      name: question,
      acceptedAnswer: { "@type": "Answer", text: answer },
    })),
  });
}
