// The wiki's Open Graph images (src/wiki/build/og.ts, docs/web-deploy.md, "Open Graph images"), which
// site/test.sh runs:
//
//   node test-og.mjs stand-in MEDIA          add to test.sh's stand-in media, at MEDIA, the
//                                            sprites the cards draw: the fonts, the title, the Spelunker,
//                                            and the Snake's first frame (made-up pixels, real PNGs)
//   node test-og.mjs check DIST SITE MEDIA URL
//                                            check the built site in DIST, served at SITE, built with MEDIA
//                                            (served at URL, /wiki/media)
//
// Every pre-rendered wiki page and hub (not the source viewer's pages, noindex code with the site's default
// image: an image each would be 1,140 more files) must name its own image, at 1200x630 with its alt text, in og:image
// and twitter:image; each must be served as a PNG of that size, cached for good, with the bytes the build
// wrote; no other file may be in /wiki/og/; compiling the wiki again must make the same files; and a page
// naming a sprite without pixels must fail the build.
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";
import { crc32, deflateSync } from "node:zlib";

const SITE_URL = "https://spelunky.ai";
const [mode, ...args] = process.argv.slice(2);
let failures = 0;
function fail(message) {
  console.error(`FAIL ${message}`);
  failures++;
}

if (mode === "stand-in" && args.length === 1) standIn(args[0]);
else if (mode === "check" && args.length === 4) await check(...args);
else {
  console.error("usage: node test-og.mjs stand-in MEDIA | check DIST SITE MEDIA URL");
  process.exit(2);
}
if (failures > 0) {
  console.error(`test-og.mjs: ${failures} failed`);
  process.exit(1);
}
console.log(`test-og.mjs: ${mode}: all passed`);

function standIn(media) {
  // A font: a frame per character from ' ' to 'Z', each a white block in a black outline, its height the
  // character's (so the full stop is low, and the middle dot is raised from it).
  const font = (name, size) => {
    const frames = 59;
    const pixels = image(size * frames, size, (x, y) => {
      const c = Math.floor(x / size);
      const top = c === ".".charCodeAt(0) - 32 ? size - 4 : 1;
      const [gx, gy] = [x % size, y];
      if (c === 0 || gx >= size - 1 || gy < top || gy >= size - 1) return null;
      const edge = gx === 0 || gx === size - 2 || gy === top || gy === size - 2;
      return edge ? [0, 0, 0, 255] : [255, 255, 255, 255];
    });
    sprite(name, { strip: `${name}/strip.png`, frames: Array(frames).fill({ file: null, file_scaled: null }), anim_scaled: null }, {
      [`${name}/strip.png`]: pixels,
    });
  };
  font("sFont", 16);
  font("sFontSmall", 8);
  const frame = (name, w, h, colour) =>
    sprite(name, { frames: [{ file: `${name}/frames/0.png`, file_scaled: null }], anim_scaled: null }, {
      [`${name}/frames/0.png`]: image(w, h, (x, y) => (x > 0 && y > 0 && x < w - 1 && y < h - 1 ? colour : null)),
    });
  frame("sTitle", 128, 16, [150, 140, 100, 255]);
  frame("sStandLeft", 16, 16, [200, 120, 40, 255]);
  // The pages' sprites' sprite.json are test.sh's: each of their frames, as a page's card can show any
  // (`card: { sprite, frame }`), the Snake's green.
  for (const { meta } of JSON.parse(readFileSync(join(media, "sprites/index.json"), "utf8")).sprites) {
    for (const { file } of JSON.parse(readFileSync(join(media, "sprites", meta), "utf8")).frames) {
      if (!file || existsSync(join(media, "sprites", file))) continue;
      const colour = file.startsWith("sSnakeWalkL/") ? [60, 160, 40, 255] : [120, 120, 160, 255];
      writePng(join(media, "sprites", file), image(16, 16, (x, y) => (y > 6 ? colour : null)));
    }
  }

  function sprite(name, meta, files) {
    mkdirSync(join(media, "sprites", name, "frames"), { recursive: true });
    writeFileSync(join(media, "sprites", name, "sprite.json"), JSON.stringify(meta));
    for (const [path, pixels] of Object.entries(files)) writePng(join(media, "sprites", path), pixels);
  }
}

// An RGBA image of `at(x, y)`'s pixels, transparent where it's null.
function image(width, height, at) {
  const data = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) data.set(at(x, y) ?? [0, 0, 0, 0], (y * width + x) * 4);
  return { width, height, data };
}

// An 8-bit RGBA PNG, unfiltered.
function writePng(path, { width, height, data }) {
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) raw.set(data.subarray(y * width * 4, (y + 1) * width * 4), y * (width * 4 + 1) + 1);
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 6;
  const chunk = (type, body) => {
    const out = Buffer.alloc(12 + body.length);
    out.writeUInt32BE(body.length, 0);
    out.write(type, 4, "latin1");
    body.copy(out, 8);
    out.writeUInt32BE(crc32(out.subarray(4, 8 + body.length)), 8 + body.length);
    return out;
  };
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  writeFileSync(path, Buffer.concat([signature, chunk("IHDR", header), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]));
}

async function check(dist, site, media, mediaUrl) {
  // Imported here, as the stand-in is made before build.sh installs the packages the wiki's build needs.
  const { loadWiki } = await import("./src/wiki/build/compile.ts");
  const { gameText, OG_HEIGHT, OG_WIDTH } = await import("./src/wiki/build/og.ts");
  const { decodePng } = await import("./src/wiki/build/png.ts");
  // The pages: every pre-rendered wiki page and hub (wiki.html, wiki/*.html, wiki/items/*.html), but the source
  // viewer's (wiki/source.html, wiki/source/**).
  const client = join(dist, "client");
  const source = join(client, "wiki", "source");
  const pages = [join(client, "wiki.html"), ...htmlUnder(join(client, "wiki"))].filter(
    (file) => file !== `${source}.html` && !file.startsWith(`${source}/`),
  );
  if (pages.length < 3) fail(`${client}: only ${pages.length} wiki page(s)`);
  const named = new Map();
  for (const file of pages) {
    const html = readFileSync(file, "utf8");
    const meta = (attr, name) =>
      [...html.matchAll(new RegExp(`<meta ${attr}="${name.replace(/[:]/g, "\\$&")}" content="([^"]*)"/>`, "g"))].map((m) => m[1]);
    const where = relative(client, file);
    const [image] = meta("property", "og:image");
    const want = new RegExp(`^${SITE_URL}/wiki/og/[a-z0-9/-]+\\.[0-9a-f]{16}\\.png$`);
    if (!image || !want.test(image)) {
      fail(`${where}: og:image ${image}, not ${SITE_URL}/wiki/og/<name>.<hash>.png`);
      continue;
    }
    if (meta("name", "twitter:image")[0] !== image) fail(`${where}: twitter:image isn't og:image`);
    if (meta("property", "og:image:width")[0] !== String(OG_WIDTH)) fail(`${where}: og:image:width isn't ${OG_WIDTH}`);
    if (meta("property", "og:image:height")[0] !== String(OG_HEIGHT)) fail(`${where}: og:image:height isn't ${OG_HEIGHT}`);
    const [alt] = meta("property", "og:image:alt");
    if (!alt?.includes("Unofficial fan wiki · spelunky.ai")) fail(`${where}: og:image:alt ${alt}`);
    if (meta("name", "twitter:image:alt")[0] !== alt) fail(`${where}: twitter:image:alt isn't og:image:alt`);
    if (named.has(image)) fail(`${where}: the same image as ${named.get(image)}`);
    named.set(image, where);

    const path = image.slice(SITE_URL.length);
    const response = await fetch(site + path, { redirect: "manual" });
    const body = Buffer.from(await response.arrayBuffer());
    if (response.status !== 200) fail(`${path}: status ${response.status}`);
    if (response.headers.get("content-type") !== "image/png") fail(`${path}: content-type ${response.headers.get("content-type")}`);
    if (response.headers.get("cache-control") !== "public, max-age=31536000, immutable") fail(`${path}: cache-control ${response.headers.get("cache-control")}`);
    if (!body.equals(readFileSync(join(client, path)))) fail(`${path}: not the file the build wrote`);
    try {
      const png = decodePng(body, path);
      if (png.width !== OG_WIDTH || png.height !== OG_HEIGHT) fail(`${path}: ${png.width}x${png.height}`);
    } catch (e) {
      fail(e.message);
    }
    console.log(`ok   ${where}: ${path}`);
  }
  // Nothing in /wiki/og/ that no page names.
  for (const file of filesUnder(join(client, "wiki/og"))) {
    const url = `${SITE_URL}/${relative(client, file)}`;
    if (!named.has(url)) fail(`${relative(client, file)}: no page names it`);
  }

  // The same pages and media make the same files, as the build made them.
  const here = (path) => new URL(path, import.meta.url).pathname;
  const options = {
    contentDir: here("content/wiki"),
    statsFile: here("content/wiki-data/stats.json"),
    relationsFile: here("content/wiki-data/relations.json"),
    plannedFile: here("content/wiki-planned.yaml"),
    clipsDir: here("../wiki/clips"),
    relationsSpec: here("../wiki/data/relations.toml"),
    quirksFile: here("../docs/quirks.md"),
    repoDir: here(".."),
    commit: "main",
    media: { dir: media, url: mediaUrl, date: "2026-10-27T00:00:00Z", log: null },
    sourceDir: join(media, "gml"),
  };
  const again = await loadWiki(options);
  for (const error of again.errors) fail(`loadWiki: ${error}`);
  if (again.og.size !== named.size) fail(`loadWiki: ${again.og.size} images, but the pages name ${named.size}`);
  for (const [path, png] of again.og) {
    let built;
    try {
      built = readFileSync(join(client, path));
    } catch {
      fail(`${path}: the build didn't write it`);
      continue;
    }
    if (!png.equals(built)) fail(`${path}: compiling again made other bytes`);
  }

  // A sprite without pixels has no image to show, so a page naming one fails the build, as its infobox image
  // (the Snake's, here) or its hub card's (the Bat's): src/seo/media-package.ts's spriteWithPixels.
  const blankDir = mkdtempSync(join(tmpdir(), "test-og-"));
  try {
    cpSync(options.contentDir, blankDir, { recursive: true });
    // Each page's infobox image, as it is and with the blank sprite.
    const edits = {
      "snake.mdx": [
        '  image: { sprite: sSnakeWalkL, anim: true, alt: "A green snake with a tan belly, coiled and facing left" }',
        '  image: { sprite: sBlank, alt: "" }',
      ],
      "bat.mdx": [
        '  image: { sprite: sBatLeft, anim: true, alt: "A purple bat flapping its spread wings" }',
        '  image: { sprite: sBatLeft, anim: true, alt: "A purple bat flapping its spread wings", card: { sprite: sBlank } }',
      ],
    };
    let edited = true;
    for (const [file, [image, blank]] of Object.entries(edits)) {
      const source = readFileSync(join(blankDir, file), "utf8");
      if (!source.includes(image)) {
        fail(`${file}: its infobox image isn't the one the blank sprite's test replaces`);
        edited = false;
      }
      writeFileSync(join(blankDir, file), source.replace(image, blank));
    }
    if (edited) {
      const { errors } = await loadWiki({ ...options, contentDir: blankDir });
      const blank = "sBlank has no pixels in the wiki media, so no image to show";
      const want = [`snake.mdx: infobox.image: ${blank}`, `bat.mdx: infobox.image.card: ${blank}`];
      if (JSON.stringify(errors) !== JSON.stringify(want)) fail(`a sprite without pixels: errors ${JSON.stringify(errors)}`);
      else console.log("ok   a sprite without pixels fails the build, as an infobox image or a card's");
    }
  } finally {
    rmSync(blankDir, { recursive: true, force: true });
  }

  // Text in the game's font: upper case, with what it hasn't got substituted, and a character with no
  // substitute an error.
  if (gameText("Spider – 20–50 steps", "test") !== "SPIDER - 20-50 STEPS") fail("gameText: substitutes");
  try {
    gameText("Café", "test");
    fail("gameText: É isn't in the font, but it didn't throw");
  } catch (e) {
    if (!/hasn't got/.test(e.message)) fail(`gameText: ${e.message}`);
  }
}

function htmlUnder(dir) {
  return filesUnder(dir).filter((file) => file.endsWith(".html") && !relative(dir, file).startsWith("media/") && !relative(dir, file).startsWith("og/"));
}

function filesUnder(dir) {
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => join(entry.parentPath, entry.name));
}
