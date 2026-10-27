// The SEO checks site/test.sh runs on the built site (docs/web-deploy.md, "SEO"):
//
//   node test-seo.mjs DIST [SITE]
//
// DIST is the built static assets (dist/client). Every pre-rendered page (its HTML files, but the web build
// at /play/ and the 404 page) must have one title, description and canonical URL, none shared with another
// page, its canonical must be where it's served, and its JSON-LD must parse, with @context and a known
// @type that has the properties search engines require of it. The sitemap must list exactly those pages
// (and /play/), but for those a robots meta tag keeps out of the index, and robots.txt point at it. The video and image sitemaps must list only pages the sitemap
// does, each with its media under /wiki/media/, and robots.txt name them if they list any. A page's
// VideoObjects and ImageObjects must be the media those sitemaps list under it, in order, and /wiki/snake
// must have both. Given SITE (a running server: wrangler dev), each of their URLs must be served there as
// is, a 200 and not a redirect.
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

// A page's clips and sprites, as Google's video and image rich results want them
// (https://developers.google.com/search/docs/appearance/structured-data/video): the media's URLs absolute,
// on the wiki's media, uploadDate an ISO 8601 date and time with its time zone, and the duration ISO 8601's
// whole seconds (media.ts's isoDuration).
const mediaUrls = [];
function checkMedia(file, nodes) {
  for (const node of nodes) {
    const type = node?.["@type"];
    if (type !== "VideoObject" && type !== "ImageObject") continue;
    const where = `${file}: ${type} ${node.name ?? node.contentUrl}`;
    const urls = type === "VideoObject" ? [node.contentUrl, node.thumbnailUrl] : [node.contentUrl];
    for (const url of urls) {
      if (typeof url !== "string" || !url.startsWith(`${SITE_URL}/wiki/media/`)) fail(`${where}: ${url} isn't the wiki's media`);
      else mediaUrls.push(url);
    }
    for (const prop of ["creditText", "copyrightNotice", "license"]) if (!node[prop]) fail(`${where}: no ${prop}`);
    if (type !== "VideoObject") continue;
    if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$/.test(node.uploadDate ?? ""))
      fail(`${where}: uploadDate ${node.uploadDate}, not an ISO 8601 date and time with a time zone`);
    if (!/^PT[1-9][0-9]*S$/.test(node.duration ?? "")) fail(`${where}: duration ${node.duration}, not PT<seconds>S`);
    if (!node.description) fail(`${where}: no description`);
  }
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
// Each page's top-level JSON-LD nodes, by its canonical URL.
const nodesOf = new Map();
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
  const nodes = [];
  scripts.forEach((text, i) => {
    let node;
    try {
      node = JSON.parse(text);
    } catch (error) {
      return fail(`${file}: JSON-LD ${i + 1} doesn't parse: ${error.message}`);
    }
    checkNode(`${file}: JSON-LD ${i + 1}`, node, true);
    nodes.push(...[node].flat());
  });
  checkMedia(file, nodes);
  if (canonical !== undefined) nodesOf.set(canonical, nodes);
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
// Each sitemap's media by page and tag: "sitemap-video.xml" -> loc -> "video:content_loc" -> [url, ...].
const sitemapMedia = {};
for (const [name, { item, required, media }] of Object.entries(MEDIA_SITEMAPS)) {
  const file = join(dist, name);
  if (!existsSync(file)) {
    fail(`${name}: missing`);
    continue;
  }
  const urls = readFileSync(file, "utf8").split("<url>").slice(1);
  if (urls.length > 0) listed.push(name);
  sitemapMedia[name] = new Map();
  const pageLocs = urls.map((url) => tagText(url, "loc")[0]);
  if (new Set(pageLocs).size !== pageLocs.length) fail(`${name}: lists a page twice`);
  for (const [i, url] of urls.entries()) {
    const loc = pageLocs[i];
    if (tagText(url, "loc").length !== 1) fail(`${name}: a <url> without one <loc>`);
    else if (!locs.includes(loc)) fail(`${name}: ${loc} isn't in sitemap.xml`);
    const items = url.split(`<${item}>`).slice(1);
    if (items.length === 0) fail(`${name}: ${loc} has no <${item}>`);
    sitemapMedia[name].set(loc, Object.fromEntries([...required, ...media].map((tag) => [tag, items.flatMap((text) => tagText(text, tag))])));
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

// A page's media JSON-LD says what the media sitemaps do of it (media.ts's builders make both), in the same
// order: the sitemaps leave out only what's on a page they don't list (noindex).
for (const [canonical, nodes] of nodesOf) {
  if (unindexed.has(canonical)) continue;
  const of = (type) => nodes.filter((node) => node?.["@type"] === type);
  const videos = of("VideoObject");
  const images = of("ImageObject");
  const inVideo = sitemapMedia["sitemap-video.xml"]?.get(canonical) ?? {};
  const inImage = sitemapMedia["sitemap-image.xml"]?.get(canonical) ?? {};
  const same = (what, ours, theirs = []) => {
    if (JSON.stringify(ours) !== JSON.stringify(theirs))
      fail(`${canonical}: its JSON-LD's ${what} are ${JSON.stringify(ours)}, the sitemap's ${JSON.stringify(theirs)}`);
  };
  same("videos", videos.map((v) => v.contentUrl), inVideo["video:content_loc"]);
  same("videos' thumbnails", videos.map((v) => v.thumbnailUrl), inVideo["video:thumbnail_loc"]);
  same("videos' names", videos.map((v) => v.name), inVideo["video:title"]);
  same("videos' upload dates", videos.map((v) => v.uploadDate), inVideo["video:publication_date"]);
  same("videos' durations", videos.map((v) => v.duration), inVideo["video:duration"]?.map((s) => `PT${s}S`));
  same("images", images.map((i) => i.contentUrl), inImage["image:loc"]);
}
// The Snake's page shows a clip the media have and its infobox's sprite, animated (site/test.sh's stand-in
// package has the one clip), so its head has their nodes, after the page's own.
const snake = nodesOf.get(`${SITE_URL}/wiki/snake`);
if (!snake) fail("wiki/snake.html: missing");
else {
  const before = failures;
  const types = [...new Set(snake.map((node) => node["@type"]))].join(", ");
  if (types !== "Article, BreadcrumbList, VideoObject, ImageObject") fail(`wiki/snake.html: JSON-LD of ${types}`);
  const sprite = snake.filter((node) => node["@type"] === "ImageObject").map((node) => node.contentUrl);
  if (sprite.length !== 1 || !sprite[0].endsWith("/sprites/sSnakeWalkL/anim%404x.png"))
    fail(`wiki/snake.html: ImageObjects of ${sprite.join(", ")}, not its infobox's animated sSnakeWalkL`);
  if (failures === before) console.log(`ok   /wiki/snake: ${types}`);
}

if (site) {
  for (const loc of [...locs, ...new Set([...mediaLocs, ...mediaUrls])]) {
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
