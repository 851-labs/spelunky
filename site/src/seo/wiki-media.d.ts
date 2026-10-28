// The wiki media's indexes, read at build time (media-package.ts); null without them (`vite dev`).
declare module "virtual:wiki-media" {
  import type { WikiMedia } from "./media";
  const media: WikiMedia | null;
  export default media;
}
