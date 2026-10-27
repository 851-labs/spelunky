// The Worker. Everything but the content-hashed files (/play/v/*, /assets/*: wrangler.jsonc's
// run_worker_first) comes through here, on spelunky.ai and on a pull request's preview URL alike
// (docs/web-deploy.md, "Previews").
import handler from "@tanstack/react-start/server-entry";

const APEX = "spelunky.ai";
// TanStack Start's default base for server functions' URLs.
const SERVER_FN_BASE = "/_serverFn/";

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
    if (url.pathname.startsWith(SERVER_FN_BASE)) return withDefaultCaching(await handler.fetch(request));
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

// Whether the router might have the page: a GET of a path without an extension. So a miss like /favicon.ico
// or /wp-login.php is only the asset store's 404, with no render.
function maybePage(request: Request, url: URL): boolean {
  return (request.method === "GET" || request.method === "HEAD") && !/\.[^/]*$/.test(url.pathname);
}

// None of the assets the Worker serves is content-hashed (the pages, /play/manifest.txt, the landing page's
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
