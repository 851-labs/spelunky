// The wiki's breadcrumbs: the trail a page or hub shows above its title and its BreadcrumbList JSON-LD both
// come from here, so they can't disagree. URLs are flat (851-3329), so a page's trail is its first
// category, not its path: Wiki, the category's hub (after /wiki/items for an item's), then the page.
import type { Crumb } from "../seo/jsonld";
import { hubById } from "./build/categories.ts";
import type { Category, Hub } from "./build/categories.ts";

const WIKI: Crumb = { name: "Wiki", path: "/wiki" };

// A hub's trail: Wiki, the hub above it if any, the hub.
export function hubTrail(hub: Hub): Crumb[] {
  const parent = hub.parent === null ? undefined : hubById(hub.parent);
  return [WIKI, ...(parent ? [{ name: parent.name, path: parent.path }] : []), { name: hub.name, path: hub.path }];
}

export function trail(page: { slug: string; title: string; categories: readonly Category[] }): Crumb[] {
  // Every category has one (categories.ts's HUBS).
  const hub = hubById(page.categories[0])!;
  return [...hubTrail(hub), { name: page.title, path: `/wiki/${page.slug}` }];
}
