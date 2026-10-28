// The wiki's Open Graph images: a 1200x630 card per page and hub, made at build time from the pinned media
// (docs/web-deploy.md, "Open Graph images"). Everything on it is the game's own pixel art, nearest neighbour
// at whole scales: the page's sprite, the SPELUNKY title sprite, and the text in the game's sprite fonts
// (sFont and sFontSmall, which it draws with font_add_sprite: monospaced, ' ' to 'Z', so upper case), on the
// wiki's cave palette and stone borders (src/wiki/wiki.css), with the line docs/wiki-license.md asks for,
// "Unofficial fan wiki · spelunky.ai".
//
// A card's file is named by a hash of its bytes, /wiki/og/<slug>.<hash>.png, so it's served from the asset
// store and cached for good, like the media (wrangler.jsonc, public/_headers), and a page whose card changes
// gets a new URL, which a social site that cached the old one fetches afresh.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { MediaPackage } from "../../seo/media-package.ts";
import type { SeoImage } from "../../seo/seo.ts";
import { decodePng, encodePng } from "./png.ts";
import type { Rgba } from "./png.ts";

export const OG_WIDTH = 1200;
export const OG_HEIGHT = 630;
// Where the cards are served: no page may have this slug (schema.ts's RESERVED_SLUGS).
export const OG_DIR = "/wiki/og";

// What a card shows.
export interface OgCard {
  // The line above the title: a page's categories, "Monsters".
  kicker: string;
  title: string;
  // [label, value]: a page's infobox's first numbers (cards.ts's keyStats), a hub's counts.
  stats: [string, string][];
  // A sprite in the media, its first frame shown (or `frame`), or null for none.
  sprite: string | null;
  frame?: number;
}

// A card made: its image for seo() and the file to write at its path.
export interface OgImage {
  image: SeoImage;
  png: Buffer;
}

type Colour = readonly [number, number, number];
const hex = (h: string): Colour => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];
// wiki.css's.
const ROCK = hex("#1b1410");
const DIRT = hex("#4a2a1a");
const DIRT_LIGHT = hex("#7a4526");
const GOLD = hex("#f0c040");
const GOLD_DARK = hex("#a8781c");
const GOLD_LIGHT = hex("#fff3b0");
const BADGE_TEXT = hex("#2a1a08");
const TEXT = hex("#e8dcc8");
const DIM = hex("#a89880");
const PANEL = hex("#0d0907");

// The media's PNGs' notice (docs/wiki-media.md), as the sprites on the card carry it.
const NOTICE =
  "Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC, and is used under the Spelunky User License v1.1b " +
  "(https://spelunky.ai/LICENSE.txt). Unofficial; not affiliated with or endorsed by Mossmouth.";
const FOOTER = "Unofficial fan wiki · spelunky.ai";

// The fonts' range: font_add_sprite(sFont, ord(' '), ...), one frame a character, ' ' to 'Z'.
const FIRST_CHAR = 32;
const LAST_CHAR = 90;
// The most characters of a stat's label shown, so the value keeps most of the row.
const LABEL_MOST = 12;
// What the fonts haven't got, as near as they have it. Anything else not in them fails the build, naming it.
const SUBSTITUTES: Record<string, string> = {
  "–": "-",
  "—": "-",
  "‘": "'",
  "’": "'",
  "“": '"',
  "”": '"',
  "×": "X",
  "…": "...",
  " ": " ",
};
// A middle dot, which the fonts haven't got either: their full stop, raised to the middle of a capital.
const MIDDLE_DOT = "·";
// The title's scales and how many lines each may take, biggest first.
const TITLE_SIZES = [
  [4, 2],
  [3, 3],
  [2, 4],
] as const;

interface Font {
  strip: Rgba;
  width: number;
  height: number;
  count: number;
  // How far to raise the full stop to make a middle dot.
  dotRaise: number;
}

// Renders cards from the media package (media-package.ts). Throws if it hasn't got the fonts or the title.
export function ogRenderer(media: MediaPackage) {
  const sprite = (name: string) => {
    if (!Object.hasOwn(media.sprites, name)) throw new Error(`og: no sprite ${name} in the wiki media`);
    return media.sprites[name];
  };
  const png = (path: string) => decodePng(readFileSync(join(media.dir, "sprites", path)), `og: sprites/${path}`);
  const font = (name: string): Font => {
    const s = sprite(name);
    const frames = s.frames.length;
    // gameText's range, which a card's text is checked against.
    if (frames !== LAST_CHAR - FIRST_CHAR + 1) throw new Error(`og: ${name} has ${frames} frames, not one for each of ' ' to 'Z'`);
    if (s.strip === null) throw new Error(`og: ${name} has no pixels in the wiki media`);
    const strip = png(s.strip);
    if (strip.width !== s.width * frames || strip.height !== s.height) throw new Error(`og: ${name}'s strip isn't ${frames} frames of ${s.width}x${s.height}`);
    const f = { strip, width: s.width, height: s.height, count: frames, dotRaise: 0 };
    const rows = (c: string) => opaqueBox(strip, (c.charCodeAt(0) - FIRST_CHAR) * s.width, 0, s.width, s.height);
    const dot = rows(".");
    const capital = rows("A");
    if (dot && capital) f.dotRaise = Math.round(dot.y + dot.h / 2 - (capital.y + capital.h / 2));
    return f;
  };
  const big = font("sFont");
  const small = font("sFontSmall");
  // A sprite's frame (its first, unless said), or null if it has no pixels (wiki-sprites writes no file). A
  // hub's card often shows a page's sprite again.
  const frames = new Map<string, Rgba | null>();
  const frame = (name: string, i = 0): Rgba | null => {
    const key = `${name}/${i}`;
    if (!frames.has(key)) {
      const meta = sprite(name).frames[i];
      if (meta === undefined) throw new Error(`og: ${name} has no frame ${i} in the wiki media`);
      frames.set(key, meta.file === null ? null : png(meta.file));
    }
    return frames.get(key)!;
  };
  const title = frame("sTitle");
  // The masthead's height: the title at 3x, 128x16 in the game.
  if (!title || title.height * 3 > 48 || title.width * 3 > 600) throw new Error("og: sTitle isn't a picture of at most 200x16");

  // What every card has: the cave wall, the masthead, the footer.
  const chrome = new Canvas(OG_WIDTH, OG_HEIGHT);
  background(chrome);
  // The masthead: the title sprite and a "fan wiki" badge, as the wiki's header has them.
  chrome.image(title, 0, 0, title.width, title.height, 40, 32 + (48 - title.height * 3) / 2, 3);
  const badge = 40 + title.width * 3 + 24;
  chrome.fill(badge, 32, 216, 48, GOLD);
  chrome.fill(badge, 32, 216, 4, GOLD_LIGHT);
  chrome.fill(badge, 32, 4, 48, GOLD_LIGHT);
  chrome.fill(badge, 76, 216, 4, GOLD_DARK);
  chrome.fill(badge + 212, 32, 4, 48, GOLD_DARK);
  chrome.text(small, gameText("Fan wiki", "og"), badge + 12, 44, 3, BADGE_TEXT);
  chrome.fill(40, 104, 1120, 4, DIRT);
  // The footer, the license's line.
  chrome.fill(8, 548, 1184, 74, DIRT);
  chrome.fill(8, 548, 1184, 4, DIRT_LIGHT);
  const footer = gameText(FOOTER, "og");
  chrome.text(small, footer, (OG_WIDTH - footer.length * small.width * 3) / 2, 573, 3, TEXT);

  return function render(card: OgCard, id: string): OgImage {
    const what = `og: ${id}`;
    const canvas = new Canvas(OG_WIDTH, OG_HEIGHT);
    canvas.rgb.set(chrome.rgb);

    // The sprite in a stone-framed panel, and the text beside it; without one, the text has the width.
    const image = card.sprite === null ? null : frame(card.sprite, card.frame);
    const box = image && opaqueBox(image, 0, 0, image.width, image.height);
    let left = 40;
    if (image && box) {
      canvas.fill(40, 132, 376, 376, DIRT);
      canvas.fill(44, 136, 368, 368, DIRT_LIGHT);
      canvas.fill(48, 140, 360, 360, PANEL);
      const scale = Math.max(1, Math.min(20, Math.floor(320 / Math.max(box.w, box.h))));
      const x = 48 + Math.floor((360 - box.w * scale) / 2);
      const y = 140 + Math.floor((360 - box.h * scale) / 2);
      canvas.image(image, box.x, box.y, box.w, box.h, x, y, scale, null, { x: 48, y: 140, w: 360, h: 360 });
      left = 448;
    }
    const width = 1160 - left;
    canvas.text(small, truncate(gameText(card.kicker, what), Math.floor(width / (small.width * 3))), left, 140, 3, DIM);
    // The title as big as it fits: 4x on two lines, 3x on three, 2x on four (cut short past that).
    const name = gameText(card.title, what);
    let y = 184;
    let scale = 2;
    let lines: string[] = [];
    for (const [s, most] of TITLE_SIZES) {
      const perLine = Math.floor(width / (big.width * s));
      lines = wrap(name, perLine);
      scale = s;
      if (lines.length <= most) break;
      if (s === 2) lines = [...lines.slice(0, most - 1), truncate(lines.slice(most - 1).join(" "), perLine)];
    }
    for (const line of lines) {
      canvas.text(big, line, left, y, scale, GOLD);
      y += big.height * scale + 8;
    }
    // The stats, a row each: the label small (cut short past LABEL_MOST), the value in the big font at 2x.
    y += 20;
    const labels = card.stats.map(([label]) => truncate(gameText(label, what), LABEL_MOST));
    const labelWidth = (Math.max(0, ...labels.map((l) => l.length)) + 1) * small.width * 3;
    for (const [i, [, value]] of card.stats.entries()) {
      if (y + big.height * 2 > 520) break;
      canvas.text(small, labels[i], left, y + 4, 3, DIM);
      canvas.text(big, truncate(gameText(value, what), Math.floor((width - labelWidth) / (big.width * 2))), left + labelWidth, y, 2, TEXT);
      y += big.height * 2 + 8;
    }

    // Named by its bytes, which are what a cache keeps: the pixels, the notice and zlib's output.
    const file = encodePng(OG_WIDTH, OG_HEIGHT, canvas.rgb, { Copyright: NOTICE });
    const hash = createHash("sha256").update(file).digest("hex").slice(0, 16);
    const stats = card.stats.map(([label, value]) => `${label} ${value}`).join(", ");
    return {
      image: {
        path: `${OG_DIR}/${id}.${hash}.png`,
        width: OG_WIDTH,
        height: OG_HEIGHT,
        alt: `${card.title}: ${card.kicker}.${stats ? ` ${stats}.` : ""} ${FOOTER}`,
      },
      png: file,
    };
  };

  // The cave wall (wiki.css's body): rock with a faint checker of 8-pixel tiles, in stone borders.
  function background(c: Canvas) {
    const light = ROCK.map((v) => v + Math.floor(((255 - v) * 6 + 127) / 255)) as unknown as Colour;
    for (let y = 0; y < c.height; y += 8) {
      for (let x = 0; x < c.width; x += 8) c.fill(x, y, 8, 8, ((x + y) / 8) % 2 === 0 ? light : ROCK);
    }
    c.fill(0, 0, c.width, 4, DIRT);
    c.fill(0, c.height - 4, c.width, 4, DIRT);
    c.fill(0, 0, 4, c.height, DIRT);
    c.fill(c.width - 4, 0, 4, c.height, DIRT);
    c.fill(4, 4, c.width - 8, 4, DIRT_LIGHT);
    c.fill(4, c.height - 8, c.width - 8, 4, DIRT_LIGHT);
    c.fill(4, 4, 4, c.height - 8, DIRT_LIGHT);
    c.fill(c.width - 8, 4, 4, c.height - 8, DIRT_LIGHT);
  }
}

// Text as the fonts can draw it: upper case, with SUBSTITUTES. Throws on a character they haven't got.
export function gameText(text: string, what: string): string {
  let out = "";
  for (const c of text.toUpperCase()) {
    const s = SUBSTITUTES[c] ?? c;
    const code = s.charCodeAt(0);
    if (s !== MIDDLE_DOT && (code < FIRST_CHAR || code > LAST_CHAR)) {
      throw new Error(`${what}: "${text}" has ${JSON.stringify(c)}, which the game's font hasn't got: add a substitute to og.ts's SUBSTITUTES`);
    }
    out += s;
  }
  return out;
}

// Words into lines of at most `most` characters; a word longer than a line is broken.
function wrap(text: string, most: number): string[] {
  const lines: string[] = [];
  let line = "";
  for (let word of text.split(/ +/).filter(Boolean)) {
    while (word.length > most) {
      if (line) lines.push(line);
      lines.push(word.slice(0, most));
      word = word.slice(most);
      line = "";
    }
    if (!line) line = word;
    else if (line.length + 1 + word.length <= most) line += ` ${word}`;
    else {
      lines.push(line);
      line = word;
    }
  }
  if (line) lines.push(line);
  return lines;
}

const truncate = (text: string, most: number) =>
  text.length <= most ? text : most < 4 ? text.slice(0, Math.max(0, most)) : `${text.slice(0, most - 3).trimEnd()}...`;

// The smallest box around a region's pixels that aren't fully transparent, or null if it has none.
function opaqueBox(image: Rgba, x0: number, y0: number, w: number, h: number) {
  let left = Infinity;
  let top = Infinity;
  let right = -1;
  let bottom = -1;
  for (let y = y0; y < y0 + h; y++) {
    for (let x = x0; x < x0 + w; x++) {
      if (image.data[(y * image.width + x) * 4 + 3] === 0) continue;
      left = Math.min(left, x);
      right = Math.max(right, x);
      top = Math.min(top, y);
      bottom = Math.max(bottom, y);
    }
  }
  return right < 0 ? null : { x: left, y: top, w: right - left + 1, h: bottom - top + 1 };
}

// An opaque RGB picture, drawn on with whole pixels and integer arithmetic only, so it's the same everywhere.
class Canvas {
  readonly width: number;
  readonly height: number;
  readonly rgb: Uint8Array;
  constructor(width: number, height: number) {
    this.width = width;
    this.height = height;
    this.rgb = new Uint8Array(width * height * 3);
  }

  fill(x: number, y: number, w: number, h: number, [r, g, b]: Colour) {
    for (let yy = Math.max(0, y); yy < Math.min(this.height, y + h); yy++) {
      for (let xx = Math.max(0, x); xx < Math.min(this.width, x + w); xx++) {
        const o = (yy * this.width + xx) * 3;
        this.rgb[o] = r;
        this.rgb[o + 1] = g;
        this.rgb[o + 2] = b;
      }
    }
  }

  // A region of an image at (x, y), each pixel `scale` by `scale` (nearest neighbour), over what's there by
  // its alpha. With `tint`, multiplied by it as GM's draw_set_color tints a sprite font: white becomes the
  // colour and black stays black. `clip` bounds what it draws.
  image(src: Rgba, sx: number, sy: number, sw: number, sh: number, x: number, y: number, scale: number, tint: Colour | null = null, clip = { x: 0, y: 0, w: this.width, h: this.height }) {
    for (let py = 0; py < sh; py++) {
      for (let px = 0; px < sw; px++) {
        const i = ((sy + py) * src.width + sx + px) * 4;
        const a = src.data[i + 3];
        if (a === 0) continue;
        const r = tint ? Math.floor((src.data[i] * tint[0] + 127) / 255) : src.data[i];
        const g = tint ? Math.floor((src.data[i + 1] * tint[1] + 127) / 255) : src.data[i + 1];
        const b = tint ? Math.floor((src.data[i + 2] * tint[2] + 127) / 255) : src.data[i + 2];
        for (let yy = y + py * scale; yy < y + (py + 1) * scale; yy++) {
          if (yy < clip.y || yy >= clip.y + clip.h) continue;
          for (let xx = x + px * scale; xx < x + (px + 1) * scale; xx++) {
            if (xx < clip.x || xx >= clip.x + clip.w) continue;
            const o = (yy * this.width + xx) * 3;
            this.rgb[o] = Math.floor((r * a + this.rgb[o] * (255 - a) + 127) / 255);
            this.rgb[o + 1] = Math.floor((g * a + this.rgb[o + 1] * (255 - a) + 127) / 255);
            this.rgb[o + 2] = Math.floor((b * a + this.rgb[o + 2] * (255 - a) + 127) / 255);
          }
        }
      }
    }
  }

  // Text in a sprite font, already gameText's, its top left at (x, y).
  text(font: Font, text: string, x: number, y: number, scale: number, tint: Colour) {
    for (const [i, c] of [...text].entries()) {
      const dot = c === MIDDLE_DOT;
      const frame = (dot ? 46 : c.charCodeAt(0)) - FIRST_CHAR;
      if (frame < 0 || frame >= font.count) continue;
      const left = x + i * font.width * scale;
      this.image(font.strip, frame * font.width, 0, font.width, font.height, left, y - (dot ? font.dotRaise * scale : 0), scale, tint);
    }
  }
}
