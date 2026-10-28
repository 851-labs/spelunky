// The content types the Worker (src/server.ts) sets itself, which site/test.sh can't check (docs/web-deploy.md,
// "Testing locally"):
//
//   node test-server.mjs
//
// wrangler dev's asset store isn't Cloudflare's: it serves .txt as text/plain; charset=utf-8, where spelunky.ai's
// serves bare text/plain, so test.sh's checks pass whether or not the Worker adds the charset. Here the Worker
// runs against a stand-in asset store that answers as Cloudflare's does, its HEADs and 304s with a 200's headers,
// and every text file it serves must come out in UTF-8, and the files with a type of their own with it.
import assert from "node:assert/strict";
import { registerHooks } from "node:module";

// Node runs the TypeScript itself (it strips the types), but the Worker imports "./seo/seo", as Vite resolves
// it, not "./seo/seo.ts". Every page is pre-rendered, so the router, which the Worker asks for a page that isn't
// a file, has none.
const ROUTER = `export default { fetch: async () => new Response("router: no such page", { status: 404 }) };`;
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "@tanstack/react-start/server-entry") {
      return { url: `data:text/javascript,${encodeURIComponent(ROUTER)}`, shortCircuit: true };
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

// The types spelunky.ai's asset store gives (curl -I), and wrangler dev's .txt, which already has the charset.
const ASSET_TYPES = {
  ".txt": "text/plain",
  ".md": "text/markdown",
  ".xml": "application/xml",
  ".html": "text/html",
};
const ETAG = '"stand-in"';
const BODY = "Spelunky is © 2008, 2009 Derek Yu";
const assets = {
  async fetch(request) {
    const { pathname } = new URL(request.url);
    const type =
      pathname === "/wrangler-dev.txt" ? "text/plain; charset=utf-8" : ASSET_TYPES[/\.[a-z]+$/.exec(pathname)?.[0]];
    if (type === undefined) return new Response("404 page", { status: 404, headers: { "content-type": "text/html" } });
    const headers = { "content-type": type, etag: ETAG };
    if (request.headers.get("if-none-match") === ETAG) return new Response(null, { status: 304, headers });
    return new Response(request.method === "HEAD" ? null : BODY, { headers });
  },
};

const UTF8_TEXT = "text/plain; charset=utf-8";
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
};

let failures = 0;
for (const [path, type] of Object.entries(WANT)) {
  for (const how of ["GET", "HEAD", "If-None-Match"]) {
    const revalidation = how === "If-None-Match";
    const request = new Request(`https://spelunky.ai${path}`, {
      method: how === "HEAD" ? "HEAD" : "GET",
      headers: revalidation ? { "if-none-match": ETAG } : {},
    });
    const response = await worker.fetch(request, { ASSETS: assets });
    const what = `${how} ${path}`;
    try {
      assert.equal(response.status, revalidation ? 304 : 200, `${what}: status`);
      assert.equal(response.headers.get("content-type"), type, `${what}: content-type`);
      assert.equal(response.headers.get("cache-control"), "no-cache", `${what}: cache-control`);
      assert.equal(await response.text(), how === "GET" ? BODY : "", `${what}: body`);
      console.log(`ok   ${what}: ${type}`);
    } catch (error) {
      failures++;
      console.error(`FAIL ${error.message}`);
    }
  }
}
if (failures > 0) {
  console.error(`test-server.mjs: ${failures} failed`);
  process.exit(1);
}
console.log("site/test-server.mjs: all passed");
