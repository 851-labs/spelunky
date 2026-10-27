// The SEO checks site/test.sh runs on the built site (docs/web-deploy.md, "SEO"):
//
//   node test-seo.mjs DIST [SITE]
//
// DIST is the built static assets (dist/client). Every pre-rendered page (its HTML files, but the web build
// at /play/ and the 404 page) must have one title, description and canonical URL, none shared with another
// page, its canonical must be where it's served, and its JSON-LD must parse, with @context and a known
// @type that has the properties search engines require of it. The sitemap must list exactly those pages
// (and /play/), but for those a robots meta tag keeps out of the index, and robots.txt point at it. The video and image sitemaps must list only pages the sitemap
// does, each with its media under /wiki/media/, and robots.txt name them if they list any. Given SITE (a
// running server: wrangler dev), each of their URLs must be served there as is, a 200 and not a redirect.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, relative, sep } from "node:path";

const SITE_URL = "https://spelunky.ai";
const [dist, site] = process.argv.slice(2);
if (!dist) {
  console.error("usage: node test-seo.mjs DIST [SITE]");
  process.exit(2);
}

let failures = 0;
function fail(message) {
  console.error(`FAIL ${message}`);
  failures++;
}

// The schema.org types the builders in src/seo/jsonld.ts make, with the properties each needs (Google's
// required properties for its rich results, and what the builders always set). A node of a type not here
// fails at the top level; nested, it's only checked if it's here.
const REQUIRED = {
  WebSite: ["name", "url"],
  Organization: ["name"],
  VideoGame: ["name"],
  BreadcrumbList: ["itemListElement"],
  ListItem: ["position"],
  Article: ["headline"],
  ItemList: ["itemListElement"],
  VideoObject: ["name", "thumbnailUrl", "uploadDate", "contentUrl"],
  ImageObject: ["contentUrl"],
  FAQPage: ["mainEntity"],
  Question: ["name", "acceptedAnswer"],
  Answer: ["text"],
  SearchAction: ["target", "query-input"],
  EntryPoint: ["urlTemplate"],
  Person: ["name"],
  Thing: ["name"],
};
const TOP_LEVEL = [
  "WebSite",
  "Organization",
  "VideoGame",
  "BreadcrumbList",
  "Article",
  "ItemList",
  "VideoObject",
  "ImageObject",
  "FAQPage",
];

function checkNode(where, node, top) {
  if (typeof node !== "object" || node === null) return;
  // A script of several nodes is several top-level nodes.
  if (Array.isArray(node)) return node.forEach((item) => checkNode(where, item, top));
  const type = node["@type"];
  if (top) {
    if (node["@context"] !== "https://schema.org")
      fail(`${where}: @context ${node["@context"]}, not https://schema.org`);
    if (!TOP_LEVEL.includes(type)) fail(`${where}: @type ${JSON.stringify(type)} isn't one we make`);
  }
  for (const prop of REQUIRED[type] ?? []) {
    if (node[prop] === undefined || node[prop] === "") fail(`${where}: a ${type} without ${prop}`);
  }
  for (const value of Object.values(node)) checkNode(where, value, false);
}

// A head's tags, by a regex per tag: the pages are React's output, so attributes are always double-quoted,
// in the order the route gave them, and escaped (&amp; &quot; and so on).
function all(html, regex) {
  return [...html.matchAll(regex)].map((m) => unescapeHtml(m[1]));
}
// The entities React writes, and the numeric ones sitemap.ts's escapeXml does.
function unescapeHtml(text) {
  const named = { quot: '"', lt: "<", gt: ">", amp: "&", apos: "'" };
  return text.replace(/&(?:#x([0-9a-f]+)|#([0-9]+)|([a-z]+));/gi, (entity, hex, dec, name) => {
    if (hex) return String.fromCodePoint(parseInt(hex, 16));
    if (dec) return String.fromCodePoint(parseInt(dec, 10));
    return named[name] ?? entity;
  });
}
const meta = (html, attr, name) => all(html, new RegExp(`<meta ${attr}="${name}" content="([^"]*)"`, "g"));

function htmlFiles(dir) {
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".html"))
    .map((entry) => relative(dist, join(entry.parentPath, entry.name)).split(sep).join("/"));
}

// Where the asset store serves a file (wrangler.jsonc's html_handling, auto-trailing-slash): dir/index.html
// at /dir/, page.html at /page.
function servedPath(file) {
  if (file === "index.html") return "/";
  if (file.endsWith("/index.html")) return `/${file.slice(0, -"index.html".length)}`;
  return `/${file.slice(0, -".html".length)}`;
}

const pages = htmlFiles(dist).filter((file) => file !== "404.html" && !file.startsWith("play/"));
if (pages.length === 0) fail(`${dist}: no pre-rendered pages`);
const seen = { title: new Map(), description: new Map(), canonical: new Map() };
const canonicals = new Set();
// Pages with <meta name="robots" content="noindex">, which the sitemap mustn't list.
const unindexed = new Set();
for (const file of pages) {
  const before = failures;
  const html = readFileSync(join(dist, file), "utf8");
  const head = html.slice(0, html.indexOf("</head>"));
  const one = (what, values) => {
    if (values.length !== 1) fail(`${file}: ${values.length} ${what}s, not 1`);
    else if (!values[0].trim()) fail(`${file}: an empty ${what}`);
    else return values[0];
  };
  const title = one("title", all(head, /<title>([^<]*)<\/title>/g));
  const description = one("description", meta(head, "name", "description"));
  const canonical = one("canonical", all(head, /<link rel="canonical" href="([^"]*)"/g));
  for (const [what, value] of Object.entries({ title, description, canonical })) {
    if (value === undefined) continue;
    const other = seen[what].get(value);
    if (other) fail(`${file}: the same ${what} as ${other}: ${value}`);
    seen[what].set(value, file);
  }
  const want = SITE_URL + servedPath(file);
  const noindex = meta(head, "name", "robots").some((content) => content.split(/,\s*/).includes("noindex"));
  if (canonical !== undefined) {
    (noindex ? unindexed : canonicals).add(canonical);
    if (canonical !== want) fail(`${file}: canonical ${canonical}, but it's served at ${want}`);
  }
  // Open Graph and Twitter: each once, the URL the canonical one, the image absolute.
  for (const [attr, name] of [
    ["property", "og:title"],
    ["property", "og:description"],
    ["property", "og:url"],
    ["property", "og:image"],
    ["property", "og:type"],
    ["property", "og:site_name"],
    ["name", "twitter:card"],
    ["name", "twitter:title"],
    ["name", "twitter:description"],
    ["name", "twitter:image"],
  ]) {
    const values = meta(head, attr, name);
    if (values.length !== 1) fail(`${file}: ${values.length} ${name}, not 1`);
  }
  const [ogUrl] = meta(head, "property", "og:url");
  if (canonical !== undefined && ogUrl !== canonical) fail(`${file}: og:url ${ogUrl}, not the canonical ${canonical}`);
  for (const image of [...meta(head, "property", "og:image"), ...meta(head, "name", "twitter:image")]) {
    if (!image.startsWith("https://")) fail(`${file}: image ${image} isn't an absolute https URL`);
  }
  // The router escapes JSON-LD for a <script> (<, > and & as <...), which JSON.parse undoes itself.
  const scripts = [...head.matchAll(/<script type="application\/ld\+json">(.*?)<\/script>/gs)].map((m) => m[1]);
  scripts.forEach((text, i) => {
    let node;
    try {
      node = JSON.parse(text);
    } catch (error) {
      return fail(`${file}: JSON-LD ${i + 1} doesn't parse: ${error.message}`);
    }
    checkNode(`${file}: JSON-LD ${i + 1}`, node, true);
  });
  if (failures === before) console.log(`ok   ${servedPath(file)}: ${title} (${scripts.length} JSON-LD)`);
}

// The sitemap: every pre-rendered page, and only pages the site serves.
const sitemapFile = join(dist, "sitemap.xml");
const locs = existsSync(sitemapFile) ? all(readFileSync(sitemapFile, "utf8"), /<loc>([^<]*)<\/loc>/g) : [];
if (locs.length === 0) fail("sitemap.xml: missing, or lists no pages");
if (new Set(locs).size !== locs.length) fail("sitemap.xml: lists a page twice");
for (const canonical of canonicals) {
  if (!locs.includes(canonical)) fail(`sitemap.xml: doesn't list ${canonical}`);
}
for (const canonical of unindexed) {
  if (locs.includes(canonical)) fail(`sitemap.xml: lists ${canonical}, which is noindex`);
}
for (const loc of locs) {
  if (!loc.startsWith(`${SITE_URL}/`)) fail(`sitemap.xml: ${loc} isn't on ${SITE_URL}`);
  else if (!canonicals.has(loc)) {
    // Not a route's page: a file build.sh adds, like /play/.
    const path = loc.slice(SITE_URL.length);
    const file = path.endsWith("/") ? `${path}index.html` : `${path}.html`;
    if (!existsSync(join(dist, file))) fail(`sitemap.xml: ${loc} is no page in ${dist}`);
  }
}
// The media sitemaps: pages of sitemap.xml's, each with its media (src/seo/sitemap.ts). Each item needs
// these tags once, the URLs of `media` on /wiki/media/.
const MEDIA_SITEMAPS = {
  "sitemap-video.xml": {
    item: "video:video",
    required: ["video:title", "video:description", "video:duration", "video:publication_date"],
    media: ["video:thumbnail_loc", "video:content_loc"],
  },
  "sitemap-image.xml": { item: "image:image", required: [], media: ["image:loc"] },
};
const tagText = (xml, tag) => all(xml, new RegExp(`<${tag}>([^<]*)</${tag}>`, "g"));
const mediaLocs = [];
const listed = [];
for (const [name, { item, required, media }] of Object.entries(MEDIA_SITEMAPS)) {
  const file = join(dist, name);
  if (!existsSync(file)) {
    fail(`${name}: missing`);
    continue;
  }
  const urls = readFileSync(file, "utf8").split("<url>").slice(1);
  if (urls.length > 0) listed.push(name);
  const pageLocs = urls.map((url) => tagText(url, "loc")[0]);
  if (new Set(pageLocs).size !== pageLocs.length) fail(`${name}: lists a page twice`);
  for (const [i, url] of urls.entries()) {
    const loc = pageLocs[i];
    if (tagText(url, "loc").length !== 1) fail(`${name}: a <url> without one <loc>`);
    else if (!locs.includes(loc)) fail(`${name}: ${loc} isn't in sitemap.xml`);
    const items = url.split(`<${item}>`).slice(1);
    if (items.length === 0) fail(`${name}: ${loc} has no <${item}>`);
    for (const text of items) {
      for (const tag of [...required, ...media]) {
        const values = tagText(text, tag);
        if (values.length !== 1 || !values[0]) fail(`${name}: ${loc}: a <${item}> without one <${tag}>`);
      }
      for (const url of media.flatMap((tag) => tagText(text, tag))) {
        if (!url.startsWith(`${SITE_URL}/wiki/media/`)) fail(`${name}: ${url} isn't the wiki's media`);
        mediaLocs.push(url);
      }
    }
  }
  console.log(`ok   /${name}: ${urls.length} pages`);
}

if (site) {
  for (const loc of [...locs, ...mediaLocs]) {
    const url = site + loc.slice(SITE_URL.length);
    const response = await fetch(url, { redirect: "manual" }).catch((error) => error);
    if (!(response instanceof Response)) fail(`${url}: no response (${response.message})`);
    else if (response.status !== 200) fail(`${url}: status ${response.status}, not 200`);
    await response.body?.cancel?.();
  }
}

// robots.txt names sitemap.xml, and the media sitemaps if they list a page (an empty <urlset> isn't valid).
const robots = existsSync(join(dist, "robots.txt")) ? readFileSync(join(dist, "robots.txt"), "utf8") : "";
const named = robots
  .split("\n")
  .filter((line) => line.startsWith("Sitemap: "))
  .map((line) => line.slice("Sitemap: ".length));
const want = ["sitemap.xml", ...listed].map((name) => `${SITE_URL}/${name}`);
if (named.join(" ") !== want.join(" ")) fail(`robots.txt names ${named.join(", ")}, not ${want.join(", ")}`);

if (failures > 0) {
  console.error(`site/test-seo.mjs: ${failures} failed`);
  process.exit(1);
}
console.log(`site/test-seo.mjs: ${pages.length} pages and ${locs.length} sitemap URLs ok`);
