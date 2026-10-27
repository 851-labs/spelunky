// The hover cards' data (src/wiki/hover-card.tsx): for each written page, what a card previewing a link to it
// shows. Plain JSON, made once at build time like the hubs', and its own chunk (plugin.ts's
// `virtual:wiki/hover-cards`), which the browser loads the first time a link is hovered or focused.
import type { PageMeta } from "./compile.ts";

export interface HoverCard {
  title: string;
  // The page's type, as a reader names it: "Monster".
  kind: string;
  lede: string;
  // The infobox's sprite, at the size the card draws it: animated, with its first frame for
  // `prefers-reduced-motion`.
  image: { src: string; still: string | null; width: number; height: number } | null;
  // The infobox's first numbers (its `stat` and `init` rows), as [label, text]: a monster's health,
  // damage and speed.
  stats: [string, string][];
}

// The most numbers a card shows.
const CARD_STATS = 3;
// The largest a card draws a sprite: its @4x file, or a half or a quarter of it (2 or 1 screen pixels a
// game pixel), whichever fits first. A sprite over 96 game pixels (none an infobox shows) is a quarter,
// and bigger.
const CARD_IMAGE_MAX = 96;

export function hoverCards(pages: PageMeta[]): Record<string, HoverCard> {
  return Object.fromEntries(pages.map((meta) => [meta.slug, hoverCard(meta)]));
}

function hoverCard(meta: PageMeta): HoverCard {
  const { image } = meta;
  let shown: HoverCard["image"] = null;
  if (image?.src && image.width > 0 && image.height > 0) {
    const divisor = [1, 2, 4].find((d) => Math.max(image.width, image.height) / d <= CARD_IMAGE_MAX) ?? 4;
    // Animated, `still` is the first frame; still, it's `src` itself, which the card needn't name twice.
    const still = image.anim && image.still !== image.src ? image.still : null;
    shown = { src: image.src, still, width: image.width / divisor, height: image.height / divisor };
  }
  return {
    title: meta.title,
    kind: meta.type[0].toUpperCase() + meta.type.slice(1),
    lede: meta.lede,
    image: shown,
    stats: keyStats(meta),
  };
}

// A page's key numbers, for its hover card and its Open Graph image (og.ts): its infobox's first data rows.
export function keyStats(meta: PageMeta): [string, string][] {
  return meta.infobox
    .filter((row) => row.value !== undefined && row.text !== undefined)
    .slice(0, CARD_STATS)
    .map((row) => [row.label, row.text!]);
}
