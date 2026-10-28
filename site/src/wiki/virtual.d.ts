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

declare module "virtual:wiki/hover-cards" {
  import type { HoverCard } from "./build/cards";
  export const cards: Record<string, HoverCard>;
}

// The source viewer's, from src/wiki/build/source.ts.
declare module "virtual:wiki-source/files" {
  import type { SourceFile } from "./build/source";
  export function sourceFile(path: string): SourceFile | null;
}

declare module "virtual:wiki-source/index" {
  import type { SourceIndex } from "./build/source";
  const index: SourceIndex;
  export default index;
}
