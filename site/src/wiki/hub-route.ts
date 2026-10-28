// What the hubs' routes need outside their components (src/wiki/hub.tsx): their loaders' data and their heads.
// The route tree is in every page's entry chunk, the landing page's too, and this is the part of the routes
// that isn't split off with their components, so it mustn't import a component: through one, the whole wiki's
// UI and Base UI would load on every page (851-3407).
import { hubs } from "virtual:wiki/hubs";
import { breadcrumbs, itemList } from "../seo/jsonld";
import { seo } from "../seo/seo";
import { hubById } from "./build/categories.ts";
import type { Hub } from "./build/categories.ts";
import type { HubData } from "./build/hubs";
import { hubTrail } from "./crumbs";
import css from "./wiki.css?raw";

// A hub's data, for its route's loader, or null if there's no such hub. Each hub's is its own chunk
// (src/wiki/build/plugin.ts): the pre-rendered page carries what it shows, so the browser hydrates it without
// loading the chunk, and loads it only if the router ever navigates there itself.
export async function loadHub(id: string): Promise<{ hub: Hub; data: HubData } | null> {
  const hub = hubById(id);
  return hub ? { hub, data: (await hubs[hub.id]()).default } : null;
}

// A hub's head: its title and description, its breadcrumbs, and an ItemList of its written pages in the
// grid's order (the ones to come have no URL to list). One with none written is noindex, and not in the
// sitemap (src/wiki/sitemap.ts), until it has: there's nothing on it to read yet.
export function hubHead(hub: Hub, data: HubData) {
  const written = data.cards.filter((card) => card.href !== null);
  return {
    ...seo({
      title: `${hub.name} – Spelunky Classic fan wiki`,
      description: hub.description,
      path: hub.path,
      image: data.og ?? undefined,
      noindex: written.length === 0,
      search: true,
      jsonLd: [
        breadcrumbs(hubTrail(hub)),
        ...(written.length === 0 ? [] : [itemList(`${hub.name} in the Spelunky Classic fan wiki`, written.map((card) => ({ name: card.title, path: card.href! })))]),
      ],
    }),
    styles: [{ children: css }],
  };
}

// The hubs' ids that a single path segment names (/wiki/monsters), which the page route defers to.
export const isTopHub = (slug: string) => hubById(slug)?.parent === null;
