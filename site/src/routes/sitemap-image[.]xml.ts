import { createFileRoute } from "@tanstack/react-router";
import wikiMedia from "virtual:wiki-media";
import { imageSitemapXml } from "../seo/sitemap";

// /sitemap-image.xml, pre-rendered like /sitemap.xml: the wiki's sprites, by the pages that show them.
export const Route = createFileRoute("/sitemap-image.xml")({
  server: {
    handlers: {
      GET: () =>
        new Response(imageSitemapXml(wikiMedia), { headers: { "content-type": "application/xml; charset=utf-8" } }),
    },
  },
});
