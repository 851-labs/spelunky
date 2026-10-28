// Reads the wiki's media package (docs/wiki-media.md) at build time: build.sh unpacks the pinned one and sets
// WIKI_MEDIA_DIR, WIKI_MEDIA_URL and WIKI_MEDIA_DATE. The one reader of its indexes: the pages, their hubs'
// cards and their Open Graph images (src/wiki/build/compile.ts, og.ts) read readMediaPackage's, and their
// JSON-LD and the sitemaps its wikiMedia(), so a sprite one can show the others can describe.
//
// The pages and sitemaps are rendered in workerd, which has no files to read, so a Vite plugin hands them
// wikiMedia() as "virtual:wiki-media" (media.ts), built in. Without WIKI_MEDIA_DIR (no pin, or `vite dev`) the
// module is null.
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
      code ??= `export default ${JSON.stringify(dir ? wikiMedia(readMediaPackage(dir, url ?? "", date ?? "")) : null)};\n`;
      return code;
    },
  };
}

// What the build reads of the package. Paths are relative to clips/ and sprites/, as in its JSON.
export interface MediaPackage {
  // Where it's unpacked.
  dir: string;
  // As WikiMedia's.
  url: string;
  date: string;
  // How many screen pixels a game pixel is in the sprites' upscaled files (index.json's `scale`: 4).
  scale: number;
  // By id (snake/bite).
  clips: Record<string, PackageClip>;
  // By name (sSnakeWalkL), with or without pixels.
  sprites: Record<string, PackageSprite>;
  // Each object's own sprite, by name, or null for one without (oGame).
  objects: Record<string, string | null>;
}

export interface PackageClip extends MediaClip {
  webm: string;
}

export interface PackageSprite {
  // In game pixels.
  width: number;
  height: number;
  // Each frame's file, at native size and upscaled. A sprite without pixels has neither (wiki-sprites writes
  // null: PNG can't hold an empty image).
  frames: { file: string | null; scaled: string | null }[];
  // The frames side by side, at native size.
  strip: string | null;
  // The upscaled animation, for a sprite of several frames.
  anim: string | null;
}

// The package unpacked at DIR, served at URL (/wiki/media/HASH) and published on DATE (ISO 8601). Throws if
// it isn't one scripts/wiki-media-package made.
export function readMediaPackage(dir: string, url: string, date: string): MediaPackage {
  if (!/^\/wiki\/media\/[0-9a-f]{16}$/.test(url)) throw new Error(`wiki media: URL ${url}, not /wiki/media/HASH`);
  if (!/^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2}))?$/.test(date))
    throw new Error(`wiki media: date '${date}' isn't ISO 8601 (site/wiki-media.txt's date line)`);
  const read = (file: string) => JSON.parse(readFileSync(join(dir, file), "utf8"));

  const clips: Record<string, PackageClip> = {};
  for (const clip of array(read("clips/index.json").clips, "clips/index.json: clips")) {
    const where = `clips/index.json: ${clip.id}`;
    clips[string(clip.id, "clips/index.json: a clip's id")] = {
      title: string(clip.title, `${where}: title`),
      description: string(clip.description, `${where}: description`),
      duration: number(clip.duration, `${where}: duration`),
      width: number(clip.width, `${where}: width`),
      height: number(clip.height, `${where}: height`),
      mp4: string(clip.files?.mp4, `${where}: files.mp4`),
      poster: string(clip.files?.poster, `${where}: files.poster`),
      // Media from before clips had sound don't say.
      audio: clip.has_audio === true,
      webm: string(clip.files?.webm, `${where}: files.webm`),
    };
  }

  const index = read("sprites/index.json");
  const sprites: Record<string, PackageSprite> = {};
  for (const sprite of array(index.sprites, "sprites/index.json: sprites")) {
    const where = `sprites/index.json: ${sprite.name}`;
    // Each sprite's sprite.json names its files.
    const file = string(sprite.meta, `${where}: meta`);
    const meta = read(`sprites/${file}`);
    const frames = array(meta.frames, `sprites/${file}: frames`).map((frame, i) => ({
      file: optional(frame.file, `sprites/${file}: frames[${i}].file`),
      scaled: optional(frame.file_scaled, `sprites/${file}: frames[${i}].file_scaled`),
    }));
    if (frames.length !== sprite.frames) throw new Error(`wiki media: ${where}: frames is ${sprite.frames}, but sprites/${file} has ${frames.length}`);
    sprites[string(sprite.name, "sprites/index.json: a sprite's name")] = {
      width: size(sprite.width, `${where}: width`),
      height: size(sprite.height, `${where}: height`),
      frames,
      strip: optional(meta.strip, `sprites/${file}: strip`),
      anim: optional(meta.anim_scaled, `sprites/${file}: anim_scaled`),
    };
  }
  const objects: Record<string, string | null> = {};
  for (const object of array(index.objects ?? [], "sprites/index.json: objects")) {
    objects[string(object.name, "sprites/index.json: an object's name")] = optional(object.sprite, `sprites/index.json: ${object.name}: sprite`);
  }
  return { dir, url, date, scale: number(index.scale, "sprites/index.json: scale"), clips, sprites, objects };
}

// The sprite NAME of the package's that has pixels, or what's wrong: the package hasn't got it, or it has no
// pixels, so there's no image of it to show.
export function spriteWithPixels(media: MediaPackage, name: string): PackageSprite | string {
  if (!Object.hasOwn(media.sprites, name)) return `no sprite ${name} in the wiki media`;
  const sprite = media.sprites[name];
  if (sprite.frames[0]?.scaled == null) return `${name} has no pixels in the wiki media, so no image to show`;
  return sprite;
}

// What the pages' JSON-LD and the sitemaps need of the package (media.ts): the clips, and the sprites with
// pixels, their sizes upscaled.
export function wikiMedia(media: MediaPackage): WikiMedia {
  const clips: Record<string, MediaClip> = {};
  for (const [id, { title, description, duration, width, height, mp4, poster, audio }] of Object.entries(media.clips)) {
    clips[id] = { title, description, duration, width, height, mp4, poster, audio };
  }
  const sprites: Record<string, MediaSprite> = {};
  for (const name of Object.keys(media.sprites)) {
    const sprite = spriteWithPixels(media, name);
    if (typeof sprite === "string") continue;
    sprites[name] = {
      width: number(sprite.width, `sprites/index.json: ${name}: width`) * media.scale,
      height: number(sprite.height, `sprites/index.json: ${name}: height`) * media.scale,
      image: sprite.frames[0].scaled!,
      anim: sprite.anim,
    };
  }
  return { url: media.url, date: media.date, clips, sprites };
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

// A file that isn't there for a sprite without pixels (or of one frame, its animation): null.
function optional(value: unknown, what: string): string | null {
  return value === null || value === undefined ? null : string(value, what);
}

function number(value: unknown, what: string): number {
  if (typeof value !== "number" || !(value > 0)) throw new Error(`wiki media: ${what} isn't a positive number`);
  return value;
}

// A sprite without pixels may be 0 wide or high.
function size(value: unknown, what: string): number {
  if (typeof value !== "number" || !(value >= 0)) throw new Error(`wiki media: ${what} isn't a number`);
  return value;
}
