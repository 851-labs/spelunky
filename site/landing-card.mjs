// The landing page's Open Graph image, public/media/card.png (src/seo/seo.ts's DEFAULT_IMAGE, which /play/ and any
// page without an image of its own share): 1200x630, as the wiki's cards are (src/wiki/build/og.ts), so X shows it
// whole under summary_large_image and Facebook and LinkedIn as the large card, not a thumbnail. It's the landing
// page's own pixels, scaled by whole factors with nearest neighbour so the pixel art stays sharp: its gameplay
// still twice over, and its logo twice over, on the logo's own background.
//
//   node landing-card.mjs           write public/media/card.png from public/media/gameplay.png and logo.png
//   node landing-card.mjs --check   fail unless the committed card is what they make (site/test.sh)
import { readFileSync, writeFileSync } from "node:fs";
import { decodePng, encodePng } from "./src/wiki/build/png.ts";

export const WIDTH = 1200;
export const HEIGHT = 630;
const CARD = "public/media/card.png";
// The media's notice (docs/wiki-media.md), as the wiki's cards carry it.
const NOTICE =
  "Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC, and is used under the Spelunky User License v1.1b " +
  "(https://spelunky.ai/LICENSE.txt). Unofficial; not affiliated with or endorsed by Mossmouth.";
const STILL_SCALE = 2;
const LOGO_SCALE = 2;
// Around the still: the wiki's dark gold (src/wiki/wiki.css's --gold-dark).
const BORDER = [0xa8, 0x78, 0x1c];
const BORDER_WIDTH = 4;

function card() {
  const still = decodePng(readFileSync("public/media/gameplay.png"), "gameplay.png");
  const logo = decodePng(readFileSync("public/media/logo.png"), "logo.png");
  const rgb = new Uint8Array(WIDTH * HEIGHT * 3);
  const background = logo.data.subarray(0, 3);
  for (let i = 0; i < rgb.length; i += 3) rgb.set(background, i);
  const fill = (x0, y0, w, h, colour) => {
    for (let y = y0; y < y0 + h; y++) for (let x = x0; x < x0 + w; x++) rgb.set(colour, (y * WIDTH + x) * 3);
  };
  const draw = (image, scale, x0, y0) => {
    for (let y = 0; y < image.height * scale; y++) {
      for (let x = 0; x < image.width * scale; x++) {
        const from = (Math.floor(y / scale) * image.width + Math.floor(x / scale)) * 4;
        rgb.set(image.data.subarray(from, from + 3), ((y0 + y) * WIDTH + x0 + x) * 3);
      }
    }
  };
  // The still on the left, centred top to bottom; the logo centred in the rest.
  const stillW = still.width * STILL_SCALE;
  const stillH = still.height * STILL_SCALE;
  const stillY = (HEIGHT - stillH) / 2;
  const stillX = stillY;
  fill(stillX - BORDER_WIDTH, stillY - BORDER_WIDTH, stillW + 2 * BORDER_WIDTH, stillH + 2 * BORDER_WIDTH, BORDER);
  draw(still, STILL_SCALE, stillX, stillY);
  const rest = stillX + stillW + BORDER_WIDTH;
  const logoW = logo.width * LOGO_SCALE;
  const logoH = logo.height * LOGO_SCALE;
  draw(logo, LOGO_SCALE, rest + Math.floor((WIDTH - rest - logoW) / 2), Math.floor((HEIGHT - logoH) / 2));
  return encodePng(WIDTH, HEIGHT, rgb, { Copyright: NOTICE });
}

const made = card();
if (process.argv[2] === "--check") {
  if (!made.equals(readFileSync(CARD))) {
    console.error(`landing-card.mjs: ${CARD} isn't what gameplay.png and logo.png make: run node landing-card.mjs`);
    process.exit(1);
  }
  console.log(`landing-card.mjs: ${CARD} is current`);
} else {
  writeFileSync(CARD, made);
  console.log(`landing-card.mjs: wrote ${CARD} (${made.length} bytes)`);
}
