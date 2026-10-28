import { createFileRoute } from "@tanstack/react-router";
import { opensearchXml } from "../seo/opensearch";
import { OPENSEARCH_TYPE } from "../seo/seo";

// /opensearch.xml, pre-rendered like /robots.txt. The asset store serves it as application/xml, so the Worker
// gives it this type again (src/server.ts).
export const Route = createFileRoute("/opensearch.xml")({
  server: {
    handlers: {
      GET: () => new Response(opensearchXml(), { headers: { "content-type": OPENSEARCH_TYPE } }),
    },
  },
});
