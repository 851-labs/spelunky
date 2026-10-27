// The Worker. Everything but the content-hashed files (/play/v/*, /assets/*, and /wiki/media/* but its clips:
// wrangler.jsonc's run_worker_first) comes through here, on spelunky.ai and on a pull request's preview URL alike
// (docs/web-deploy.md, "Previews").
import handler from "@tanstack/react-start/server-entry";
import { absoluteUrl } from "./seo/seo";

const APEX = "spelunky.ai";
// TanStack Start's default base for server functions' URLs.
const SERVER_FN_BASE = "/_serverFn/";
// Files a route makes (src/routes/sitemap[.]xml.ts and the like). Pre-rendered, so on spelunky.ai they're
// assets; the router makes them under `vite dev` and for the pre-render.
const ROUTED_FILES = new Set(["/sitemap.xml", "/sitemap-video.xml", "/sitemap-image.xml", "/robots.txt"]);
// The wiki's media (docs/web-deploy.md, "Wiki media"), and its clips' videos, the only media the asset store
// leaves to the Worker (wrangler.jsonc), for their byte ranges.
const MEDIA_BASE = "/wiki/media/";
// After MEDIA_BASE: HASH/clips/<id>/clip.mp4 and the like.
const CLIP_VIDEO = /^[^/]+\/clips\/.+\.(?:mp4|webm)$/;
// A wiki page's Markdown twin, /wiki/<slug>.md (src/wiki/build/markdown.ts), a static file.
const MARKDOWN_TWIN = /^\/wiki\/([a-z0-9-]+)\.md$/;

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
    // Anything else under /wiki/media/ is the asset store's as it has it, never the router's.
    if (url.pathname.startsWith(MEDIA_BASE)) {
      return CLIP_VIDEO.test(url.pathname.slice(MEDIA_BASE.length)) ? withRanges(request, env) : env.ASSETS.fetch(request);
    }
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
    const twin = MARKDOWN_TWIN.exec(url.pathname);
    // And its revalidations' 304s, which carry the headers a 200 would.
    if (twin && (asset.status === 200 || asset.status === 304)) return markdownTwin(asset, twin[1]);
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
// HTML5 Audio and Video Guide), so a clip's satisfiable single range is a 206 cut from the asset (RFC 9110,
// section 14), and anything else the plain 200 a server may always send instead. The asset's headers stay:
// _headers' immutable caching, its type and ETag.
async function withRanges(request: Request, env: Env): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") return env.ASSETS.fetch(request);
  const head = request.method === "HEAD";
  // A HEAD asks for the file too, for its size. The Worker does the ranges, so the asset store isn't asked for
  // one, whether or not it ignores them.
  const whole = new Request(request, { method: "GET" });
  whole.headers.delete("range");
  whole.headers.delete("if-range");
  const asset = await env.ASSETS.fetch(whole);
  // A 304 for an If-None-Match, a 404 for another package's clip: as the asset store has them. And an encoded
  // file (which a clip never is) can't be cut, so it's the plain 200 too.
  if (asset.status !== 200 || asset.headers.has("content-encoding")) {
    if (!head) return asset;
    await asset.body?.cancel();
    return new Response(null, asset);
  }
  // The binding's answer has no Content-Length (and an asset store's HEAD none either), so the file is read to
  // learn its size: a clip is a few hundred KB.
  const file = new Uint8Array(await asset.arrayBuffer());
  const range = requestedRange(request, asset.headers.get("etag"), file.byteLength);
  const headers = new Headers(asset.headers);
  headers.set("accept-ranges", "bytes");
  const body = range ? file.subarray(range.start, range.end + 1) : file;
  headers.set("content-length", String(body.byteLength));
  if (range) headers.set("content-range", `bytes ${range.start}-${range.end}/${file.byteLength}`);
  return new Response(head ? null : body, { status: range ? 206 : 200, headers });
}

// The range a GET or HEAD asks for, from first to last byte, or null for the whole file. That's when there's no
// Range, or one a server may ignore (RFC 9110, section 14.2): not bytes=FIRST-[LAST] or bytes=-SUFFIX, several
// ranges (a <video> asks for one at a time, and several would take a multipart answer), or none of the file
// (rather than a 416, which a browser playing a clip never asks for); or when If-Range isn't the file's ETag (a
// date never matches: the assets have no Last-Modified).
function requestedRange(request: Request, etag: string | null, size: number): { start: number; end: number } | null {
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
    return suffix === 0 || size === 0 ? null : { start: Math.max(0, size - suffix), end: size - 1 };
  }
  const start = Number(first);
  if (last !== "" && Number(last) < start) return null;
  if (start >= size) return null;
  return { start, end: last === "" ? size - 1 : Math.min(Number(last), size - 1) };
}

// A Markdown twin is the page's text again, so it names the page as the canonical URL (in a Link header, as it
// has no <head>), which search engines index rather than the twin. And its type is Markdown, with the charset
// a browser needs for its ©, whatever the asset store makes of .md.
function markdownTwin(asset: Response, slug: string): Response {
  const response = noCache(asset);
  response.headers.set("content-type", "text/markdown; charset=utf-8");
  response.headers.set("link", `<${absoluteUrl(`/wiki/${slug}`)}>; rel="canonical"`);
  return response;
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
