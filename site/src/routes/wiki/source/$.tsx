import { createFileRoute, notFound } from "@tanstack/react-router";
import { seo } from "../../../seo/seo";
import { WikiFrame } from "../../../wiki/layout";
import { RoomPage, SourcePage } from "../../../wiki/source-view";

// A file of the game's GML, /wiki/source/objects/oSnake/step for gml/objects/oSnake/step.gml
// (src/wiki/source-url.ts), or a room, /wiki/source/rooms/rOlmec: every one pre-rendered from the wiki media's
// gml/ tree and rooms/ (vite.config.ts lists them), none without. The page carries its file or room, which it
// hydrates with: its loader's data, as /wiki/source's carries the list. What it loads and its head are
// src/wiki/source.ts's.
export const Route = createFileRoute("/wiki/source/$")({
  loader: async ({ params }) => {
    const page = await (await import("../../../wiki/source")).sourcePage(params._splat ?? "");
    if (!page) throw notFound();
    return page;
  },
  head: ({ loaderData }) => (loaderData ? seo(loaderData.head) : {}),
  component: () => {
    const data = Route.useLoaderData();
    return <WikiFrame>{"file" in data ? <SourcePage file={data.file} /> : <RoomPage data={data} />}</WikiFrame>;
  },
});
