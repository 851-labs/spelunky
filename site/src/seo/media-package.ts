// Reads the wiki's media package (docs/wiki-media.md) at build time, for "virtual:wiki-media" (media.ts):
// build.sh unpacks the pinned one and sets WIKI_MEDIA_DIR, WIKI_MEDIA_URL and WIKI_MEDIA_DATE. The pages and
// sitemaps are rendered in workerd, which has no files to read, so this Vite plugin hands them what they need
// of the indexes as a module, built in. Without WIKI_MEDIA_DIR (no pin, or `vite dev`) the module is null.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { Plugin } from "vite";
import type { MediaClip, MediaSprite, WikiMedia } from "./media";

const MODULE = "virtual:wiki-media";

export function wikiMediaPlugin(): Plugin {
  let code: string | undefined;
  return {
    name: "wiki-media",
    resolveId: (id) => (id === MODULE ? `\0${MODULE}` : undefined),
    load(id) {
      if (id !== `\0${MODULE}`) return undefined;
      const { WIKI_MEDIA_DIR: dir, WIKI_MEDIA_URL: url, WIKI_MEDIA_DATE: date } = process.env;
      code ??= `export default ${JSON.stringify(dir ? readWikiMedia(dir, url ?? "", date ?? "") : null)};\n`;
      return code;
    },
  };
}

// The package unpacked at DIR, served at URL (/wiki/media/HASH) and published on DATE (ISO 8601). Throws if
// it isn't one scripts/wiki-media-package made.
export function readWikiMedia(dir: string, url: string, date: string): WikiMedia {
  if (!/^\/wiki\/media\/[0-9a-f]{16}$/.test(url)) throw new Error(`wiki media: URL ${url}, not /wiki/media/HASH`);
  if (!/^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2}))?$/.test(date))
    throw new Error(`wiki media: date '${date}' isn't ISO 8601 (site/wiki-media.txt's date line)`);
  const read = (file: string) => JSON.parse(readFileSync(join(dir, file), "utf8"));

  const clips: Record<string, MediaClip> = {};
  for (const clip of array(read("clips/index.json").clips, "clips/index.json: clips")) {
    const where = `clips/index.json: ${clip.id}`;
    clips[clip.id] = {
      title: string(clip.title, `${where}: title`),
      description: string(clip.description, `${where}: description`),
      duration: number(clip.duration, `${where}: duration`),
      width: number(clip.width, `${where}: width`),
      height: number(clip.height, `${where}: height`),
      mp4: string(clip.files?.mp4, `${where}: files.mp4`),
      poster: string(clip.files?.poster, `${where}: files.poster`),
    };
  }

  const index = read("sprites/index.json");
  const scale = number(index.scale, "sprites/index.json: scale");
  const sprites: Record<string, MediaSprite> = {};
  for (const sprite of array(index.sprites, "sprites/index.json: sprites")) {
    // Each sprite's sprite.json names its files. A sprite without pixels has none (wiki-sprites writes
    // null), so there's no image of it to show.
    const meta = read(`sprites/${string(sprite.meta, `sprites/index.json: ${sprite.name}: meta`)}`);
    const image = meta.frames?.[0]?.file_scaled;
    if (image === null || image === undefined) continue;
    sprites[sprite.name] = {
      width: number(sprite.width, `sprites/index.json: ${sprite.name}: width`) * scale,
      height: number(sprite.height, `sprites/index.json: ${sprite.name}: height`) * scale,
      image: string(image, `sprites/${sprite.meta}: frames[0].file_scaled`),
      anim: meta.anim_scaled ?? null,
    };
  }
  return { url, date, clips, sprites };
}

// The JSON's values are unchecked, so each is looked at as it's read.
function array(value: unknown, what: string): any[] {
  if (!Array.isArray(value)) throw new Error(`wiki media: ${what} isn't a list`);
  return value;
}

function string(value: unknown, what: string): string {
  if (typeof value !== "string" || value === "") throw new Error(`wiki media: ${what} isn't a string`);
  return value;
}

function number(value: unknown, what: string): number {
  if (typeof value !== "number" || !(value > 0)) throw new Error(`wiki media: ${what} isn't a positive number`);
  return value;
}
