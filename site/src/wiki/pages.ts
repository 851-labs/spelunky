// A wiki page as the routes see it: the module src/wiki/build/compile.ts makes of it, loaded on demand.
import type { ComponentType, ReactPromise } from "react";
import { loaders } from "virtual:wiki/loaders";
import type { PageMeta, Relation, RelationLink } from "./build/compile";
import type { WikiComponents } from "./components";

export type { PageMeta, Relation, RelationLink };

// A compiled MDX fragment: the body, the lede, an infobox value, an answer.
export type Fragment = ComponentType<{ components: WikiComponents }>;

export interface PageModule {
  meta: PageMeta;
  Body: Fragment;
  Lede: Fragment;
  Hatnote: Fragment | null;
  Answers: Fragment[];
  Infobox: Record<string, Fragment>;
}

// What the /wiki index and the sitemaps list (virtual:wiki/pages): with the clips the page shows and its
// infobox's sprite, for the media sitemaps (src/wiki/sitemap.ts).
export type PageSummary = Pick<PageMeta, "slug" | "title" | "type" | "categories" | "lede" | "verified"> & {
  clips: string[];
  sprite: { name: string; anim: boolean } | null;
};

// A promise React's use() reads at once when it's settled: the route's loader awaits it before the page
// renders, so the server render never suspends (which would stream the page after a fallback). In the
// browser, hydration doesn't rerun the loader: the page suspends until its chunk loads, keeping the
// pre-rendered HTML meanwhile.
type Tracked<T> = Promise<T> & ReactPromise<T>;
const cache = new Map<string, Tracked<PageModule>>();

// The page, or null if there's none of that slug.
export function loadPage(slug: string): Tracked<PageModule> | null {
  let page = cache.get(slug);
  if (page) return page;
  const load = Object.hasOwn(loaders, slug) ? loaders[slug] : undefined;
  if (!load) return null;
  const promise = load().then((m) => m.default) as Tracked<PageModule>;
  promise.then(
    (value) => Object.assign(promise, { status: "fulfilled", value }),
    // A chunk that didn't load (the network, say) is tried again next time; the page shows the error.
    () => cache.delete(slug),
  );
  cache.set(slug, promise);
  return promise;
}
