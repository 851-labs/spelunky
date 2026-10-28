// The landing page's scripts, which site/test.sh checks on the built site:
//
//   node test-bundle.mjs DIST
//
// DIST is the built static assets (dist/client). A page's up-front JS is what its HTML loads (its module
// scripts and modulepreloads) and every chunk those import statically, as the browser fetches them all
// before the page hydrates. The landing page's mustn't have the wiki's code: none of its layout
// (src/wiki/layout.tsx), and none of Base UI, which only the wiki uses. The route tree is in the landing
// page's entry chunk, so a route's loader or head that imports a component brings the wiki with it
// (src/wiki/hub-route.ts). And it must stay under BUDGET gzipped: React, the router and the routes'
// definitions. It prints every page's, as the wiki's are measured against it. No page's has the wiki's CSS,
// which is a stylesheet of its own (src/wiki/stylesheet.tsx): inlined from the wiki routes' heads, it was in
// the route tree, so every page's entry chunk. And no page's has the live clips' player (src/wiki/live-clip.tsx):
// it's a chunk the "Play live" button loads, as the game it embeds is /play/'s. And the landing page's names no
// wiki page's chunk: the map of them (src/wiki/pages.ts's loaders) grows with the wiki, so the page route's
// loader imports it when it runs rather than from the route tree (851-3566).
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, posix } from "node:path";
import { gzipSync } from "node:zlib";

// 117 KB when it was set (851-3428), from 123 KB with the wiki's CSS, and 168 KB when the landing page loaded
// the wiki's code too (851-3407). Raised to 122 KB (851-3533) when 146 pages took it to 120.5 KB, as the entry
// chunk had each wiki page's chunk path, and back to 117 KB (851-3566) when the map of them left it: 115.5 KB.
const BUDGET = 117 * 1024;
// Whatever quotes the minifier picks: WikiFrame's header (its className), and Base UI's production errors'
// URL, which its every component can throw.
const WIKI = /className:["'`]wiki-header["'`]/;
const BASE_UI = /base-ui\.com\/production-error/;
// A rule of wiki.css's, as a ?raw import would carry it, or a minifier.
const WIKI_CSS = /\.wiki-header ?\{/;
// What the live clips' player asks of the game (src/wiki/live-clip-player.tsx).
const LIVE_PLAYER = /spelunky:take-control/;
// A wiki page's chunk (src/wiki/build/compile.ts's module): its meta, which starts with its slug.
const PAGE_MODULE = /\bmeta["'`]?:\{["'`]?slug["'`]?:["'`][a-z0-9-]+["'`]/;
const PAGES = [
  "index.html",
  "wiki.html",
  "wiki/snake.html",
  "wiki/monsters.html",
  "wiki/items/weapons.html",
  "wiki/source/objects/oSnake/step.html",
];

const [dist] = process.argv.slice(2);
if (!dist) {
  console.error("usage: node test-bundle.mjs DIST");
  process.exit(2);
}

const assets = readdirSync(join(dist, "assets")).filter((file) => file.endsWith(".js"));
const pageChunks = assets.filter((file) => PAGE_MODULE.test(readFileSync(join(dist, "assets", file), "utf8")));
// The page chunks a chunk names: its loader map's, if it has it.
const names = (code) => pageChunks.filter((file) => code.includes(file));

let failures = 0;
function fail(message) {
  console.error(`FAIL ${message}`);
  failures++;
}

// A page's up-front chunks, as paths in DIST (assets/index-HASH.js), with their code.
function upFront(page) {
  const html = readFileSync(join(dist, page), "utf8");
  const todo = [...html.matchAll(/<(?:script|link)\b[^>]*\b(?:src|href)="\/(assets\/[^"]+\.js)"/g)].map((m) => m[1]);
  const chunks = new Map();
  while (todo.length > 0) {
    const chunk = todo.pop();
    if (chunks.has(chunk)) continue;
    const code = readFileSync(join(dist, chunk), "utf8");
    chunks.set(chunk, code);
    // Static imports and re-exports only (`import{a as b}from"./x.js"`, `import"./x.js"`): a dynamic import
    // is a chunk loaded later, if ever.
    const imports = /(?:^|[;}\s/])(?:import|export)\s*(?:[\w$*{},\s]*from\s*)?["'](\.\/[^"']+\.js)["']/g;
    for (const m of code.matchAll(imports)) todo.push(posix.join(dirname(chunk), m[1]));
  }
  return chunks;
}

const kb = (bytes) => `${(bytes / 1024).toFixed(1)} KB`;
// Whether some wiki page loads each marker up front: else the landing page's checks find nothing, whatever
// it loads.
const found = { wiki: false, baseUi: false, loaders: false };
for (const page of PAGES) {
  const chunks = upFront(page);
  const size = [...chunks.values()].reduce((sum, code) => sum + gzipSync(code, { level: 9 }).length, 0);
  const has = (marker) => [...chunks].filter(([, code]) => marker.test(code)).map(([chunk]) => chunk);
  console.log(`${page}: ${kb(size)} of JS gzipped up front, in ${chunks.size} chunks`);
  if (has(WIKI_CSS).length > 0) fail(`${page} loads the wiki's CSS as JS up front: ${has(WIKI_CSS).join(", ")}`);
  if (has(LIVE_PLAYER).length > 0) fail(`${page} loads the live clips' player up front: ${has(LIVE_PLAYER).join(", ")}`);
  if (page === "index.html") {
    if (has(WIKI).length > 0) fail(`${page} loads the wiki's layout up front: ${has(WIKI).join(", ")}`);
    if (has(BASE_UI).length > 0) fail(`${page} loads Base UI up front: ${has(BASE_UI).join(", ")}`);
    const loaders = [...chunks].filter(([, code]) => names(code).length > 0).map(([chunk]) => chunk);
    if (loaders.length > 0) fail(`${page} loads wiki pages' chunk paths up front: ${loaders.join(", ")}`);
    if (size > BUDGET) fail(`${page}: ${kb(size)} of JS up front, over its ${kb(BUDGET)} budget`);
  } else {
    found.wiki ||= has(WIKI).length > 0;
    found.baseUi ||= has(BASE_UI).length > 0;
    found.loaders ||= [...chunks.values()].some((code) => names(code).length > 0);
  }
}
if (!found.wiki) fail("no wiki page loads its layout up front, so WIKI here is out of date");
if (!found.baseUi) fail("no wiki page loads Base UI up front, so BASE_UI here is out of date");
if (pageChunks.length === 0) fail("no chunk is a wiki page's, so PAGE_MODULE here is out of date");
if (!found.loaders) fail("no wiki page loads the map of the pages' chunks up front, so the check of the landing page's is out of date");
if (!assets.some((file) => LIVE_PLAYER.test(readFileSync(join(dist, "assets", file), "utf8"))))
  fail("no chunk has the live clips' player, so LIVE_PLAYER here is out of date");

if (failures > 0) {
  console.error(`site/test-bundle.mjs: ${failures} failed`);
  process.exit(1);
}
console.log(`site/test-bundle.mjs: the landing page loads none of the wiki's code, within ${kb(BUDGET)}`);
