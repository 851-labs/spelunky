// What /sitemap.xml lists (src/routes/sitemap[.]xml.ts, pre-rendered to a static file): every page a search
// engine should index, by its canonical path. Each source below is a list of pages: the site's own, and the
// wiki's (src/wiki/sitemap.ts). site/test-seo.mjs fails if a pre-rendered page is missing or the
// sitemap lists a page that isn't served.
//
// And Google's video and image sitemaps, /sitemap-video.xml and /sitemap-image.xml: the wiki's clips and
// sprites (media.ts), each under the page that shows it, of those /sitemap.xml lists. robots.txt names each
// sitemap that has a page in it.
import { wikiPageMedia, wikiSitemap } from "../wiki/sitemap";
import { durationSeconds, mediaClip, mediaOnPages, mediaSprite, mediaUrl, spriteFile } from "./media";
import type { PageMedia, WikiMedia } from "./media";
import { absoluteUrl, SITE_URL } from "./seo";

export interface SitemapEntry {
  // The canonical path, as seo({ path }) has it.
  path: string;
  // ISO 8601 date of the last real change (a wiki page's verified date); left out rather than guessed.
  lastmod?: string;
}

// The pages outside the wiki: the landing page and the game (build.sh's /play/, not a route).
const SITE_PAGES: SitemapEntry[] = [{ path: "/" }, { path: "/play/" }];

export const SITEMAP_SOURCES: Array<() => SitemapEntry[]> = [() => SITE_PAGES, wikiSitemap];

// Which page shows which media, for the media sitemaps: each wiki page's clips and infobox sprite.
export const PAGE_MEDIA_SOURCES: Array<(media: WikiMedia) => PageMedia[]> = [wikiPageMedia];

export function sitemapXml(sources = SITEMAP_SOURCES): string {
  const entries = sources.flatMap((source) => source());
  const seen = new Set<string>();
  const urls = entries.map(({ path, lastmod }) => {
    if (seen.has(path)) throw new Error(`the sitemap has ${path} twice`);
    seen.add(path);
    const mod = lastmod === undefined ? "" : `    <lastmod>${escapeXml(lastmod)}</lastmod>\n`;
    return url(path, mod);
  });
  return urlset("", urls);
}

// The paths /sitemap.xml lists.
export function sitemapPaths(sources = SITEMAP_SOURCES): string[] {
  return sources.flatMap((source) => source()).map(({ path }) => path);
}

const VIDEO_NS = 'xmlns:video="http://www.google.com/schemas/sitemap-video/1.1"';
const IMAGE_NS = 'xmlns:image="http://www.google.com/schemas/sitemap-image/1.1"';

// https://developers.google.com/search/docs/crawling-indexing/sitemaps/video-sitemaps: a <video:video> per
// clip, with what its VideoObject says (media.ts's clipVideo).
export function videoSitemapXml(
  media: WikiMedia | null,
  pages = sitemapPaths(),
  sources = PAGE_MEDIA_SOURCES,
): string {
  if (!media) return urlset(` ${VIDEO_NS}`, []);
  const urls = mediaOnPages(media, pages, sources)
    .filter((page) => page.clips.length > 0)
    .map(({ path, clips }) => {
      const videos = clips.map((id) => {
        const clip = mediaClip(media, id);
        return (
          "    <video:video>\n" +
          tag(6, "video:thumbnail_loc", absoluteUrl(mediaUrl(media, `clips/${clip.poster}`))) +
          tag(6, "video:title", clip.title) +
          // Google's limit, in characters, not UTF-16's halves of one.
          tag(6, "video:description", Array.from(clip.description).slice(0, 2048).join("")) +
          tag(6, "video:content_loc", absoluteUrl(mediaUrl(media, `clips/${clip.mp4}`))) +
          tag(6, "video:duration", String(durationSeconds(clip.duration))) +
          tag(6, "video:publication_date", clip.date) +
          "    </video:video>\n"
        );
      });
      return url(path, videos.join(""));
    });
  return urlset(` ${VIDEO_NS}`, urls);
}

// https://developers.google.com/search/docs/crawling-indexing/sitemaps/image-sitemaps: an <image:image> per
// sprite, upscaled as the page shows it, animated or not (media.ts's spriteImage). Only <image:loc>: Google
// ignores the rest.
export function imageSitemapXml(
  media: WikiMedia | null,
  pages = sitemapPaths(),
  sources = PAGE_MEDIA_SOURCES,
): string {
  if (!media) return urlset(` ${IMAGE_NS}`, []);
  const urls = mediaOnPages(media, pages, sources)
    .filter((page) => page.sprites.length > 0)
    .map(({ path, sprites }) => {
      const images = sprites.map(({ name, anim }) => {
        const loc = absoluteUrl(mediaUrl(media, `sprites/${spriteFile(mediaSprite(media, name), anim)}`));
        return `    <image:image>\n${tag(6, "image:loc", loc)}    </image:image>\n`;
      });
      return url(path, images.join(""));
    });
  return urlset(` ${IMAGE_NS}`, urls);
}

// robots.txt (src/routes/robots[.]txt.ts): everything allowed, and each sitemap with a page in it. The
// sitemaps' schema wants a <url> at least, so an empty one isn't named, though it's served.
export function robotsTxt(media: WikiMedia | null, pages = sitemapPaths(), sources = PAGE_MEDIA_SOURCES): string {
  const on = mediaOnPages(media, pages, sources);
  const sitemaps = [
    "/sitemap.xml",
    ...(on.some((page) => page.clips.length > 0) ? ["/sitemap-video.xml"] : []),
    ...(on.some((page) => page.sprites.length > 0) ? ["/sitemap-image.xml"] : []),
  ];
  return `User-agent: *\nAllow: /\n\n${sitemaps.map((path) => `Sitemap: ${SITE_URL}${path}\n`).join("")}`;
}

function urlset(namespaces: string, urls: string[]): string {
  return (
    '<?xml version="1.0" encoding="UTF-8"?>\n' +
    `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"${namespaces}>\n` +
    urls.join("") +
    "</urlset>\n"
  );
}

function url(path: string, children: string): string {
  return `  <url>\n    <loc>${escapeXml(absoluteUrl(path))}</loc>\n${children}  </url>\n`;
}

function tag(indent: number, name: string, text: string): string {
  return `${" ".repeat(indent)}<${name}>${escapeXml(text)}</${name}>\n`;
}

function escapeXml(text: string): string {
  return text.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}
