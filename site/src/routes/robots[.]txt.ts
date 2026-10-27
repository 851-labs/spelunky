import { createFileRoute } from "@tanstack/react-router";
import wikiMedia from "virtual:wiki-media";
import { robotsTxt } from "../seo/sitemap";

// /robots.txt, pre-rendered like /sitemap.xml, so that it names only the sitemaps with pages in them.
export const Route = createFileRoute("/robots.txt")({
  server: {
    handlers: {
      GET: () => new Response(robotsTxt(wikiMedia), { headers: { "content-type": "text/plain; charset=utf-8" } }),
    },
  },
});
