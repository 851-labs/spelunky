// The modules src/wiki/build/plugin.ts makes from the pages.
declare module "virtual:wiki/pages" {
  import type { PageSummary } from "./pages";
  export const pages: PageSummary[];
}

declare module "virtual:wiki/loaders" {
  import type { PageModule } from "./pages";
  export const loaders: Record<string, () => Promise<{ default: PageModule }>>;
}
