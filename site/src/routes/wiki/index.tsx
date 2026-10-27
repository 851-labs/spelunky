import { createFileRoute } from "@tanstack/react-router";
import { home } from "virtual:wiki/hubs";
import { itemList } from "../../seo/jsonld";
import { seo } from "../../seo/seo";
import { HUBS } from "../../wiki/build/categories.ts";
import type { HomeData } from "../../wiki/build/hubs";
import { WikiHome } from "../../wiki/hub";
import { WikiFrame } from "../../wiki/layout";
import css from "../../wiki/wiki.css?raw";

// /wiki: the wiki's hub (src/wiki/hub.tsx), its contents loaded as it pre-renders, so the page carries them.
export const Route = createFileRoute("/wiki/")({
  loader: async () => (await home()).default,
  head: ({ loaderData }: { loaderData?: HomeData }) => ({
    ...seo({
      title: "Spelunky Classic fan wiki",
      description:
        "An unofficial fan wiki for Spelunky Classic 1.1: its monsters, items, traps and places, with numbers " +
        "from the game's code and clips recorded from the game.",
      path: "/wiki",
      // Its categories, as it lists them: those with a page written, as the rest are noindex (src/wiki/hub.tsx).
      jsonLd: [
        itemList(
          "Spelunky Classic fan wiki categories",
          HUBS.filter((hub) => loaderData?.hubs.find((h) => h.id === hub.id)?.written).map((hub) => ({ name: hub.name, path: hub.path })),
        ),
      ],
    }),
    styles: [{ children: css }],
  }),
  component: WikiIndex,
});

function WikiIndex() {
  return (
    <WikiFrame>
      <WikiHome data={Route.useLoaderData()} />
    </WikiFrame>
  );
}
