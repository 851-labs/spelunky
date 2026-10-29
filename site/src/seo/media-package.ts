// Reads the wiki's media (docs/wiki-media.md) at build time: build.sh sets WIKI_MEDIA_DIR (wiki/media, or
// test.sh's stand-in), WIKI_MEDIA_URL, WIKI_MEDIA_DATE and WIKI_MEDIA_LOG. The one reader of its indexes: the
// pages, their hubs' cards and their Open Graph images (src/wiki/build/compile.ts, og.ts) read
// readMediaPackage's, and their JSON-LD and the sitemaps its wikiMedia(), so a sprite one can show the others
// can describe.
//
// The pages and sitemaps are rendered in workerd, which has no files to read, so a Vite plugin hands them
// wikiMedia() as "virtual:wiki-media" (media.ts), built in. Without WIKI_MEDIA_DIR (`vite dev`) the module is
// null.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { Plugin } from "vite";
import type { MediaClip, MediaSprite, WikiMedia } from "./media.ts";

const MODULE = "virtual:wiki-media";

export function wikiMediaPlugin(): Plugin {
  let code: string | undefined;
  return {
    name: "wiki-media",
    resolveId: (id) => (id === MODULE ? `\0${MODULE}` : undefined),
    load(id) {
      if (id !== `\0${MODULE}`) return undefined;
      const { WIKI_MEDIA_DIR: dir, WIKI_MEDIA_URL: url, WIKI_MEDIA_DATE: date, WIKI_MEDIA_LOG: log } = process.env;
      code ??= `export default ${JSON.stringify(dir ? wikiMedia(readMediaPackage(dir, url ?? "", date ?? "", log || null)) : null)};\n`;
      return code;
    },
  };
}

// What the build reads of the media. Paths are relative to clips/ and sprites/, as in their JSON.
export interface MediaPackage {
  // Where they are.
  dir: string;
  // As WikiMedia's.
  url: string;
  // When they last changed: the date of a clip the log hasn't got.
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
  // The clip's replay and its frame the clip starts on (clip.json's `replay_from`), for playing the clip in the
  // game (src/wiki/live-clip.tsx), and whether it's a scene's: one cut from a committed replay plays only in a
  // build that says it can (build/compile.ts's LiveClips.replays). Null for the settings program's, which the
  // media have no replay of, and for media from before they had the committed replays' (851-3572). `state` is
  // clip.json's `state_hash`, the game's state at the clip's first frame, and `states` its `states_hash`, the game's
  // states at each of the clip's frames, which the page holds the game at /play/ to (src/wiki/live-clip-player.tsx).
  live: { replay: string; from: number; scene: boolean; state: string; states: string } | null;
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

const ISO_DATE = /^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2}))?$/;

// The media at DIR, served at URL (/wiki/media) and changed last on DATE (ISO 8601), their clips dated by the
// git log in the file LOG (build.sh's), or all on DATE without one. Throws if they aren't what
// scripts/wiki-media renders.
export function readMediaPackage(dir: string, url: string, date: string, log: string | null = null): MediaPackage {
  if (url !== "/wiki/media") throw new Error(`wiki media: URL ${url}, not /wiki/media`);
  if (!ISO_DATE.test(date)) throw new Error(`wiki media: date '${date}' isn't ISO 8601 (build.sh's WIKI_MEDIA_DATE)`);
  const read = (file: string) => JSON.parse(readFileSync(join(dir, file), "utf8"));
  const changed = log === null ? new Map<string, string>() : fileDates(readFileSync(log, "utf8"));

  const clips: Record<string, PackageClip> = {};
  for (const clip of array(read("clips/index.json").clips, "clips/index.json: clips")) {
    const where = `clips/index.json: ${clip.id}`;
    const mp4 = string(clip.files?.mp4, `${where}: files.mp4`);
    const webm = string(clip.files?.webm, `${where}: files.webm`);
    clips[string(clip.id, "clips/index.json: a clip's id")] = {
      title: string(clip.title, `${where}: title`),
      description: string(clip.description, `${where}: description`),
      duration: number(clip.duration, `${where}: duration`),
      width: number(clip.width, `${where}: width`),
      height: number(clip.height, `${where}: height`),
      mp4,
      poster: string(clip.files?.poster, `${where}: files.poster`),
      // Media from before clips had sound don't say.
      audio: clip.has_audio === true,
      // Its videos' last change, not its clip.json's: a new title or description isn't a new video, which a
      // newer uploadDate tells search engines it is.
      date: [changed.get(mp4), changed.get(webm)].filter((d) => d !== undefined).sort().at(-1) ?? date,
      webm,
      // The replay's frame, `replay_from`, not the spec's `from`, which counts from a scene's first: its replay
      // boots the game and loads the scene's level from the title's menu before it. A scene always has its replay.
      live:
        clip.source?.kind === "scene" || clip.files?.replay !== undefined
          ? {
              replay: string(clip.files?.replay, `${where}: files.replay`),
              from: frame(clip.replay_from, `${where}: replay_from`),
              scene: clip.source?.kind === "scene",
              state: hash(clip.state_hash, `${where}: state_hash`),
              states: hash(clip.states_hash, `${where}: states_hash`),
            }
          : null,
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

// The sprite NAME as the pages show it and their JSON-LD describes it: its first frame and its animation,
// upscaled. Or what's wrong: the media haven't got it, or it has no pixels (wiki-sprites writes no files for
// one: PNG can't hold an empty image), so there's no image of it to show.
export function spriteWithPixels(media: MediaPackage, name: string): MediaSprite | string {
  if (!Object.hasOwn(media.sprites, name)) return `no sprite ${name} in the wiki media`;
  const { width, height, frames, anim } = media.sprites[name];
  const image = frames[0]?.scaled;
  if (width === 0 || height === 0 || !image) return `${name} has no pixels in the wiki media, so no image to show`;
  return { width: width * media.scale, height: height * media.scale, image, anim };
}

// What the pages' JSON-LD and the sitemaps need of the media (media.ts): the clips, and the sprites with
// pixels.
export function wikiMedia(media: MediaPackage): WikiMedia {
  const clips: Record<string, MediaClip> = {};
  for (const [id, { title, description, duration, width, height, mp4, poster, audio, date }] of Object.entries(media.clips)) {
    clips[id] = { title, description, duration, width, height, mp4, poster, audio, date };
  }
  const sprites: Record<string, MediaSprite> = {};
  for (const name of Object.keys(media.sprites)) {
    const sprite = spriteWithPixels(media, name);
    if (typeof sprite !== "string") sprites[name] = sprite;
  }
  return { url: media.url, clips, sprites };
}

// When each file last changed (the newest commit that changed it), by its path, from LOG: build.sh's git log
// of the clips' videos, each commit an '@DATE' line (UTC, to the second, so that the dates compare as
// strings), then the files it changed, a line each, relative to clips/.
export function fileDates(log: string): Map<string, string> {
  const dates = new Map<string, string>();
  let date: string | null = null;
  for (const line of log.split("\n")) {
    if (line.startsWith("@")) {
      date = line.slice(1);
      if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(date))
        throw new Error(`wiki media: the clips' log has date '${date}', not UTC's YYYY-MM-DDTHH:MM:SSZ (build.sh's WIKI_MEDIA_LOG)`);
    } else if (line !== "") {
      if (date === null) throw new Error(`wiki media: the clips' log names ${line} before a date (build.sh's WIKI_MEDIA_LOG)`);
      const newest = dates.get(line);
      if (!newest || date > newest) dates.set(line, date);
    }
  }
  return dates;
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

// A frame number: a scene's first is 0.
function frame(value: unknown, what: string): number {
  if (!Number.isInteger(value) || (value as number) < 0) throw new Error(`wiki media: ${what} isn't a frame number`);
  return value as number;
}

// One of scripts/wiki-media's hashes: 16 hex digits.
function hash(value: unknown, what: string): string {
  if (typeof value !== "string" || !/^[0-9a-f]{16}$/.test(value)) throw new Error(`wiki media: ${what} isn't a hash`);
  return value;
}

// A sprite without pixels may be 0 wide or high.
function size(value: unknown, what: string): number {
  if (typeof value !== "number" || !(value >= 0)) throw new Error(`wiki media: ${what} isn't a number`);
  return value;
}
