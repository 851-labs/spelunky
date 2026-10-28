import { createFileRoute, notFound } from "@tanstack/react-router";
import { seo } from "../../../seo/seo";
import { WikiFrame } from "../../../wiki/layout";
import { SourcePage } from "../../../wiki/source-view";

// A file of the game's GML, /wiki/source/objects/oSnake/step for gml/objects/oSnake/step.gml
// (src/wiki/source-url.ts): every one pre-rendered from the wiki media's gml/ tree (vite.config.ts lists them),
// none without. The page carries its file, which it hydrates with: its loader's data, as /wiki/source's
// carries the list. What it loads and its head are src/wiki/source.ts's.
export const Route = createFileRoute("/wiki/source/$")({
  loader: async ({ params }) => {
    const page = await (await import("../../../wiki/source")).sourcePage(params._splat ?? "");
    if (!page) throw notFound();
    return page;
  },
  head: ({ loaderData }) => (loaderData ? seo(loaderData.head) : {}),
  component: () => (
    <WikiFrame>
      <SourcePage file={Route.useLoaderData().file} />
    </WikiFrame>
  ),
});
