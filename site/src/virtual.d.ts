declare module "virtual:site/updated" {
  // When the build was made, as an HTTP date (vite.config.ts); null under `vite dev`.
  export const updated: string | null;
}
