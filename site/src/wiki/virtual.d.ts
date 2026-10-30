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

declare module "virtual:wiki/aliases" {
  // Each page's aliases, to its slug.
  export const aliases: Map<string, string>;
}

declare module "virtual:wiki/quirk-links" {
  // Each page's links to docs/quirks.md, by key (src/wiki/quirk-links.ts), by slug; empty in the browser.
  export const quirkLinks: Record<string, Record<string, string>>;
}

declare module "virtual:wiki/source-links" {
  import type { PageLink } from "./build/plugin";
  // The page each object and room is on, by name.
  export const objectPages: Record<string, PageLink>;
  export const roomPages: Record<string, PageLink>;
}

// The source viewer's, from src/wiki/build/source.ts.
declare module "virtual:wiki-source/files" {
  import type { SourceFile, SourceRoom } from "./build/source";
  export function sourceFile(path: string): SourceFile | null;
  export function sourceRoom(name: string): SourceRoom | null;
}

declare module "virtual:wiki-source/index" {
  import type { SourceIndex } from "./build/source";
  const index: SourceIndex;
  export default index;
}
