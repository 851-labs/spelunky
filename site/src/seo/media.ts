// The wiki's media (docs/wiki-media.md) as search engines see them: a clip's VideoObject and a sprite's
// ImageObject for a page's JSON-LD, and which page shows which, for the video and image sitemaps
// (sitemap.ts). The media are the pinned package's, read at build time (media-package.ts) and imported from
// "virtual:wiki-media":
//
//   import wikiMedia from "virtual:wiki-media";
//   jsonLd: [article({ ... }), clipVideo(wikiMedia, "snake/bite"), spriteImage(wikiMedia, "sSnakeLeft")]
//
// Everything here takes them as an argument, so site/test-media.mjs can run it on a stand-in package.
import { imageObject, videoObject } from "./jsonld";
import type { JsonLd } from "./jsonld";

// What the pages need of the package's indexes. Paths are relative to the package, as in its JSON.
export interface WikiMedia {
  // Where the package is served: /wiki/media/HASH (site/wiki-media.sh).
  url: string;
  // ISO 8601: when the pinned release was published (wiki-media.txt's `date`), every clip's uploadDate.
  date: string;
  // By id (snake/bite).
  clips: Record<string, MediaClip>;
  // By name (sSnakeWalkL).
  sprites: Record<string, MediaSprite>;
  // Each object's own sprite (oSnake's is sSnakeLeft), for a page about it.
  objectSprites: Record<string, string>;
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

// A file of the package, site-relative. Each segment is escaped: the asset store redirects a sprite's
// 0@4x.png to 0%404x.png (docs/web-deploy.md, "Wiki media"), so that's the URL that's a 200.
export function mediaUrl(media: WikiMedia, file: string): string {
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

// "virtual:wiki-media" is null while site/wiki-media.txt pins no package, when a page can't show any.
function pinned(media: WikiMedia | null, what: string): WikiMedia {
  if (!media) throw new Error(`no wiki media for ${what}: site/wiki-media.txt pins none`);
  return media;
}

// A clip's VideoObject: its title and description from its spec, the poster as its thumbnail, the MP4 (every
// browser plays it) as its content, and the release's date. The game's notice, credit and license are
// videoObject()'s (docs/wiki-license.md, "JSON-LD").
export function clipVideo(wikiMedia: WikiMedia | null, id: string): JsonLd {
  const media = pinned(wikiMedia, `clip ${id}`);
  const clip = mediaClip(media, id);
  return videoObject({
    name: clip.title,
    description: clip.description,
    thumbnailUrl: mediaUrl(media, `clips/${clip.poster}`),
    contentUrl: mediaUrl(media, `clips/${clip.mp4}`),
    uploadDate: media.date,
    duration: isoDuration(clip.duration),
    width: clip.width,
    height: clip.height,
  });
}

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
    contentUrl: mediaUrl(media, `sprites/${(anim && sprite.anim) || sprite.image}`),
    name,
    caption,
    width: sprite.width,
    height: sprite.height,
  });
}

// The media a page shows, by its canonical path, for the sitemaps.
export interface PageMedia {
  path: string;
  clips?: string[];
  sprites?: string[];
}

// Until the wiki's pages say which clips they show (851-3333's `clips`), each clip is on its entity's page,
// its spec's folder (snake/bite on /wiki/snake), with that object's sprite (oSnake's) if there is one. The
// sitemaps only list a page that's in /sitemap.xml, so this lists nothing until the page is there.
export function entityPages(media: WikiMedia): PageMedia[] {
  const pages = new Map<string, PageMedia & { clips: string[]; sprites: string[] }>();
  for (const id of Object.keys(media.clips).sort()) {
    const entity = id.slice(0, id.indexOf("/"));
    let page = pages.get(entity);
    if (!page) {
      const sprite = media.objectSprites[`o${pascalCase(entity)}`];
      page = { path: `/wiki/${entity}`, clips: [], sprites: sprite && media.sprites[sprite] ? [sprite] : [] };
      pages.set(entity, page);
    }
    page.clips.push(id);
  }
  return [...pages.values()];
}

// yeti-king to YetiKing.
function pascalCase(slug: string): string {
  return slug.replace(/(?:^|-)([a-z0-9])/g, (_, c: string) => c.toUpperCase());
}

export const PAGE_MEDIA_SOURCES: Array<(media: WikiMedia) => PageMedia[]> = [entityPages];

// Each page among `pages` that shows media, with its clips and sprites (each once, in the sources' order),
// in the order of `pages`. A clip or sprite the package doesn't have fails the build.
export function mediaOnPages(
  media: WikiMedia | null,
  pages: string[],
  sources = PAGE_MEDIA_SOURCES,
): Array<{ path: string; clips: string[]; sprites: string[] }> {
  if (!media) return [];
  const byPath = new Map<string, { clips: Set<string>; sprites: Set<string> }>();
  for (const { path, clips = [], sprites = [] } of sources.flatMap((source) => source(media))) {
    clips.forEach((id) => mediaClip(media, id));
    sprites.forEach((name) => mediaSprite(media, name));
    const page = byPath.get(path) ?? { clips: new Set(), sprites: new Set() };
    clips.forEach((id) => page.clips.add(id));
    sprites.forEach((name) => page.sprites.add(name));
    byPath.set(path, page);
  }
  return pages.flatMap((path) => {
    const page = byPath.get(path);
    return page ? [{ path, clips: [...page.clips], sprites: [...page.sprites] }] : [];
  });
}
