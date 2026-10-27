// The modules src/wiki/build/plugin.ts makes from the pages.
declare module "virtual:wiki/pages" {
  import type { PageSummary } from "./pages";
  export const pages: PageSummary[];
}

declare module "virtual:wiki/loaders" {
  import type { PageModule } from "./pages";
  export const loaders: Record<string, () => Promise<{ default: PageModule }>>;
}

declare module "virtual:wiki/hubs" {
  import type { HomeData, HubData } from "./build/hubs";
  export const home: () => Promise<{ default: HomeData }>;
  export const hubs: Record<string, () => Promise<{ default: HubData }>>;
}

declare module "virtual:wiki/search-index" {
  import type { SearchIndex } from "./search";
  export const index: SearchIndex;
}
