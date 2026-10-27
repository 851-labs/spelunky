// The Worker. Everything but the content-hashed files (/play/v/*, /assets/*, and /wiki/media/* but its clips:
// wrangler.jsonc's run_worker_first) comes through here, on spelunky.ai and on a pull request's preview URL alike
// (docs/web-deploy.md, "Previews").
import handler from "@tanstack/react-start/server-entry";

const APEX = "spelunky.ai";
// TanStack Start's default base for server functions' URLs.
const SERVER_FN_BASE = "/_serverFn/";
// Files a route makes (src/routes/sitemap[.]xml.ts and the like). Pre-rendered, so on spelunky.ai they're
// assets; the router makes them under `vite dev` and for the pre-render.
const ROUTED_FILES = new Set(["/sitemap.xml", "/sitemap-video.xml", "/sitemap-image.xml", "/robots.txt"]);
// The wiki's media (docs/web-deploy.md, "Wiki media"). Only the clips come here, for their byte ranges.
const MEDIA_BASE = "/wiki/media/";

interface Env {
  ASSETS: { fetch(request: Request): Promise<Response> };
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (url.hostname === `www.${APEX}`) {
      url.hostname = APEX;
      return Response.redirect(url.toString(), 301);
    }
    if (url.pathname.startsWith(MEDIA_BASE)) return withRanges(request, env);
    if (url.pathname.startsWith(SERVER_FN_BASE)) return withDefaultCaching(await handler.fetch(request));
    // /sitemap.xml/ to /sitemap.xml, as the asset store sends /LICENSE.txt/ to /LICENSE.txt, rather than a second
    // copy rendered on every request. The pre-render asks for the slashed path first, and follows this.
    if (url.pathname.endsWith("/") && ROUTED_FILES.has(url.pathname.slice(0, -1))) {
      url.pathname = url.pathname.slice(0, -1);
      return noCache(Response.redirect(url.toString(), 307));
    }
    // Pages are pre-rendered (vite.config.ts), so on spelunky.ai they're static assets like the rest. A miss
    // is the 404 page (wrangler.jsonc's not_found_handling), and other methods than GET and HEAD a 405.
    const asset = await env.ASSETS.fetch(request);
    if (asset.status !== 404 || !maybePage(request, url)) return noCache(asset);
    // The router renders a page that isn't a file: at build time to pre-render it, under `vite dev`, and on
    // spelunky.ai only one that opts out of pre-rendering.
    const rendered = await handler.fetch(request);
    if (rendered.status === 404) {
      await rendered.body?.cancel();
      return noCache(asset);
    }
    await asset.body?.cancel();
    return withDefaultCaching(rendered);
  },
};

// Whether the router might have the page: a GET of a path without an extension, or of a file a route makes.
// So a miss like /favicon.ico or /wp-login.php is only the asset store's 404, with no render.
function maybePage(request: Request, url: URL): boolean {
  return (
    (request.method === "GET" || request.method === "HEAD") &&
    (!/\.[^/]*$/.test(url.pathname) || ROUTED_FILES.has(url.pathname))
  );
}

// Workers static assets ignore Range: a request for bytes=0-1 gets a 200 with the whole file. Safari asks for
// ranges to play a video, and Apple requires a server to answer them for iOS ("Configuring Your Server", Safari
// HTML5 Audio and Video Guide), so the clips' single ranges are 206s sliced from the asset (RFC 9110, section
// 14). The asset's headers stay: _headers' immutable caching, its type and ETag. A clip is a few hundred KB,
// so a range is cut from it in memory; the whole file streams through.
async function withRanges(request: Request, env: Env): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") return env.ASSETS.fetch(request);
  const head = request.method === "HEAD";
  // The asset store's answer to a HEAD has no Content-Length, so a HEAD asks for the file, and drops it.
  const asset = await env.ASSETS.fetch(new Request(request, { method: "GET" }));
  // A 304 for an If-None-Match, a 404 for another package's clip: as the asset store has them.
  if (asset.status !== 200) {
    if (!head) return asset;
    await asset.body?.cancel();
    return new Response(null, asset);
  }
  // The asset store sends a GET's length; were it not to, the file would be read to learn it.
  const length = asset.headers.get("content-length");
  const file = length === null ? await asset.arrayBuffer() : null;
  const size = file?.byteLength ?? Number(length);
  const range = requestedRange(request, asset.headers.get("etag"), size);
  // A 416 and a HEAD send none of it.
  if ((range === "unsatisfiable" || head) && file === null) await asset.body?.cancel();
  if (range === "unsatisfiable") {
    // An error, not the clip, so none of its headers: neither its ETag nor its caching.
    return new Response(null, {
      status: 416,
      headers: { "accept-ranges": "bytes", "content-range": `bytes */${size}`, "cache-control": "no-store" },
    });
  }
  const headers = new Headers(asset.headers);
  headers.set("accept-ranges", "bytes");
  headers.set("content-length", String(range ? range.end - range.start + 1 : size));
  if (range) headers.set("content-range", `bytes ${range.start}-${range.end}/${size}`);
  const status = range ? 206 : 200;
  if (head) return new Response(null, { status, headers });
  if (!range) return new Response(file ?? asset.body, { status, headers });
  const bytes = file ?? (await asset.arrayBuffer());
  return new Response(new Uint8Array(bytes, range.start, range.end - range.start + 1), { status, headers });
}

// The range a GET or HEAD asks for, from first to last byte; "unsatisfiable" if it starts past the end; or null
// for the whole file. That's when there's no Range, or one a server ignores (RFC 9110, section 14.2): not
// bytes=FIRST-[LAST] or bytes=-SUFFIX, or several ranges (a <video> asks for one at a time, and several would
// take a multipart answer); or when If-Range isn't the file's ETag (a date never matches: the assets have no
// Last-Modified).
function requestedRange(
  request: Request,
  etag: string | null,
  size: number,
): { start: number; end: number } | "unsatisfiable" | null {
  const header = request.headers.get("range");
  if (header === null) return null;
  const ifRange = request.headers.get("if-range");
  // If-Range compares strongly, so a weak ETag never matches.
  if (ifRange !== null && (etag === null || etag.startsWith("W/") || ifRange.trim() !== etag)) return null;
  const match = /^bytes=(\d*)-(\d*)$/i.exec(header);
  if (match === null) return null;
  const [, first, last] = match;
  if (first === "") {
    if (last === "") return null;
    const suffix = Number(last);
    return suffix === 0 || size === 0 ? "unsatisfiable" : { start: Math.max(0, size - suffix), end: size - 1 };
  }
  const start = Number(first);
  if (last !== "" && Number(last) < start) return null;
  if (start >= size) return "unsatisfiable";
  return { start, end: last === "" ? size - 1 : Math.min(Number(last), size - 1) };
}

// None of the other assets the Worker serves is content-hashed (the pages, /play/manifest.txt, the landing page's
// media), so browsers revalidate them every time: a deploy shows up at once, and an unchanged file is a 304.
function noCache(response: Response): Response {
  const copy = new Response(response.body, response);
  copy.headers.set("cache-control", "no-cache");
  return copy;
}

// The same for what the router renders, unless a route says otherwise.
function withDefaultCaching(response: Response): Response {
  return response.headers.has("cache-control") ? response : noCache(response);
}
