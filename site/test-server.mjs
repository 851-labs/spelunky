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
import { readdirSync, readFileSync } from "node:fs";
import { registerHooks } from "node:module";

// Node runs the TypeScript itself (it strips the types), but the Worker imports "./seo/seo", as Vite resolves
// it, not "./seo/seo.ts". Every page is pre-rendered, so the router, which the Worker asks for a page that isn't
// a file, has none. And the wiki's names are the build's, whose redirects test.sh and test-links.mjs check on
// the built site: here there are a few pages, an alias and a hub (ALIASES).
// It counts its renders, which www's one-hop probe mustn't ask for.
const ROUTER = `export default { fetch: async () => {
  globalThis.routerRenders = (globalThis.routerRenders ?? 0) + 1;
  return new Response("router: no such page", { status: 404 });
} };`;
// Pages called jar (alias pot), snake, yeti-king and olmecs-lair, and a hub called monsters.
const ALIASES = `export const names = new Map(${JSON.stringify([["jar", "jar"], ["pot", "jar"], ["snake", "snake"], ["yeti-king", "yeti-king"], ["olmecs-lair", "olmecs-lair"], ["monsters", "monsters"]])});
export const twins = new Set(["jar", "snake", "yeti-king", "olmecs-lair"]);`;
// The build's date (vite.config.ts), which the Worker gives the pages.
const UPDATED = "Mon, 02 Nov 2026 12:00:00 GMT";
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "@tanstack/react-start/server-entry") {
      return { url: `data:text/javascript,${encodeURIComponent(ROUTER)}`, shortCircuit: true };
    }
    if (specifier === "virtual:site/updated") {
      const source = `export const updated = ${JSON.stringify(UPDATED)};`;
      return { url: `data:text/javascript,${encodeURIComponent(source)}`, shortCircuit: true };
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
  ".bin": "application/octet-stream",
  ".wav": "audio/wav",
};
const ETAG = '"stand-in"';
const BODY = "Spelunky is © 2008, 2009 Derek Yu";
const assets = {
  async fetch(request) {
    const { pathname } = new URL(request.url);
    const type =
      pathname === "/wrangler-dev.txt" ? "text/plain; charset=utf-8" : ASSET_TYPES[/\.[a-z]+$/.exec(pathname)?.[0]];
    // The 404 page at its path (html_handling), and the slash it drops from a page's path, with a 307, as
    // Cloudflare's does.
    if (pathname === "/404") return new Response("404 page", { headers: { "content-type": "text/html" } });
    if (pathname === "/wiki/monsters/") {
      return new Response(null, { status: 307, headers: { location: pathname.slice(0, -1) } });
    }
    // The 404 page gets the path's _headers too, as Cloudflare's does.
    const headers = new Headers({ "content-type": type ?? "text/html" });
    if (type !== undefined) headers.set("etag", ETAG);
    const given = new Headers();
    const removed = [];
    for (const rule of headerRules.filter((r) => r.path.test(pathname))) {
      for (const [name, value] of rule.headers) value === null ? removed.push(name) : given.append(name, value);
    }
    for (const [name, value] of given) headers.set(name, value);
    for (const name of removed) headers.delete(name);
    if (type === undefined) return new Response("404 page", { status: 404, headers });
    if (request.headers.get("if-none-match") === ETAG) return new Response(null, { status: 304, headers });
    return new Response(request.method === "HEAD" ? null : BODY, { headers });
  },
};
// A request to spelunky.ai, which the Worker sees or not as run_worker_first says.
const serve = (request) =>
  toWorker(new URL(request.url).pathname) ? worker.fetch(request, { ASSETS: assets }) : assets.fetch(request);

// The headers every response from spelunky.ai has: public/_headers' /* rule's, which the asset store gives, and the
// Worker (src/server.ts) the same, whatever the status.
const EVERY_RESPONSE = headerRules.find((r) => r.path.source === pattern("/*").source).headers;
for (const name of [
  "strict-transport-security",
  "content-security-policy",
  "x-content-type-options",
  "x-frame-options",
  "referrer-policy",
  "permissions-policy",
]) {
  assert.ok(EVERY_RESPONSE.some(([n]) => n.toLowerCase() === name), `public/_headers' /* has no ${name}`);
}
function assertEveryResponse(response, what) {
  for (const [name, value] of EVERY_RESPONSE) assert.equal(response.headers.get(name), value, `${what}: ${name}`);
}

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
  "/third-party-licenses.txt": UTF8_TEXT,
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
      assertEveryResponse(response, what);
      assert.equal(await response.text(), how === "GET" ? BODY : "", `${what}: body`);
      console.log(`ok   ${what}: ${type}`);
    } catch (error) {
      failures++;
      console.error(`FAIL ${error.message}`);
    }
  }
}
// The web build's text files (its third-party notices) skip the Worker, in UTF-8 as _headers says, and immutable.
{
  const path = "/play/v/0123456789abcdef/THIRD-PARTY-LICENSES.txt";
  try {
    assert.equal(toWorker(path), false, `${path}: reaches the Worker`);
    const response = await serve(new Request(`https://spelunky.ai${path}`));
    assert.equal(response.headers.get("content-type"), UTF8_TEXT, `${path}: content-type`);
    assert.equal(response.headers.get("cache-control"), "public, max-age=31536000, immutable", `${path}: cache-control`);
    console.log(`ok   ${path}: ${UTF8_TEXT}`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
// The media's files skip the Worker but the clips' videos; their URLs from before the media were committed, under
// a release's package's hash, come through it, and it redirects them to what's at the same path now.
const OLD = "/wiki/media/05d8cd2055735618";
// A web build's content-hashed files (scripts/web --package).
const GAME = "/play/v/0123456789abcdef";
const ROUTES = {
  [`${MEDIA}/sprites/sSnakeWalkL/frames/0%404x.png`]: false,
  [`${MEDIA}/sprites/index.json`]: false,
  [`${MEDIA}/clips/index.json`]: false,
  [`${MEDIA}/clips/snake/bite/poster.png`]: false,
  [`${MEDIA}/clips/snake/bite/replay.txt`]: false,
  [`${MEDIA}/LICENSE.txt`]: false,
  [`${GAME}/assets/bundle.bin`]: false,
  [`${GAME}/assets/config/bundle.bin`]: false,
  [`${GAME}/assets/sounds/jump.wav`]: false,
  ["/media/logo.png"]: false,
  ["/media/card.png"]: false,
  ["/media/gameplay.png"]: false,
  ["/media/gameplay.mp4"]: true,
  [`${MEDIA}/clips/snake/bite/clip.mp4`]: true,
  [`${MEDIA}/clips/snake/bite/clip.webm`]: true,
  [`${OLD}/sprites/sSnakeWalkL/frames/0%404x.png`]: true,
  [`${OLD}/clips/snake/bite/poster.png`]: true,
  [`${OLD}/clips/snake/bite/clip.mp4`]: true,
  [`${OLD}/NOTICE.txt`]: true,
};
// The game's assets and sounds skip the Worker (ROUTES, below), as a type Cloudflare compresses (public/_headers),
// which the asset store's .bin, application/octet-stream, and .wav, audio/wav, aren't, and with the rest of
// /play/v/'s headers, on a HEAD and a 304 too.
for (const path of [`${GAME}/assets/bundle.bin`, `${GAME}/assets/config/bundle.bin`, `${GAME}/assets/sounds/jump.wav`]) {
  for (const how of ["GET", "HEAD", "If-None-Match"]) {
    const what = `${how} ${path}`;
    try {
      const response = await serve(
        new Request(`https://spelunky.ai${path}`, {
          method: how === "HEAD" ? "HEAD" : "GET",
          headers: how === "If-None-Match" ? { "if-none-match": ETAG } : {},
        }),
      );
      assert.equal(response.status, how === "If-None-Match" ? 304 : 200, `${what}: status`);
      assert.equal(response.headers.get("content-type"), "application/x-protobuf", `${what}: content-type`);
      assert.equal(response.headers.get("cache-control"), "public, max-age=31536000, immutable", `${what}: cache-control`);
      assert.equal(response.headers.get("x-content-type-options"), "nosniff", `${what}: x-content-type-options`);
      console.log(`ok   ${what}: application/x-protobuf`);
    } catch (error) {
      failures++;
      console.error(`FAIL ${error.message}`);
    }
  }
}
for (const [path, worker] of Object.entries(ROUTES)) {
  try {
    assert.equal(toWorker(path), worker, `${path}: ${worker ? "doesn't reach" : "reaches"} the Worker`);
    if (path.startsWith(`${OLD}/`)) {
      const response = await serve(new Request(`https://spelunky.ai${path}`));
      assert.equal(response.status, 301, `${path}: status`);
      assert.equal(response.headers.get("location"), `https://spelunky.ai${MEDIA}${path.slice(OLD.length)}`, `${path}: location`);
      assertEveryResponse(response, path);
    }
    console.log(`ok   ${path}: ${worker ? "the Worker" : "the asset store"}`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
// An alias is a 301 to its page, or its twin, no-cache, in one hop, the query kept; but only for a GET or HEAD.
// So is a page's or hub's name in another case, with underscores or spaces, or with its apostrophe (851-3895).
const ALIAS_REDIRECTS = {
  "/wiki/pot": "/wiki/jar",
  "/wiki/pot/": "/wiki/jar",
  "/wiki/pot.md": "/wiki/jar.md",
  "/wiki/pot?a=1": "/wiki/jar?a=1",
  "/wiki/Snake": "/wiki/snake",
  "/wiki/SNAKE/": "/wiki/snake",
  "/wiki/Snake.md": "/wiki/snake.md",
  "/wiki/Yeti_King": "/wiki/yeti-king",
  "/wiki/Yeti%20King?a=1": "/wiki/yeti-king?a=1",
  "/wiki/Olmec%27s_Lair": "/wiki/olmecs-lair",
  "/wiki/Olmec's_Lair": "/wiki/olmecs-lair",
  "/wiki/Pot": "/wiki/jar",
  "/wiki/Monsters": "/wiki/monsters",
  "/wiki/Yeti_-_King": "/wiki/yeti-king",
  "/wiki/_Snake_": "/wiki/snake",
  "/wiki/Snake.MD": "/wiki/snake.md",
  "/wiki/snake.MD": "/wiki/snake.md",
  "/wiki/Sn%C3%A1ke": "/wiki/snake",
};
for (const [path, to] of Object.entries(ALIAS_REDIRECTS)) {
  for (const method of ["GET", "HEAD"]) {
    try {
      const response = await serve(new Request(`https://spelunky.ai${path}`, { method }));
      assert.equal(response.status, 301, `${method} ${path}: status`);
      assert.equal(response.headers.get("location"), `https://spelunky.ai${to}`, `${method} ${path}: location`);
      assert.equal(response.headers.get("cache-control"), "no-cache", `${method} ${path}: cache-control`);
      assertEveryResponse(response, `${method} ${path}`);
      console.log(`ok   ${method} ${path}: 301 to ${to}`);
    } catch (error) {
      failures++;
      console.error(`FAIL ${error.message}`);
    }
  }
}
for (const path of ["/wiki/pot", "/wiki/Snake"]) {
  try {
    const response = await serve(new Request(`https://spelunky.ai${path}`, { method: "POST" }));
    assert.notEqual(response.status, 301, `POST ${path}: redirected`);
    console.log(`ok   POST ${path}: ${response.status}, no redirect`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
// A page's own slug is the asset store's (which has it, or 404s), a hub has no twin, and a name that's no
// page's or hub's, or that doesn't decode, is a miss: none is redirected.
for (const path of ["/wiki/snake", "/wiki/snake.md", "/wiki/Monsters.md", "/wiki/Cobra", "/wiki/%E0%A4%A"]) {
  try {
    const response = await serve(new Request(`https://spelunky.ai${path}`));
    assert.notEqual(response.status, 301, `GET ${path}: redirected to ${response.headers.get("location")}`);
    console.log(`ok   GET ${path}: ${response.status}, no redirect`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
// Plain HTTP and www are a 301 to https://spelunky.ai, path and query kept, in one hop (a 308 for a POST, which
// keeps its method), whether or not the Worker would have served the path; the files that skip it are the zone's
// "Always Use HTTPS" setting's (docs/web-deploy.md, "HTTPS"), so here only the Worker's paths.
const HTTPS_REDIRECTS = {
  "http://spelunky.ai/": "https://spelunky.ai/",
  "http://spelunky.ai/play/?a=1": "https://spelunky.ai/play/?a=1",
  // (A GET or HEAD of an alias goes to its page in the same hop: below.)
  "http://spelunky.ai/wiki/pot": "https://spelunky.ai/wiki/pot",
  "http://www.spelunky.ai/wiki/snake": "https://spelunky.ai/wiki/snake",
  "https://www.spelunky.ai/wiki/snake?a=1": "https://spelunky.ai/wiki/snake?a=1",
  "https://www.spelunky.ai/wiki/media/clips/snake/bite/clip.mp4":
    "https://spelunky.ai/wiki/media/clips/snake/bite/clip.mp4",
};
for (const [from, to] of Object.entries(HTTPS_REDIRECTS)) {
  for (const method of from.endsWith("/wiki/pot") ? ["POST"] : ["GET", "HEAD", "POST"]) {
    try {
      const response = await worker.fetch(new Request(from, { method }), { ASSETS: assets });
      assert.equal(response.status, method === "POST" ? 308 : 301, `${method} ${from}: status`);
      assert.equal(response.headers.get("location"), to, `${method} ${from}: location`);
      assertEveryResponse(response, `${method} ${from}`);
      console.log(`ok   ${method} ${from}: ${response.status} to ${to}`);
    } catch (error) {
      failures++;
      console.error(`FAIL ${error.message}`);
    }
  }
}
// But only spelunky.ai's own names: not Vite's servers, on localhost, nor a preview, on workers.dev.
for (const url of ["http://localhost:5173/wiki/snake.md", "http://abc123-spelunky.example.workers.dev/wiki/snake.md"]) {
  try {
    const response = await worker.fetch(new Request(url), { ASSETS: assets });
    assert.equal(response.status, 200, `${url}: status`);
    console.log(`ok   ${url}: no redirect`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
// The Worker gives them itself, not only by passing on the asset store's: here, a store that gives none.
const bare = {
  async fetch(request) {
    const { pathname } = new URL(request.url);
    if (pathname !== "/wiki/snake.md") return new Response("404 page", { status: 404, headers: { "content-type": "text/html" } });
    return new Response(BODY, { headers: { "content-type": "text/markdown", etag: ETAG } });
  },
};
for (const [path, method] of [["/wiki/snake.md", "GET"], ["/nowhere", "GET"], ["/", "POST"], ["/wiki/pot", "GET"]]) {
  try {
    const response = await worker.fetch(new Request(`https://spelunky.ai${path}`, { method }), { ASSETS: bare });
    assertEveryResponse(response, `${method} ${path} (a bare asset store)`);
    console.log(`ok   ${method} ${path}: the Worker's own headers`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
// Every other response has them too: a redirect, a 404, the Worker's or the asset store's, a 405, and the files
// that skip the Worker, once each and not joined with another rule's.
for (const [path, method] of [
  ["https://www.spelunky.ai/wiki/snake", "GET"],
  ["/nowhere", "GET"],
  ["/wp-login.php", "GET"],
  ["/play/v/0123456789abcdef/nope.js", "GET"],
  ["/", "POST"],
  ["/assets/index-0123.js", "GET"],
  ["/play/v/0123456789abcdef/app.wasm", "GET"],
  ["/wiki/og/snake.0123456789abcdef.png", "GET"],
  [`${MEDIA}/sprites/sSnakeWalkL/frames/0%404x.png`, "GET"],
  [`${MEDIA}/LICENSE.txt`, "GET"],
]) {
  try {
    assertEveryResponse(await serve(new Request(new URL(path, "https://spelunky.ai"), { method })), `${method} ${path}`);
    console.log(`ok   ${method} ${path}: the headers every response has`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
// A page has the build's date, and a revalidation with it is a 304, with a 200's headers and no body: Cloudflare
// drops a page's ETag as it compresses it, so the asset store's can't be one. Not another date (an earlier build's,
// after a rollback, or a later one's), nor with an If-None-Match, which the asset store answers; and only a page.
const PAGE = "/wiki/snake.html";
for (const [what, headers, status] of [
  ["GET", {}, 200],
  ["the date", { "if-modified-since": UPDATED }, 304],
  ["another date", { "if-modified-since": "Sun, 01 Nov 2026 12:00:00 GMT" }, 200],
  ["a later date", { "if-modified-since": "Tue, 03 Nov 2026 12:00:00 GMT" }, 200],
  ["the date and an ETag", { "if-modified-since": UPDATED, "if-none-match": '"other"' }, 200],
]) {
  for (const method of ["GET", "HEAD"]) {
    const label = `${method} ${PAGE}, ${what}`;
    try {
      const response = await serve(new Request(`https://spelunky.ai${PAGE}`, { method, headers }));
      assert.equal(response.status, status, `${label}: status`);
      assert.equal(response.headers.get("last-modified"), UPDATED, `${label}: last-modified`);
      assert.equal(response.headers.get("cache-control"), "no-cache", `${label}: cache-control`);
      assert.equal(await response.text(), status === 200 && method === "GET" ? BODY : "", `${label}: body`);
      console.log(`ok   ${label}: ${status}`);
    } catch (error) {
      failures++;
      console.error(`FAIL ${error.message}`);
    }
  }
}
// The asset store's own 304 for a page (its If-None-Match matched) has the date too.
try {
  const response = await serve(new Request(`https://spelunky.ai${PAGE}`, { headers: { "if-none-match": ETAG } }));
  assert.equal(response.status, 304, `${PAGE}, its ETag: status`);
  assert.equal(response.headers.get("last-modified"), UPDATED, `${PAGE}, its ETag: last-modified`);
  console.log(`ok   ${PAGE}, its ETag: the asset store's 304, dated`);
} catch (error) {
  failures++;
  console.error(`FAIL ${error.message}`);
}
// Nor any other file the Worker serves: a text file's, a twin's, a 404.
for (const path of ["/robots.txt", "/play/manifest.txt", "/wiki/snake.md", "/nowhere"]) {
  try {
    const response = await serve(new Request(`https://spelunky.ai${path}`, { headers: { "if-modified-since": UPDATED } }));
    assert.notEqual(response.status, 304, `${path}: a 304 by date`);
    assert.equal(response.headers.get("last-modified"), null, `${path}: last-modified`);
    console.log(`ok   ${path}: no date, ${response.status}`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
// The Low batch (851-3888): the 404 page is a 404 at its own paths; the server functions' path is the 404 page;
// the asset store's slash redirects are permanent; the files for LLMs aren't for search engines; www goes where
// the apex would end up in one hop; and an old media URL with an @ goes to the escaped name.
const LOW = [
  ["https://spelunky.ai/404", 404, null],
  ["https://spelunky.ai/404.html", 404, null],
  ["https://spelunky.ai/_serverFn/", 404, null],
  ["https://spelunky.ai/wiki/monsters/", 301, "/wiki/monsters"],
  ["https://www.spelunky.ai/wiki/pot/", 301, "https://spelunky.ai/wiki/jar"],
  ["http://spelunky.ai/wiki/pot", 301, "https://spelunky.ai/wiki/jar"],
  ["https://www.spelunky.ai/wiki/monsters/", 301, "https://spelunky.ai/wiki/monsters"],
  ["https://www.spelunky.ai/wiki/snake.md", 301, "https://spelunky.ai/wiki/snake.md"],
  [`https://spelunky.ai${OLD}/sprites/sSnakeWalkL/anim@4x.png`, 301, `https://spelunky.ai${MEDIA}/sprites/sSnakeWalkL/anim%404x.png`],
  [`https://www.spelunky.ai${OLD}/sprites/sSnakeWalkL/anim@4x.png`, 301, `https://spelunky.ai${MEDIA}/sprites/sSnakeWalkL/anim%404x.png`],
  // A miss on www isn't rendered to learn it's no redirect (below).
  ["https://www.spelunky.ai/wiki/nonexistent", 301, "https://spelunky.ai/wiki/nonexistent"],
];
for (const [url, status, location] of LOW) {
  try {
    const renders = globalThis.routerRenders ?? 0;
    const response = await worker.fetch(new Request(url), { ASSETS: assets });
    if (url.startsWith("https://www.")) assert.equal(globalThis.routerRenders ?? 0, renders, `${url}: the router rendered a probe`);
    assert.equal(response.status, status, `${url}: status`);
    assert.equal(response.headers.get("location"), location, `${url}: location`);
    if (status === 404) assert.equal(await response.text(), "404 page", `${url}: body`);
    // Where an alias goes can change, so www's hop to it is asked again, as the apex's alias redirects are.
    if (url.includes("/wiki/pot")) assert.equal(response.headers.get("cache-control"), "no-cache", `${url}: cache-control`);
    console.log(`ok   ${url}: ${status}${location ? ` to ${location}` : ""}`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
// The Worker sends /_serverFn/ to the 404 page, as the site has no server functions: one would need that gone.
const serverFns = readdirSync("src", { recursive: true })
  .filter((file) => /\.tsx?$/.test(file) && readFileSync(`src/${file}`, "utf8").includes("createServerFn"));
try {
  assert.deepEqual(serverFns, [], "src/ has server functions, which src/server.ts sends to the 404 page");
  console.log("ok   no server functions, so /_serverFn/ is the 404 page");
} catch (error) {
  failures++;
  console.error(`FAIL ${error.message}`);
}
for (const path of ["/llms.txt", "/llms-full.txt"]) {
  try {
    const response = await serve(new Request(`https://spelunky.ai${path}`));
    assert.equal(response.headers.get("x-robots-tag"), "noindex", `${path}: x-robots-tag`);
    assert.equal(response.headers.get("content-type"), UTF8_TEXT, `${path}: content-type`);
    console.log(`ok   ${path}: noindex`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${error.message}`);
  }
}
if (failures > 0) {
  console.error(`test-server.mjs: ${failures} failed`);
  process.exit(1);
}
console.log("site/test-server.mjs: all passed");
