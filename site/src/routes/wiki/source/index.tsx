import { createFileRoute, notFound } from "@tanstack/react-router";
import { seo } from "../../../seo/seo";
import { WikiFrame } from "../../../wiki/layout";
import { SourceIndexView } from "../../../wiki/source-view";

// /wiki/source: every file of the source viewer, loaded as it pre-renders, so the page carries the list. None
// without the wiki media's gml/ tree. What it loads and its head are src/wiki/source.ts's.
export const Route = createFileRoute("/wiki/source/")({
  loader: async () => {
    const page = await (await import("../../../wiki/source")).sourceIndexPage();
    if (!page) throw notFound();
    return page;
  },
  head: ({ loaderData }) => (loaderData ? seo(loaderData.head) : {}),
  component: () => (
    <WikiFrame>
      <SourceIndexView index={Route.useLoaderData().index} />
    </WikiFrame>
  ),
});
