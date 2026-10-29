// The wiki's links and what's left to write, which site/test.sh checks on the built site (851-3641's audit):
//
//   node test-links.mjs DIST [SITE]
//
// DIST is the built static assets (dist/client). Every link, script, image and video a page names on the site
// must be a file DIST has (a page, its Markdown twin, an asset, a sprite or clip of the media), as it's named:
// not a path the Worker only redirects (a trailing slash, 307, or an alias), and a #fragment one of the
// target's ids. Every wiki page must be reachable from /wiki through the pages' links, so none is an orphan
// that only search or the sitemap finds. And the wiki is complete: no hub shows a page to come and no page a
// red link, but for those TO_COME names with the ticket that's writing them, and wiki-planned.yaml plans
// nothing else, so a page can't be planned without someone to write it (851-3638's five had none, and were
// missed). A wiki page or Markdown twin that names one of our tickets (851-NNNN) anywhere, its text,
// attributes or JSON-LD, fails too: that's bookkeeping, not for a reader.
//
// The pages' aliases (their frontmatter's) are the Worker's redirects (src/server.ts): none is a file DIST has,
// each one's page is, and nothing on the site names one rather than its page: a link, a canonical URL, a
// sitemap's <loc>, JSON-LD or a Markdown file. Given SITE, the served site (test.sh's wrangler dev, or a
// preview), each alias, with a slash and as a Markdown twin, must be a 301 to its page (or twin) on SITE,
// which is a 200: one hop.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { parse as parseYaml } from "yaml";

const SITE_URL = "https://spelunky.ai";
// The pages still to come, each with its title (its hub card's) and the ticket writing it. Remove a page in
// the PR that writes it; the check fails on one that's written, or that nothing shows any more.
// None: every planned page is written (851-3638).
/** @type {Map<string, { title: string, ticket: string }>} */
const TO_COME = new Map([]);

const [dist, site] = process.argv.slice(2);
if (!dist) {
  console.error("usage: node test-links.mjs DIST [SITE]");
  process.exit(2);
}

let failures = 0;
function fail(message) {
  console.error(`FAIL ${message}`);
  failures++;
}

const walk = (dir) =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
// The pre-rendered pages, as paths in DIST with / separators: not the web build's (play/), nor the 404 page.
const files = new Set(walk(dist).map((path) => relative(dist, path).split(sep).join("/")));
const pages = [...files].filter((file) => file.endsWith(".html") && !file.startsWith("play/") && file !== "404.html");
const html = new Map(pages.map((file) => [file, readFileSync(join(dist, file), "utf8")]));
// The wiki's pages and hubs, but the source viewer's (noindex, and linked only from citations).
const wiki = pages.filter((file) => /^wiki(\.html$|\/)/.test(file) && !file.startsWith("wiki/source"));
const isWiki = new Set(wiki);
const urlOf = (file) =>
  file === "index.html" ? "/" : `/${file.replace(/(^|\/)index\.html$/, "$1").replace(/\.html$/, "")}`;

// The file a path on the site is, as the asset store serves it without a redirect, or null. A page is its
// path's .html (/wiki/snake, /wiki), a directory's its index.html with the slash (/play/), anything else
// the file itself, its name unescaped (a sprite's 0%404x.png).
function fileAt(path) {
  const name = decoded(path)?.slice(1);
  if (name === undefined) return null;
  if (name === "" || name.endsWith("/")) return files.has(`${name}index.html`) ? `${name}index.html` : null;
  if (files.has(`${name}.html`)) return `${name}.html`;
  return files.has(name) ? name : null;
}
// A URL's part unescaped, or undefined for a malformed escape (a stray %), which is no file and no id.
function decoded(part) {
  try {
    return decodeURIComponent(part);
  } catch {
    return undefined;
  }
}
// An attribute's or text's value, its HTML entities unescaped as React writes them (Kali&#x27;s Altar).
const ENTITIES = { amp: "&", lt: "<", gt: ">", quot: '"' };
const unescaped = (text) =>
  text.replace(/&(?:#x([0-9a-f]+)|#(\d+)|(amp|lt|gt|quot));/gi, (_, hex, dec, name) => {
    if (hex) return String.fromCodePoint(parseInt(hex, 16));
    return dec ? String.fromCodePoint(Number(dec)) : ENTITIES[name.toLowerCase()];
  });

const idsOf = new Map();
const ids = (file) => {
  if (!idsOf.has(file)) {
    const text = html.get(file) ?? readFileSync(join(dist, file), "utf8");
    idsOf.set(file, new Set([...text.matchAll(/\sid="([^"]+)"/g)].map((m) => unescaped(m[1]))));
  }
  return idsOf.get(file);
};
// Each page's aliases, to its slug, from its frontmatter, as compile.ts reads it.
const contentDir = new URL("content/wiki/", import.meta.url);
const aliases = new Map(
  readdirSync(contentDir)
    .filter((name) => name.endsWith(".mdx"))
    .flatMap((name) => {
      const yaml = /^---\r?\n([\s\S]*?)\r?\n---/.exec(readFileSync(new URL(name, contentDir), "utf8"));
      const page = yaml ? parseYaml(yaml[1]) : {};
      return (page.aliases ?? []).map((alias) => [alias, page.slug]);
    }),
);
// The page an alias's path is (/wiki/pot, /wiki/pot/, /wiki/pot.md), or undefined.
const aliasPage = (path) => aliases.get(/^\/wiki\/([a-z0-9-]+)(?:\/|\.md)?$/.exec(path)?.[1]);

// Each wiki page's links to other wiki pages, for the orphans below.
const links = new Map();
let checked = 0;
for (const [file, text] of html) {
  const here = urlOf(file);
  links.set(file, new Set());
  for (const m of text.matchAll(/<(a|link|img|source|video|script|iframe)\b[^>]*?\s(href|src|poster)="([^"]*)"/g)) {
    const [tag, value] = [m[1], unescaped(m[3])];
    let url;
    if (value.startsWith(`${SITE_URL}/`)) url = value.slice(SITE_URL.length);
    else if (value.startsWith("/") && !value.startsWith("//")) url = value;
    else if (value.startsWith("#")) url = here + value;
    else continue;
    checked++;
    const [beforeHash, fragment] = url.split("#");
    const path = beforeHash.split("?")[0];
    const target = fileAt(path);
    if (target === null) {
      // A slash on the end of a page's path is the Worker's redirect to the path without it.
      if (path.endsWith("/") && fileAt(path.slice(0, -1)) !== null) {
        fail(`${here}: ${value} redirects (drop the slash)`);
      } else if (aliasPage(path)) {
        fail(`${here}: ${value} is an alias, which redirects (link /wiki/${aliasPage(path)})`);
      } else fail(`${here}: ${value} isn't on the site`);
      continue;
    }
    // The source viewer's #L3-L4 is lines, which its script highlights, not an element's id.
    const lines = /^L\d/.test(fragment);
    if (fragment && target.endsWith(".html") && !lines && !ids(target).has(decoded(fragment)))
      fail(`${here}: ${value}: ${urlOf(target)} has no #${fragment}`);
    if (tag === "a" && target !== file && isWiki.has(target)) links.get(file).add(target);
  }
}

// Every wiki page, reachable from /wiki through the wiki's links.
const reached = new Set(["wiki.html"]);
const todo = ["wiki.html"];
while (todo.length > 0) {
  for (const target of links.get(todo.pop()) ?? []) {
    if (!reached.has(target)) todo.push(target), reached.add(target);
  }
}
for (const file of wiki) {
  if (reached.has(file)) continue;
  const from = wiki.filter((page) => links.get(page).has(file)).map(urlOf);
  const why = from.length > 0 ? ` (only ${from.join(", ")} link it)` : ": no page links it";
  fail(`${urlOf(file)} isn't reachable from /wiki${why}`);
}

// The pages to come: the hubs' cards (by title) and the pages' red links (by slug), as src/wiki/hub.tsx and
// src/wiki/components.tsx's WikiLink mark them. If they stop, every page would pass as written.
const CARD = /<li class="card card-unwritten">[\s\S]*?<span class="card-title">([^<]*)<\/span>/g;
const RED_LINK = /<span\b[^>]*\bclass="wikilink-new"[^>]*>/g;
const source = (file) => readFileSync(new URL(file, import.meta.url), "utf8");
if (!source("src/wiki/hub.tsx").includes('"card card-unwritten"'))
  fail("src/wiki/hub.tsx marks no card card-unwritten, so CARD here is out of date");
if (!source("src/wiki/components.tsx").includes('className="wikilink-new"'))
  fail("src/wiki/components.tsx marks no red link wikilink-new, so RED_LINK here is out of date");
const cards = new Map();
const redLinks = new Map();
for (const file of wiki) {
  const text = html.get(file);
  const where = urlOf(file);
  for (const [, card] of text.matchAll(CARD)) {
    const title = unescaped(card);
    cards.set(title, [...(cards.get(title) ?? []), where]);
  }
  for (const [span] of text.matchAll(RED_LINK)) {
    const slug = unescaped(/\sdata-slug="([^"]+)"/.exec(span)?.[1] ?? "(no data-slug)");
    redLinks.set(slug, [...new Set([...(redLinks.get(slug) ?? []), where])]);
  }
}
const titles = new Map([...TO_COME].map(([slug, { title }]) => [title, slug]));
for (const [title, where] of cards) {
  if (!titles.has(title)) fail(`${where.join(", ")}: "${title}" is to come, and TO_COME has no ticket writing it`);
}
for (const [slug, where] of redLinks) {
  if (!TO_COME.has(slug)) fail(`${where.join(", ")}: a red link to ${slug}, and TO_COME has no ticket writing it`);
}
for (const [, slug] of source("content/wiki-planned.yaml").matchAll(/^([a-z0-9-]+):/gm)) {
  if (!TO_COME.has(slug)) fail(`content/wiki-planned.yaml plans ${slug}, and TO_COME has no ticket writing it`);
}
for (const [slug, { title, ticket }] of TO_COME) {
  if (html.has(`wiki/${slug}.html`)) fail(`/wiki/${slug} is written: remove it from TO_COME (${ticket})`);
  else if (!cards.has(title) && !redLinks.has(slug))
    fail(`nothing shows ${slug} as to come: remove it from TO_COME (${ticket})`);
}

// No wiki page or twin names one of our tickets: four digits, not after a letter, digit or : (a GML line
// range, objects/oPlayer1/step.gml:1851-1882, or the source viewer's #L1851-L1882), nor 851-labs. Its
// stylesheet aside, as a tooltip, alt text or JSON-LD is read too.
for (const file of wiki) {
  const twin = file.replace(/\.html$/, ".md");
  for (const [name, text] of [
    [file, html.get(file).replace(/<style\b[\s\S]*?<\/style>/g, "")],
    ...(files.has(twin) ? [[twin, readFileSync(join(dist, twin), "utf8")]] : []),
  ]) {
    const ticket = /(?<![\w.:-])851-\d{4}\b/.exec(text);
    if (ticket) fail(`${name} names ticket ${ticket[0]}`);
  }
}

// The aliases: each one's page is in DIST and the alias isn't, and no page, Markdown file or sitemap names an
// alias's URL, in its head, JSON-LD or anywhere else (the links above fail on its path alone).
for (const [alias, slug] of aliases) {
  if (!files.has(`wiki/${slug}.html`)) fail(`alias ${alias}: its page /wiki/${slug} isn't in ${dist}`);
  for (const file of [`wiki/${alias}.html`, `wiki/${alias}.md`, `wiki/${alias}/index.html`]) {
    if (files.has(file)) fail(`alias ${alias} (of ${slug}) is ${file}, which it would hide`);
  }
}
// Absolute or from the root (a JSON-LD url, a Markdown link), not after another host.
const ALIAS_URL = new RegExp(`(?<![\\w./-])(?:${SITE_URL.replaceAll(".", "\\.")})?/wiki/([a-z0-9-]+)(?![a-z0-9-])`, "g");
const sitemaps = [...files].filter((file) => /^sitemap[\w-]*\.xml$/.test(file));
if (sitemaps.length === 0) fail(`no sitemap in ${dist}`);
const markdown = [...files].filter((file) => /^wiki\/[^/]+\.md$|^llms(-full)?\.txt$/.test(file));
for (const file of [...pages, ...markdown, ...sitemaps]) {
  const text = html.get(file) ?? readFileSync(join(dist, file), "utf8");
  for (const [url, name] of text.matchAll(ALIAS_URL)) {
    if (aliases.has(name)) fail(`${file} names ${url}, an alias: name ${SITE_URL}/wiki/${aliases.get(name)}`);
  }
}

// Each alias, on the served site: a 301 to its page, which is there. Eight requests at a time, not all of
// them (three for each alias) at once of wrangler dev or a preview.
let redirects = 0;
if (site) {
  const origin = new URL(site).origin;
  const forms = [...aliases].flatMap(([alias, slug]) => [
    [`/wiki/${alias}`, `/wiki/${slug}`],
    [`/wiki/${alias}/`, `/wiki/${slug}`],
    [`/wiki/${alias}.md`, `/wiki/${slug}.md`],
  ]);
  const ask = (path) => fetch(new URL(path, origin), { method: "HEAD", redirect: "manual" });
  // Each page's status, asked once however many aliases it has.
  const statuses = new Map();
  const statusOf = (path) => {
    if (!statuses.has(path)) statuses.set(path, ask(path).then((response) => response.status));
    return statuses.get(path);
  };
  const next = forms.entries();
  await Promise.all(
    Array.from({ length: 8 }, async () => {
      for (const [, [from, to]] of next) {
        const response = await ask(from);
        const location = response.headers.get("location");
        if (response.status !== 301) fail(`${from}: status ${response.status}, not 301 to ${to}`);
        else if (location === null || new URL(location, origin).href !== new URL(to, origin).href)
          fail(`${from}: redirects to ${location}, not ${to}`);
        else {
          const status = await statusOf(to);
          if (status !== 200) fail(`${from}: redirects to ${to}, whose status is ${status}, not 200`);
          else redirects++;
        }
      }
    }),
  );
  // And a path that's neither a page nor an alias is still the 404 page.
  const none = await ask("/wiki/no-such-page-or-alias");
  if (none.status !== 404) fail(`/wiki/no-such-page-or-alias: status ${none.status}, not 404`);
}

if (aliases.size < 2) fail(`only ${aliases.size} aliases: is content/wiki the wiki's pages?`);
if (wiki.length < 2 || checked === 0)
  fail(`only ${wiki.length} wiki pages and ${checked} links: is ${dist} the built site?`);
if (failures > 0) {
  console.error(`site/test-links.mjs: ${failures} failed`);
  process.exit(1);
}
const toCome = [...TO_COME].map(([slug, { ticket }]) => `${slug} (${ticket})`);
const left = toCome.length > 0 ? `and to come only ${toCome.join(", ")}` : "and none to come";
console.log(
  `site/test-links.mjs: ${checked} links on ${pages.length} pages all on the site, ` +
    `every wiki page reachable from /wiki, ${left}; ${aliases.size} aliases named nowhere` +
    (site ? `, and each redirects to its page on ${site} (${redirects} paths)` : ""),
);
