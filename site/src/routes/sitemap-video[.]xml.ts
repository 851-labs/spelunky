import { createFileRoute } from "@tanstack/react-router";
import wikiMedia from "virtual:wiki-media";
import { videoSitemapXml } from "../seo/sitemap";

// /sitemap-video.xml, pre-rendered like /sitemap.xml: the wiki's clips, by the pages that show them.
export const Route = createFileRoute("/sitemap-video.xml")({
  server: {
    handlers: {
      GET: () =>
        new Response(videoSitemapXml(wikiMedia), { headers: { "content-type": "application/xml; charset=utf-8" } }),
    },
  },
});
