declare module "virtual:site/updated" {
  // When the build was made, as an HTTP date (vite.config.ts); null under `vite dev`.
  export const updated: string | null;
}

declare module "virtual:site/license" {
  // The Spelunky User License, read from the repo's LICENSE (src/build/license.ts).
  export const license: import("./build/license.ts").License;
}

declare module "virtual:site/release" {
  // The native apps' downloads, from the release's update manifest (src/build/release.ts); null without one.
  export const release: import("./build/release.ts").Release | null;
}
