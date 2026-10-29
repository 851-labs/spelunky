// The wiki's media (docs/wiki-media.md) as search engines see them: a clip's VideoObject and a sprite's
// ImageObject for a page's JSON-LD, and which page shows which, for the video and image sitemaps
// (sitemap.ts). The media are wiki/media's, read at build time (media-package.ts).
//
// A wiki page's nodes are made when the wiki is compiled (src/wiki/build/compile.ts's mediaJsonLd) and come
// with the page: a route's code is the browser's too, so a route that imported "virtual:wiki-media" for its
// head would put the whole index in every page's script. Only the sitemaps' routes, which are the Worker's
// alone, import it (site/test.sh checks no browser script has it).
//
// Everything here takes the media as an argument, so site/test-media.mjs can run it on stand-in media.
import { imageObject, videoObject } from "./jsonld.ts";
import type { JsonLd } from "./jsonld.ts";

// What the pages need of the media's indexes. Paths are relative to the media, as in their JSON.
export interface WikiMedia {
  // Where the media are served: /wiki/media (site/build.sh).
  url: string;
  // By id (snake/bite).
  clips: Record<string, MediaClip>;
  // By name (sSnakeWalkL).
  sprites: Record<string, MediaSprite>;
}

export interface MediaClip {
  title: string;
  description: string;
  // In seconds.
  duration: number;
  width: number;
  height: number;
  mp4: string;
  poster: string;
  // It has the game's sound and music.
  audio: boolean;
  // ISO 8601: when its videos last changed (the last commit that changed them, build.sh), its uploadDate.
  date: string;
}

export interface MediaSprite {
  // Upscaled, as `image` and `anim` are.
  width: number;
  height: number;
  // The first frame.
  image: string;
  // The animation, for a sprite of several frames.
  anim: string | null;
}

// A file of the media, site-relative. Each segment is escaped: the asset store redirects a sprite's
// 0@4x.png to 0%404x.png (docs/web-deploy.md, "Wiki media"), so that's the URL that's a 200.
export function mediaUrl(media: Pick<WikiMedia, "url">, file: string): string {
  return `${media.url}/${file.split("/").map(encodeURIComponent).join("/")}`;
}

export function mediaClip(media: WikiMedia, id: string): MediaClip {
  const clip = media.clips[id];
  if (!clip) throw new Error(`the wiki's media have no clip ${id}`);
  return clip;
}

export function mediaSprite(media: WikiMedia, name: string): MediaSprite {
  const sprite = media.sprites[name];
  if (!sprite) throw new Error(`the wiki's media have no sprite ${name}`);
  return sprite;
}

// "virtual:wiki-media" is null without the media (`vite dev`), when a page can't show any.
function pinned(media: WikiMedia | null, what: string): WikiMedia {
  if (!media) throw new Error(`no wiki media for ${what}: the build had no WIKI_MEDIA_DIR`);
  return media;
}

// A clip's VideoObject: its title and description from its spec, the poster as its thumbnail, the MP4 (every
// browser plays it) as its content, and when its videos last changed. The game's notice, credit and license are
// videoObject()'s (docs/wiki-license.md, "JSON-LD"); a clip with sound credits the music's authors too
// ("Audio credit").
export function clipVideo(wikiMedia: WikiMedia | null, id: string): JsonLd {
  const media = pinned(wikiMedia, `clip ${id}`);
  const clip = mediaClip(media, id);
  return videoObject({
    ...(clip.audio ? { creditText: AUDIO_CREDIT_TEXT } : {}),
    name: clip.title,
    description: clip.description,
    thumbnailUrl: mediaUrl(media, `clips/${clip.poster}`),
    contentUrl: mediaUrl(media, `clips/${clip.mp4}`),
    uploadDate: clip.date,
    duration: isoDuration(clip.duration),
    width: clip.width,
    height: clip.height,
  });
}

const AUDIO_CREDIT_TEXT = "Derek Yu and Mossmouth, LLC (Spelunky Classic); music by George Buzinkai and Jonathan Perry";

// ISO 8601, in whole seconds (the clips are 3 to 20): PT3S. As Google's examples have it, and its video
// sitemap's <video:duration> can only be.
export function isoDuration(seconds: number): string {
  return `PT${durationSeconds(seconds)}S`;
}

export function durationSeconds(seconds: number): number {
  return Math.max(1, Math.round(seconds));
}

export interface SpriteImageProps {
  // The animation rather than the first frame, as a page's infobox may show it (a still if it has one frame).
  anim?: boolean;
  // What it shows, for people: "A snake".
  caption?: string;
}

// A sprite's ImageObject, upscaled as a page shows it.
export function spriteImage(
  wikiMedia: WikiMedia | null,
  name: string,
  { anim, caption }: SpriteImageProps = {},
): JsonLd {
  const media = pinned(wikiMedia, `sprite ${name}`);
  const sprite = mediaSprite(media, name);
  return imageObject({
    contentUrl: mediaUrl(media, `sprites/${spriteFile(sprite, anim)}`),
    name,
    caption,
    width: sprite.width,
    height: sprite.height,
  });
}

// The media a page shows, by its canonical path, for the sitemaps (sitemap.ts's PAGE_MEDIA_SOURCES).
export interface PageMedia {
  path: string;
  // Clip ids.
  clips?: string[];
  sprites?: PageSprite[];
}

// A sprite as the page shows it: its animation (`anim`, if it has one) or its first frame.
export interface PageSprite {
  name: string;
  anim?: boolean;
}

// The file of a sprite a page shows, in the media.
export function spriteFile(sprite: MediaSprite, anim = false): string {
  return (anim && sprite.anim) || sprite.image;
}

// Each page among `pages` that shows media, with its clips and sprites (each once, in the sources' order),
// in the order of `pages`. A clip the media don't have is left out: a page may show a clip whose spec has no
// render yet, which scripts/check fails a pull request for (src/wiki/build/compile.ts warns of it). So is
// a sprite it hasn't got or has no pixels of, which compile.ts fails the build for anyway.
export function mediaOnPages(
  media: WikiMedia | null,
  pages: string[],
  sources: Array<(media: WikiMedia) => PageMedia[]>,
): Array<{ path: string; clips: string[]; sprites: PageSprite[] }> {
  if (!media) return [];
  const byPath = new Map<string, { clips: Set<string>; sprites: Map<string, PageSprite> }>();
  for (const { path, clips = [], sprites = [] } of sources.flatMap((source) => source(media))) {
    const page = byPath.get(path) ?? { clips: new Set(), sprites: new Map() };
    for (const id of clips) if (Object.hasOwn(media.clips, id)) page.clips.add(id);
    for (const sprite of sprites) {
      const anim = sprite.anim ?? false;
      if (!Object.hasOwn(media.sprites, sprite.name)) continue;
      page.sprites.set(`${sprite.name} ${anim}`, { name: sprite.name, anim });
    }
    byPath.set(path, page);
  }
  return pages.flatMap((path) => {
    const page = byPath.get(path);
    return page ? [{ path, clips: [...page.clips], sprites: [...page.sprites.values()] }] : [];
  });
}
