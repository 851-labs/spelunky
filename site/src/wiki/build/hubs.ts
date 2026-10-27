// The hubs' contents (src/wiki/hub.tsx): /wiki's featured pages and categories, and each category hub's grid
// of cards. A hub lists the pages that are written and, dimmed, the ones to come that a link may point at
// (wiki-data's entities, areas, special levels and shops, and the planned pages: relations.ts), so it shows
// the wiki's whole scope and what's left to write. Plain JSON, made once at build time like the pages' navboxes.
import { CATEGORIES, HUBS, hubById, inHub } from "./categories.ts";
import type { Category, HubId } from "./categories.ts";
import type { SeoImage } from "../../seo/seo.ts";
import type { PageMeta } from "./compile.ts";
import type { PageType } from "./schema.ts";

// A sprite as a card shows it: its first frame's upscaled file, at the size it's drawn.
export interface CardImage {
  src: string;
  width: number;
  height: number;
}

export interface HubCard {
  slug: string;
  title: string;
  // Its page, or null while it isn't written: a dimmed card, not a link, as a red link isn't.
  href: string | null;
  image: CardImage | null;
  // The areas it's found in, by slug, which a hub filters by: a page's `found_in`'s, and for one to come,
  // where wiki-data says generation makes its objects.
  areas: string[];
  // Its categories, which /wiki/items filters by.
  categories: Category[];
}

export interface HubData {
  id: HubId;
  cards: HubCard[];
  // The areas any of its cards is found in, in the game's order, for the filter.
  areas: { slug: string; name: string }[];
  // Its Open Graph image (og.ts), or null without the media.
  og: SeoImage | null;
}

// /wiki's own: every hub with its counts and a sprite to show, and the featured pages.
export interface HomeData {
  hubs: { id: HubId; written: number; total: number; image: CardImage | null }[];
  featured: (HubCard & { lede: string })[];
  written: number;
  total: number;
  // /wiki's Open Graph image, or null without the media.
  og: SeoImage | null;
}

export interface Hubs {
  hubs: HubData[];
  home: HomeData;
  // The sprite each hub's tile on /wiki shows, by name, for its Open Graph image: the build's alone, not in
  // the JSON the pages get.
  sprites: Partial<Record<HubId, string>>;
}

// A written page as a hub needs it: its compiled meta, and the areas and special levels it's found in.
export interface HubPage {
  meta: PageMeta;
  foundIn: string[];
}

// What a hub lists besides the written pages: the pages not written yet that links may point at
// (relations.ts's targets: wiki-data's entities, places and shops, and wiki-planned.yaml's), with what says
// which hub each goes in and where it's found.
export interface Known {
  unwritten: { slug: string; title: string; type: PageType | null; objects: string[] }[];
  // wiki-data's entities' kinds, by slug: an item's category.
  kinds: Record<string, string>;
  // The areas, in the game's order.
  areas: { slug: string; name: string }[];
  // Where the data says objects are found, by slug (relations.ts's placesFound).
  foundIn: (objects: string[]) => string[];
}

// A page to come's hub: by its wiki-data kind if it's an entity, else by its type. An `item` is a carryable
// until its page says otherwise (the Teleporter's will be a weapon's: its page's categories decide, once
// it's written). One of no known type (a planned page without one) isn't listed.
const KIND_CATEGORIES: Record<string, Category> = {
  monster: "monsters",
  character: "characters",
  weapon: "items/weapons",
  equipment: "items/equipment",
  item: "items/carryables",
  treasure: "treasure",
  trap: "traps",
  mechanic: "mechanics",
};
const TYPE_CATEGORIES: Record<PageType, Category> = {
  monster: "monsters",
  item: "items/carryables",
  treasure: "treasure",
  trap: "traps",
  place: "places",
  shop: "shops",
  character: "characters",
  mechanic: "mechanics",
  guide: "guides",
  quirk: "quirks",
};

// The box a card draws its sprite in: a 16-pixel sprite at 4 screen pixels a game pixel, a bigger one at
// fewer, always a whole number (docs/wiki-style.md: integer scales only).
const CARD_BOX = 64;
const FEATURED = 6;

// `sprite` gives a sprite's first frame's upscaled file and its size in game pixels, or null without the
// media; `objectSprite` an object's own sprite, by name.
export function makeHubs(
  pages: HubPage[],
  known: Known,
  sprite: (name: string) => { src: string; width: number; height: number } | null,
  objectSprite: (object: string) => string | null,
  warnings: string[],
): Hubs {
  for (const category of CATEGORIES) if (!hubById(category)) throw new Error(`categories.ts: no hub for ${category}`);
  // Each card's sprite, by its slug.
  const spriteNames = new Map<string, string>();
  const cardImage = (slug: string, name: string | null): CardImage | null => {
    const s = name === null ? null : sprite(name);
    // A blank sprite (0 by 0) has no frames to show.
    if (!s || s.width === 0 || s.height === 0) return null;
    spriteNames.set(slug, name!);
    const scale = Math.max(1, Math.min(4, Math.floor(CARD_BOX / Math.max(s.width, s.height))));
    return { src: s.src, width: s.width * scale, height: s.height * scale };
  };
  const areaSlugs = new Set(known.areas.map((a) => a.slug));

  const cards: HubCard[] = pages.map(({ meta, foundIn }) => ({
    slug: meta.slug,
    title: meta.title,
    href: `/wiki/${meta.slug}`,
    image: cardImage(meta.slug, meta.image?.sprite ?? null),
    areas: foundIn.filter((slug) => areaSlugs.has(slug)),
    categories: [...meta.categories],
  }));
  for (const { slug, title, type, objects } of known.unwritten) {
    const kind = known.kinds[slug];
    const category = kind === undefined ? type && TYPE_CATEGORIES[type] : KIND_CATEGORIES[kind];
    if (!category) {
      // A planned page without a type has no hub, as intended; an entity of a kind we don't know should.
      if (kind !== undefined) warnings.push(`hubs: no hub for ${slug}, of wiki-data's kind ${kind}: add it to hubs.ts`);
      continue;
    }
    const image = objects.length === 0 ? null : cardImage(slug, objectSprite(objects[0]));
    // Where the data says its objects are, until its page says (a written page's found_in is curated).
    const found = new Set(known.foundIn(objects));
    const areas = [...areaSlugs].filter((area) => found.has(area));
    cards.push({ slug, title, href: null, image, areas, categories: [category] });
  }
  cards.sort(byTitle);

  const verified = new Map(pages.map(({ meta }) => [meta.slug, meta.verified.date]));
  const hubs = HUBS.map((hub): HubData => {
    const listed = cards.filter((card) => inHub(hub, card.categories));
    return {
      id: hub.id,
      cards: listed,
      areas: known.areas.filter((area) => listed.some((card) => card.areas.includes(area.slug))),
      og: null,
    };
  });

  const written = cards.filter((card) => card.href !== null);
  const ledes = new Map(pages.map(({ meta }) => [meta.slug, meta.lede]));
  // The newest pages first: what's been added or checked lately.
  const featured = [...written]
    .sort((a, b) => verified.get(b.slug)!.localeCompare(verified.get(a.slug)!) || byTitle(a, b))
    .slice(0, FEATURED)
    .map((card) => ({ ...card, lede: ledes.get(card.slug)! }));
  // A written page's sprite if any has one, so the tile shows what's there.
  const tile = (cards: HubCard[]) => cards.find((card) => card.href !== null && card.image) ?? cards.find((card) => card.image);
  const home: HomeData = {
    hubs: hubs.map(({ id, cards }) => ({
      id,
      written: cards.filter((card) => card.href !== null).length,
      total: cards.length,
      image: tile(cards)?.image ?? null,
    })),
    featured,
    written: written.length,
    total: cards.length,
    og: null,
  };
  const sprites: Hubs["sprites"] = {};
  for (const { id, cards } of hubs) {
    const card = tile(cards);
    if (card) sprites[id] = spriteNames.get(card.slug);
  }
  return { hubs, home, sprites };
}

// By title, "The Mines" under M, as an index has it.
const sortKey = (title: string) => title.replace(/^The /, "");
const byTitle = (a: { title: string }, b: { title: string }) => sortKey(a.title).localeCompare(sortKey(b.title), "en");

