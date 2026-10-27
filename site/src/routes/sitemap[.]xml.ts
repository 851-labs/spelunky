import { createFileRoute } from "@tanstack/react-router";
import { sitemapXml } from "../seo/sitemap";

// /sitemap.xml, pre-rendered at build time like the pages (vite.config.ts), so the asset store serves it.
export const Route = createFileRoute("/sitemap.xml")({
  server: {
    handlers: {
      GET: () => new Response(sitemapXml(), { headers: { "content-type": "application/xml; charset=utf-8" } }),
    },
  },
});
