import { createFileRoute } from "@tanstack/react-router";
import { home } from "virtual:wiki/hubs";
import { itemList } from "../../seo/jsonld";
import { seo } from "../../seo/seo";
import { HUBS } from "../../wiki/build/categories.ts";
import type { HomeData } from "../../wiki/build/hubs";
import { WikiHome } from "../../wiki/hub";
import { WikiFrame } from "../../wiki/layout";
import { WikiSearch } from "../../wiki/search-box";
import css from "../../wiki/wiki.css?raw";

// /wiki: the wiki's hub (src/wiki/hub.tsx), its contents loaded as it pre-renders, so the page carries them;
// and its search box, whose results are /wiki?q=<query> (the WebSite JSON-LD's SearchAction, src/seo/jsonld.ts).
export const Route = createFileRoute("/wiki/")({
  validateSearch: (search: Record<string, unknown>): { q?: string } =>
    typeof search.q === "string" ? { q: search.q } : {},
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
  const { q = "" } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <WikiFrame>
      <WikiHome
        data={Route.useLoaderData()}
        search={
          // Each change replaces the URL, so it's the search's link, and Back leaves the page.
          <WikiSearch query={q} onQuery={(query) => navigate({ search: query ? { q: query } : {}, replace: true })} />
        }
      />
    </WikiFrame>
  );
}
