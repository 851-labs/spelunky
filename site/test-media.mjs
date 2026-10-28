// The wiki media's JSON-LD and sitemaps (src/seo/media.ts, sitemap.ts), on the stand-in package site/test.sh
// makes (docs/web-deploy.md, "SEO"):
//
//   node test-media.mjs MEDIA [SITE URL]
//
// MEDIA is the unpacked package. The wiki's pages are a stand-in Snake page that shows the package's clip,
// one it hasn't got, and its sprite animated. Given SITE (a running server: wrangler dev) and URL (the path it serves MEDIA at, /wiki/media/HASH),
// every media URL they make must be served there as is, a 200 and not a redirect.
import assert from "node:assert/strict";
import { registerHooks } from "node:module";

// Node runs the TypeScript itself (it strips the types), but the sources import "./seo", as Vite resolves
// it, not "./seo.ts", and the wiki's pages from src/wiki/build/plugin.ts's virtual module.
const PAGES = [
  { slug: "snake", clips: ["snake/bite", "snake/stomp"], sprite: { name: "sSnakeWalkL", anim: true } },
  { slug: "bat", clips: [], sprite: null },
].map((page) => ({ ...page, categories: ["monsters"], verified: { date: "2026-10-27" } }));
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "virtual:wiki/pages") {
      const code = `export const pages = ${JSON.stringify(PAGES)};`;
      return { url: `data:text/javascript,${encodeURIComponent(code)}`, shortCircuit: true };
    }
    if (/^\.\.?\//.test(specifier) && !/\.[a-z]+$/.test(specifier)) {
      try {
        return nextResolve(`${specifier}.ts`, context);
      } catch {}
    }
    return nextResolve(specifier, context);
  },
});
const { readMediaPackage, spriteWithPixels, wikiMedia } = await import("./src/seo/media-package.ts");
const { clipVideo, spriteImage } = await import("./src/seo/media.ts");
const { imageSitemapXml, robotsTxt, sitemapPaths, videoSitemapXml } = await import("./src/seo/sitemap.ts");
const { absoluteUrl } = await import("./src/seo/seo.ts");

const SITE_URL = "https://spelunky.ai";
const [dir, site, url = "/wiki/media/0123456789abcdef"] = process.argv.slice(2);
if (!dir) {
  console.error("usage: node test-media.mjs MEDIA [SITE URL]");
  process.exit(2);
}
const DATE = "2026-10-27T11:16:56Z";
const pkg = readMediaPackage(dir, url, DATE);
const media = wikiMedia(pkg);
const at = (file) => `${SITE_URL}${url}/${file}`;
const RIGHTS = {
  creditText: "Derek Yu and Mossmouth, LLC (Spelunky Classic)",
  copyrightNotice: "Spelunky © 2008, 2009 Derek Yu and Mossmouth, LLC",
  license: `${SITE_URL}/LICENSE.txt`,
};

let failures = 0;
const served = new Set();
function test(name, body) {
  try {
    body();
    console.log(`ok   ${name}`);
  } catch (error) {
    console.error(`FAIL ${name}: ${error.message}`);
    failures++;
  }
}
// The text of each <TAG> in XML, unescaped as test-seo.mjs does.
const texts = (xml, tag) =>
  [...xml.matchAll(new RegExp(`<${tag}>([^<]*)</${tag}>`, "g"))].map((m) =>
    m[1].replace(/&#([0-9]+);/g, (_, n) => String.fromCodePoint(Number(n))),
  );

test("a clip's VideoObject, crediting the music it has", () => {
  const node = clipVideo(media, "snake/bite");
  assert.deepEqual(JSON.parse(JSON.stringify(node)), {
    "@context": "https://schema.org",
    "@type": "VideoObject",
    ...RIGHTS,
    creditText: "Derek Yu and Mossmouth, LLC (Spelunky Classic); music by George Buzinkai and Jonathan Perry",
    name: "Snake bite",
    description: "A snake bites the player & takes a heart.",
    thumbnailUrl: at("clips/snake/bite/poster.png"),
    contentUrl: at("clips/snake/bite/clip.mp4"),
    uploadDate: DATE,
    duration: "PT3S",
    width: 640,
    height: 480,
  });
  served.add(node.thumbnailUrl).add(node.contentUrl);
});

test("a scene's clip plays in the game: its replay, from the replay's frame of the clip's first", () => {
  // The stand-in's `from` is 40, the scene's frame: the replay boots for 241 frames before the scene's first.
  assert.deepEqual(pkg.clips["snake/bite"].live, { replay: "snake/bite/replay.txt", from: 281 });
  // The page asks for it at the package's URL, as the game fetches it (src/wiki/live-clip-player.tsx).
  served.add(at("clips/snake/bite/replay.txt"));
});

test("a sprite's ImageObject, its @ escaped", () => {
  const node = spriteImage(media, "sSnakeWalkL", { caption: "A snake" });
  assert.deepEqual(JSON.parse(JSON.stringify(node)), {
    "@context": "https://schema.org",
    "@type": "ImageObject",
    ...RIGHTS,
    name: "sSnakeWalkL",
    caption: "A snake",
    width: 64,
    height: 64,
    contentUrl: at("sprites/sSnakeWalkL/frames/0%404x.png"),
  });
  const anim = spriteImage(media, "sSnakeWalkL", { anim: true }).contentUrl;
  assert.equal(anim, at("sprites/sSnakeWalkL/anim%404x.png"));
  served.add(node.contentUrl).add(anim);
});

test("an unknown clip or sprite, or no media, fails", () => {
  assert.throws(() => clipVideo(media, "snake/nope"), /no clip snake\/nope/);
  assert.throws(() => spriteImage(media, "sNope"), /no sprite sNope/);
  // Nor is a sprite without pixels there to show, which the pages' build fails on (compile.ts).
  assert.throws(() => spriteImage(media, "sBlank"), /no sprite sBlank/);
  assert.equal(spriteWithPixels(pkg, "sBlank"), "sBlank has no pixels in the wiki media, so no image to show");
  assert.equal(spriteWithPixels(pkg, "sNope"), "no sprite sNope in the wiki media");
  assert.throws(() => clipVideo(null, "snake/bite"), /pins none/);
  assert.throws(() => readMediaPackage(dir, url, "27 October 2026"), /isn't ISO 8601/);
});

test("absoluteUrl takes escapes, not what needs one", () => {
  assert.equal(absoluteUrl("/a/0%404x.png"), `${SITE_URL}/a/0%404x.png`);
  assert.throws(() => absoluteUrl("/a b"), /percent-encode/);
});

test("the wiki's pages are in /sitemap.xml", () => {
  // With the hubs that have one of them written: the monsters'.
  assert.deepEqual(sitemapPaths(), ["/", "/play/", "/wiki", "/wiki/monsters", "/wiki/snake", "/wiki/bat"]);
});

test("no page for the media: empty sitemaps, and robots.txt names only /sitemap.xml", () => {
  const pages = ["/", "/play/", "/wiki/bat"];
  assert.equal(texts(videoSitemapXml(media, pages), "loc").length, 0);
  assert.equal(texts(imageSitemapXml(media, pages), "loc").length, 0);
  assert.equal(robotsTxt(media, pages), `User-agent: *\nAllow: /\n\nSitemap: ${SITE_URL}/sitemap.xml\n`);
  assert.equal(robotsTxt(null), robotsTxt(media, pages));
  assert.equal(texts(videoSitemapXml(null), "loc").length, 0);
});

test("the video sitemap: the Snake page's clip, not the one the media haven't got", () => {
  const xml = videoSitemapXml(media);
  assert.match(xml, /<urlset xmlns="http:\/\/www.sitemaps.org\/schemas\/sitemap\/0.9" xmlns:video="[^"]+">/);
  assert.deepEqual(texts(xml, "loc"), [`${SITE_URL}/wiki/snake`]);
  assert.deepEqual(texts(xml, "video:title"), ["Snake bite"]);
  assert.deepEqual(texts(xml, "video:description"), ["A snake bites the player & takes a heart."]);
  assert.match(xml, /player &#38; takes/);
  assert.deepEqual(texts(xml, "video:thumbnail_loc"), [at("clips/snake/bite/poster.png")]);
  assert.deepEqual(texts(xml, "video:content_loc"), [at("clips/snake/bite/clip.mp4")]);
  assert.deepEqual(texts(xml, "video:duration"), ["3"]);
  assert.deepEqual(texts(xml, "video:publication_date"), [DATE]);
});

test("the image sitemap: the Snake page's sprite, animated as it shows it", () => {
  const xml = imageSitemapXml(media);
  assert.match(xml, /xmlns:image="http:\/\/www.google.com\/schemas\/sitemap-image\/1.1"/);
  assert.deepEqual(texts(xml, "loc"), [`${SITE_URL}/wiki/snake`]);
  assert.deepEqual(texts(xml, "image:loc"), [at("sprites/sSnakeWalkL/anim%404x.png")]);
  assert.equal(
    robotsTxt(media),
    "User-agent: *\nAllow: /\n\n" +
      `Sitemap: ${SITE_URL}/sitemap.xml\nSitemap: ${SITE_URL}/sitemap-video.xml\nSitemap: ${SITE_URL}/sitemap-image.xml\n`,
  );
});

test("sources merged by page, each clip and sprite once, in the order of the pages", () => {
  const sources = [
    () => [{ path: "/wiki/snake", clips: ["snake/bite", "x/y"], sprites: [{ name: "sSnakeWalkL" }] }],
    () => [
      { path: "/wiki/snake", clips: ["snake/bite"], sprites: [{ name: "sSnakeWalkL" }, { name: "sNope" }] },
      { path: "/wiki/mines", clips: ["snake/bite"], sprites: [{ name: "sSnakeWalkL", anim: true }] },
    ],
  ];
  const pages = ["/wiki/mines", "/wiki/snake"];
  assert.deepEqual(texts(videoSitemapXml(media, pages, sources), "loc"), pages.map((p) => SITE_URL + p));
  assert.equal(texts(videoSitemapXml(media, pages, sources), "video:content_loc").length, 2);
  assert.deepEqual(texts(imageSitemapXml(media, pages, sources), "image:loc"), [
    at("sprites/sSnakeWalkL/anim%404x.png"),
    at("sprites/sSnakeWalkL/frames/0%404x.png"),
  ]);
});

if (site) {
  for (const loc of served) {
    const where = site + loc.slice(SITE_URL.length);
    const response = await fetch(where, { redirect: "manual" }).catch((error) => error);
    if (!(response instanceof Response)) {
      console.error(`FAIL ${where}: no response (${response.message})`);
      failures++;
    } else if (response.status !== 200) {
      console.error(`FAIL ${where}: status ${response.status}, not 200`);
      failures++;
    } else console.log(`ok   ${where} ${response.status}`);
    await response.body?.cancel?.();
  }
}

if (failures > 0) {
  console.error(`site/test-media.mjs: ${failures} failed`);
  process.exit(1);
}
console.log("site/test-media.mjs: all passed");
