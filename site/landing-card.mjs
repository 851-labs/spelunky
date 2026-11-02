// The landing page's Open Graph image, public/media/card.png (src/seo/seo.ts's DEFAULT_IMAGE, which /play/ and any
// page without an image of its own share): 1200x630, as the wiki's cards are (src/wiki/build/og.ts), so X shows it
// whole under summary_large_image and Facebook and LinkedIn as the large card, not a thumbnail. It's the landing
// page's own pixels, scaled by whole factors with nearest neighbour so the pixel art stays sharp: its gameplay
// still and its logo, twice over each, on the logo's own background.
//
//   node landing-card.mjs           write public/media/card.png from public/media/gameplay.png and logo.png
//   node landing-card.mjs --check   fail unless the committed card is what they make (site/test.sh)
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { NOTICE, OG_HEIGHT, OG_WIDTH } from "./src/wiki/build/og.ts";
import { decodePng, encodePng } from "./src/wiki/build/png.ts";

const here = (path) => fileURLToPath(new URL(path, import.meta.url));
const CARD = here("public/media/card.png");
const STILL_SCALE = 2;
const LOGO_SCALE = 2;
// Around the still: the wiki's dark gold (src/wiki/wiki.css's --gold-dark).
const BORDER = [0xa8, 0x78, 0x1c];
const BORDER_WIDTH = 4;

function opaque(file) {
  const image = decodePng(readFileSync(here(`public/media/${file}`)), file);
  for (let i = 3; i < image.data.length; i += 4) {
    if (image.data[i] !== 255) throw new Error(`public/media/${file} has transparent pixels, which the card can't blend`);
  }
  return image;
}

function card() {
  const still = opaque("gameplay.png");
  const logo = opaque("logo.png");
  const rgb = new Uint8Array(OG_WIDTH * OG_HEIGHT * 3);
  const background = logo.data.subarray(0, 3);
  for (let i = 0; i < rgb.length; i += 3) rgb.set(background, i);
  const box = (what, x0, y0, w, h) => {
    if (x0 < 0 || y0 < 0 || x0 + w > OG_WIDTH || y0 + h > OG_HEIGHT) throw new Error(`the ${what} doesn't fit on the card`);
  };
  const fill = (x0, y0, w, h, colour) => {
    box("still's border", x0, y0, w, h);
    for (let y = y0; y < y0 + h; y++) for (let x = x0; x < x0 + w; x++) rgb.set(colour, (y * OG_WIDTH + x) * 3);
  };
  const draw = (what, image, scale, x0, y0) => {
    box(what, x0, y0, image.width * scale, image.height * scale);
    for (let y = 0; y < image.height * scale; y++) {
      for (let x = 0; x < image.width * scale; x++) {
        const from = (Math.floor(y / scale) * image.width + Math.floor(x / scale)) * 4;
        rgb.set(image.data.subarray(from, from + 3), ((y0 + y) * OG_WIDTH + x0 + x) * 3);
      }
    }
  };
  // The still on the left, centred top to bottom and as far from the left; the logo centred in the rest.
  const stillW = still.width * STILL_SCALE;
  const stillH = still.height * STILL_SCALE;
  const stillY = Math.floor((OG_HEIGHT - stillH) / 2);
  const stillX = stillY;
  fill(stillX - BORDER_WIDTH, stillY - BORDER_WIDTH, stillW + 2 * BORDER_WIDTH, stillH + 2 * BORDER_WIDTH, BORDER);
  draw("still", still, STILL_SCALE, stillX, stillY);
  const rest = stillX + stillW + BORDER_WIDTH;
  const logoW = logo.width * LOGO_SCALE;
  const logoH = logo.height * LOGO_SCALE;
  draw("logo", logo, LOGO_SCALE, rest + Math.floor((OG_WIDTH - rest - logoW) / 2), Math.floor((OG_HEIGHT - logoH) / 2));
  return encodePng(OG_WIDTH, OG_HEIGHT, rgb, { Copyright: NOTICE });
}

const made = card();
if (process.argv[2] === "--check") {
  if (!made.equals(readFileSync(CARD))) {
    console.error("landing-card.mjs: public/media/card.png isn't what gameplay.png and logo.png make: run node landing-card.mjs");
    process.exit(1);
  }
  console.log("landing-card.mjs: public/media/card.png is current");
} else {
  writeFileSync(CARD, made);
  console.log(`landing-card.mjs: wrote public/media/card.png (${made.length} bytes)`);
}
