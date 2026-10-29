// The content types the Worker (src/server.ts) and _headers set, which site/test.sh can't check
// (docs/web-deploy.md, "Testing locally"):
//
//   node test-server.mjs
//
// wrangler dev's asset store isn't Cloudflare's: it serves .txt as text/plain; charset=utf-8, where spelunky.ai's
// serves bare text/plain, so test.sh's checks pass whether or not the Worker or _headers adds the charset. Here a
// request goes where wrangler.jsonc's run_worker_first sends it, the Worker or a stand-in asset store that
// answers as Cloudflare's does (public/_headers' headers on the files it has, its HEADs and 304s with a 200's
// headers), and every text file the site serves must come out in UTF-8, and the files with a type of their own
// with it.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { registerHooks } from "node:module";

// Node runs the TypeScript itself (it strips the types), but the Worker imports "./seo/seo", as Vite resolves
// it, not "./seo/seo.ts". Every page is pre-rendered, so the router, which the Worker asks for a page that isn't
// a file, has none. And the wiki's aliases are the build's, whose redirects test.sh and test-links.mjs check on
// the built site: here there's one.
const ROUTER = `export default { fetch: async () => new Response("router: no such page", { status: 404 }) };`;
const ALIASES = 'export const aliases = new Map([["pot", "jar"]]);';
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "@tanstack/react-start/server-entry") {
      return { url: `data:text/javascript,${encodeURIComponent(ROUTER)}`, shortCircuit: true };
    }
    if (specifier === "virtual:wiki/aliases") {
      return { url: `data:text/javascript,${encodeURIComponent(ALIASES)}`, shortCircuit: true };
    }
    if (/^\.\.?\//.test(specifier) && !/\.[a-z]+$/.test(specifier)) {
      try {
        return nextResolve(`${specifier}.ts`, context);
      } catch {}
    }
    return nextResolve(specifier, context);
  },
});
const { default: worker } = await import("./src/server.ts");

// A run_worker_first or _headers path pattern, whose * spans directories.
function pattern(glob) {
  return new RegExp(`^${glob.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replaceAll("*", ".*")}$`);
}
// wrangler.jsonc's run_worker_first: a path goes to the Worker if a rule matches it and no negative one does.
// (Its comments are whole lines, and none of its strings has a //.)
const config = JSON.parse(
  readFileSync("wrangler.jsonc", "utf8")
    .replace(/^\s*\/\/.*$/gm, "")
    .replace(/,(\s*[}\]])/g, "$1"),
);
const workerRules = config.assets.run_worker_first.map((rule) => ({
  negative: rule.startsWith("!"),
  path: pattern(rule.replace(/^!/, "")),
}));
const toWorker = (path) =>
  workerRules.some((r) => !r.negative && r.path.test(path)) && !workerRules.some((r) => r.negative && r.path.test(path));
// public/_headers: each path pattern's headers, which replace the asset store's own (a header two rules give is
// joined with a comma, as Cloudflare does), and the ones it removes ("! Name").
const headerRules = [];
for (const line of readFileSync("public/_headers", "utf8").split(/\r?\n/)) {
  if (line.trim() === "" || line.trim().startsWith("#")) continue;
  if (!/^\s/.test(line)) {
    headerRules.push({ path: pattern(line.trim()), headers: [] });
    continue;
  }
  const header = /^\s*(?:!\s*(\S+)|([^:]+):\s*(.*))$/.exec(line);
  assert.ok(header && headerRules.length > 0, `public/_headers: can't read '${line}'`);
  headerRules.at(-1).headers.push(header[1] ? [header[1], null] : [header[2].trim(), header[3]]);
}

// The types spelunky.ai's asset store gives (curl -I), and wrangler dev's .txt, which already has the charset.
const ASSET_TYPES = {
  ".txt": "text/plain",
  ".md": "text/markdown",
  ".xml": "application/xml",
  ".html": "text/html",
  ".json": "application/json",
};
const ETAG = '"stand-in"';
const BODY = "Spelunky is © 2008, 2009 Derek Yu";
const assets = {
  async fetch(request) {
    const { pathname } = new URL(request.url);
    const type =
      pathname === "/wrangler-dev.txt" ? "text/plain; charset=utf-8" : ASSET_TYPES[/\.[a-z]+$/.exec(pathname)?.[0]];
    if (type === undefined) return new Response("404 page", { status: 404, headers: { "content-type": "text/html" } });
    const headers = new Headers({ "content-type": type, etag: ETAG });
    const given = new Headers();
    const removed = [];
    for (const rule of headerRules.filter((r) => r.path.test(pathname))) {
      for (const [name, value] of rule.headers) value === null ? removed.push(name) : given.append(name, value);
    }
    for (const [name, value] of given) headers.set(name, value);
    for (const name of removed) headers.delete(name);
    if (request.headers.get("if-none-match") === ETAG) return new Response(null, { status: 304, headers });
    return new Response(request.method === "HEAD" ? null : BODY, { headers });
  },
};
// A request to spelunky.ai, which the Worker sees or not as run_worker_first says.
const serve = (request) =>
  toWorker(new URL(request.url).pathname) ? worker.fetch(request, { ASSETS: assets }) : assets.fetch(request);

const UTF8_TEXT = "text/plain; charset=utf-8";
// The wiki's media, cached for a day (_headers), where the Worker marks what it serves no-cache.
const MEDIA = "/wiki/media";
const MEDIA_CACHE = "public, max-age=86400";
const WANT = {
  "/LICENSE.txt": UTF8_TEXT,
  "/robots.txt": UTF8_TEXT,
  "/play/manifest.txt": UTF8_TEXT,
  "/llms.txt": UTF8_TEXT,
  "/llms-full.txt": UTF8_TEXT,
  // A charset the asset store gave is kept, not given twice.
  "/wrangler-dev.txt": UTF8_TEXT,
  "/wiki/snake.md": "text/markdown; charset=utf-8",
  "/opensearch.xml": "application/opensearchdescription+xml",
  // XML says its own encoding, UTF-8 unless it says otherwise, so the asset store's type stays.
  "/sitemap.xml": "application/xml",
  // The wiki media's text files skip the Worker, and get theirs from _headers, and its other files keep theirs.
  [`${MEDIA}/LICENSE.txt`]: UTF8_TEXT,
  [`${MEDIA}/NOTICE.txt`]: UTF8_TEXT,
  [`${MEDIA}/sprites/index.json`]: "application/json",
};

let failures = 0;
for (const [path, type] of Object.entries(WANT)) {
  for (const how of ["GET", "HEAD", "If-None-Match"]) {
    const revalidation = how === "If-None-Match";
    const request = new Request(`https://spelunky.ai${path}`, {
      method: how === "HEAD" ? "HEAD" : "GET",
      headers: revalidation ? { "if-none-match": ETAG } : {},
    });
    const response = await serve(request);
    const what = `${how} ${path}`;
    const caching = path.startsWith(`${MEDIA}/`) ? MEDIA_CACHE : "no-cache";
    try {
      assert.equal(response.status, revalidation ? 304 : 200, `${what}: status`);
      assert.equal(response.headers.get("content-type"), type, `${what}: content-type`);
      assert.equal(response.headers.get("cache-control"), caching, `${what}: cache-control`);
      assert.equal(await response.text(), how === "GET" ? BODY : "", `${what}: body`);
      console.log(`ok   ${what}: ${type}`);
    } catch (error) {
      failures++;
      console.error(`FAIL ${error.message}`);
    }
  }
}
// The media's files skip the Worker but the clips' videos; their URLs from before the media were committed, under
// a release's package's hash, come through it, and it redirects them to what's at the same path now.
const OLD = "/wiki/media/05d8cd2055735618";
const ROUTES = {
  [`${MEDIA}/sprites/sSnakeWalkL/frames/0%404x.png`]: false,
  [`${MEDIA}/sprites/index.json`]: false,
  [`${MEDIA}/clips/index.json`]: false,
  [`${MEDIA}/clips/snake/bite/poster.png`]: false,
  [`${MEDIA}/clips/snake/bite/replay.txt`]: false,
  [`${MEDIA}/LICENSE.txt`]: false,
  [`${MEDIA}/clips/snake/bite/clip.mp4`]: true,
  [`${MEDIA}/clips/snake/bite/clip.webm`]: true,
  [`${OLD}/sprites/sSnakeWalkL/frames/0%404x.png`]: true,
  [`${OLD}/clips/snake/bite/poster.png`]: true,
  [`${OLD}/clips/snake/bite/clip.mp4`]: true,
  [`${OLD}/NOTICE.txt`]: true,
};
for (const [path, worker] of Object.entries(ROUTES)) {
  try {
    assert.equal(toWorker(path), worker, `${path}: ${worker ? "doesn't reach" : "reaches"} the Worker`);
    if (path.startsWith(`${OLD}/`)) {
      const response = await serve(new Request(`https://spelunky.ai${path}`));
      assert.equal(response.status, 301, `${path}: status`);
      assert.equal(response.headers.get("location"), `https://spelunky.ai${MEDIA}${path.slice(OLD.length)}`, `${path}: location`);
    }
    console.log(`ok   ${path}: ${worker ? "the Worker" : "the asset store"}`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
// An alias is a 301 to its page, or its twin, no-cache, in one hop, the query kept; but only for a GET or HEAD.
const ALIAS_REDIRECTS = {
  "/wiki/pot": "/wiki/jar",
  "/wiki/pot/": "/wiki/jar",
  "/wiki/pot.md": "/wiki/jar.md",
  "/wiki/pot?a=1": "/wiki/jar?a=1",
};
for (const [path, to] of Object.entries(ALIAS_REDIRECTS)) {
  for (const method of ["GET", "HEAD"]) {
    try {
      const response = await serve(new Request(`https://spelunky.ai${path}`, { method }));
      assert.equal(response.status, 301, `${method} ${path}: status`);
      assert.equal(response.headers.get("location"), `https://spelunky.ai${to}`, `${method} ${path}: location`);
      assert.equal(response.headers.get("cache-control"), "no-cache", `${method} ${path}: cache-control`);
      console.log(`ok   ${method} ${path}: 301 to ${to}`);
    } catch (error) {
      failures++;
      console.error(`FAIL ${error.message}`);
    }
  }
}
try {
  const response = await serve(new Request("https://spelunky.ai/wiki/pot", { method: "POST" }));
  assert.notEqual(response.status, 301, "POST /wiki/pot: redirected");
  console.log(`ok   POST /wiki/pot: ${response.status}, no redirect`);
} catch (error) {
  failures++;
  console.error(`FAIL ${error.message}`);
}
if (failures > 0) {
  console.error(`test-server.mjs: ${failures} failed`);
  process.exit(1);
}
console.log("site/test-server.mjs: all passed");
