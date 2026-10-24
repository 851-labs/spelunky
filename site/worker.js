// Everything but /play/v/* (wrangler.jsonc's run_worker_first) comes through here, on spelunky.ai and on a
// pull request's preview URL alike (docs/web-deploy.md, "Previews").
const APEX = "spelunky.ai";

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.hostname === `www.${APEX}`) {
      url.hostname = APEX;
      return Response.redirect(url.toString(), 301);
    }
    // None of this is content-hashed (the pages, /play/manifest.txt, the landing page's media), so
    // browsers revalidate it every time: a deploy shows up at once, and an unchanged file is a 304.
    const assets = await env.ASSETS.fetch(request);
    const response = new Response(assets.body, assets);
    response.headers.set("cache-control", "no-cache");
    return response;
  },
};
