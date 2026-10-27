// A wiki page's breadcrumbs: the trail the page shows above its title and its BreadcrumbList JSON-LD both
// come from here, so they can't disagree. URLs are flat (851-3329), so the trail is the page's first
// category, not its path: Wiki, then the category's hub once it has one (src/wiki/build/categories.ts), then
// the page.
import type { Crumb } from "../seo/jsonld";
import { hubPath, CATEGORY_NAMES } from "./build/categories.ts";
import type { Category } from "./build/categories.ts";

export function trail(page: { slug: string; title: string; categories: readonly Category[] }): Crumb[] {
  const category = page.categories[0];
  const hub = hubPath(category);
  return [
    { name: "Wiki", path: "/wiki" },
    ...(hub === null ? [] : [{ name: CATEGORY_NAMES[category], path: hub }]),
    { name: page.title, path: `/wiki/${page.slug}` },
  ];
}
