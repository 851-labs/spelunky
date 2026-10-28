// The hubs' contents (src/wiki/hub.tsx): /wiki's featured pages and categories, and each category hub's grid
// of cards. A hub lists the pages that are written and, dimmed, the ones to come that a link may point at
// (wiki-data's entities, areas, special levels and shops, and the planned pages: relations.ts), so it shows
// the wiki's whole scope and what's left to write. Plain JSON, made once at build time like the pages' navboxes.
import { CATEGORIES, HUBS, hubById, inHub } from "./categories.ts";
import type { Category, HubId } from "./categories.ts";
import type { SeoImage } from "../../seo/seo.ts";
import type { PageMeta } from "./compile.ts";
import type { CardSprite, PageType } from "./schema.ts";

// A sprite as a card shows it: a frame's upscaled file (the first, unless the page says), at the size it's
// drawn.
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
  // A quirk's: the hubs of the pages it involves (its `related`), which /wiki/quirks filters by, and its
  // lede, which its card shows, as a quirk's title alone doesn't say what happens.
  involves?: HubId[];
  lede?: string;
}

export interface HubData {
  id: HubId;
  cards: HubCard[];
  // The box its cards draw their sprites in (cardBox).
  box: number;
  // The areas any of its cards is found in, in the game's order, for the filter.
  areas: { slug: string; name: string }[];
  // The hubs any of its cards involves, in /wiki's order, for /wiki/quirks's filter.
  involves: { id: HubId; name: string }[];
  // Its Open Graph image (og.ts), or null without the media.
  og: SeoImage | null;
}

// /wiki's own: every hub with its counts and a sprite to show, and the featured pages.
export interface HomeData {
  hubs: { id: HubId; written: number; total: number; image: CardImage | null }[];
  featured: (HubCard & { lede: string })[];
  // The box the featured cards draw their sprites in (cardBox).
  featuredBox: number;
  written: number;
  total: number;
  // /wiki's Open Graph image, or null without the media.
  og: SeoImage | null;
}

export interface Hubs {
  hubs: HubData[];
  home: HomeData;
  // The sprite each hub's tile on /wiki shows, by name, and its frame, for its Open Graph image: the build's
  // alone, not in the JSON the pages get.
  sprites: Partial<Record<HubId, { sprite: string; frame: number }>>;
}

// A written page as a hub needs it: its compiled meta, the areas and special levels it's found in, its
// infobox image's `card`, if it has one, and the pages it's `related` to.
export interface HubPage {
  meta: PageMeta;
  foundIn: string[];
  card: CardSprite | null;
  related: string[];
}

// What a hub lists besides the written pages: the pages not written yet that links may point at
// (relations.ts's targets: wiki-data's entities, places and shops, and wiki-planned.yaml's), with what says
// which hub each goes in and where it's found.
export interface Known {
  // A planned quirk's `related` and `lede` are its card's until its page is written.
  unwritten: { slug: string; title: string; type: PageType | null; category?: Category; objects: string[]; card?: CardSprite; related?: string[]; lede?: string }[];
  // wiki-data's entities' kinds, by slug: an item's category.
  kinds: Record<string, string>;
  // The areas, in the game's order.
  areas: { slug: string; name: string }[];
  // Where the data says objects are found, by slug (relations.ts's placesFound).
  foundIn: (objects: string[]) => string[];
}

// A page to come's hub: a planned page's `category`, else by its wiki-data kind if it's an entity, else by its
// type. An `item` is a carryable until its page says otherwise: once it's written, its page's categories
// decide. One of neither (a planned page that's only a red link) isn't listed.
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

// The cards of wiki-data's pages to come (its places, which have no object, and entities whose object's
// sprite reads badly), until they're written: then the page's infobox image says, with a `card` if the
// image's first frame isn't the one (docs/wiki-style.md, "Infobox values"). A planned page's is its `card`
// in wiki-planned.yaml. Each is a sprite the game draws there: a special level's own tiles or what it's
// named for.
export const CARD_SPRITES: Record<string, CardSprite> = {
  // The levels named for what's in them, or what's always there: Kali's head over the altar (scrRoomGen's
  // oKaliHead).
  "sacrifice-pit": { sprite: "sKaliHead1" },
};

// The most a card's sprite is drawn at, on a side: a 64-pixel sprite (Olmec) at 2 screen pixels a game pixel,
// which a phone's card still fits two to a row. /wiki's category tiles draw theirs in 64.
const CARD_BOX = 128;
const TILE_BOX = 64;

// How many screen pixels a game pixel a card draws a sprite at, its larger side `size` game pixels: a 16-pixel
// sprite at 4 (64 pixels), and half again as big for each doubling, so a 32-pixel boss at 3 (96) reads bigger
// than a 16-pixel monster, a 4-pixel gem at 7 (28) isn't a speck, and the 64-pixel Olmec at 2 (128) doesn't
// need a box four times as wide. Always a whole number (docs/wiki-style.md: integer scales only), the nearest
// to that, and no more than fits `box`.
export function cardScale(size: number, box: number): number {
  const scale = Math.round((64 * 1.5 ** Math.log2(size / 16)) / size);
  return Math.max(1, Math.min(scale, Math.floor(box / size)));
}

// The box a grid draws its cards' sprites in: its biggest sprite's, and at least a 16-pixel one's at 4x. Every
// card's is that size, so the cards line up, and it's the whole list's, so it doesn't change as the hub is
// filtered.
function cardBox(cards: HubCard[]): number {
  return Math.max(16 * cardScale(16, CARD_BOX), ...cards.map((card) => (card.image ? Math.max(card.image.width, card.image.height) : 0)));
}
const FEATURED = 6;

// `sprite` gives a sprite's frame's upscaled file and its size in game pixels, null without the media, or
// what's wrong (no such sprite or frame, or a blank one, without pixels); `objectSprite` an object's own
// sprite, by name. `cardSprites` is CARD_SPRITES, or a test's.
export function makeHubs(
  pages: HubPage[],
  known: Known,
  sprite: (name: string, frame: number) => { src: string; width: number; height: number } | string | null,
  objectSprite: (object: string) => string | null,
  errors: string[],
  warnings: string[],
  cardSprites: Record<string, CardSprite> = CARD_SPRITES,
): Hubs {
  for (const category of CATEGORIES) if (!hubById(category)) throw new Error(`categories.ts: no hub for ${category}`);
  // Each card's sprite and frame, by its slug.
  const spriteNames = new Map<string, { sprite: string; frame: number }>();
  // And its image as /wiki's category tile draws it, in a smaller box.
  const tileImages = new Map<string, CardImage>();
  // `where` names the `card` that chose the sprite or frame, for an error, or is null for the default (the
  // infobox image's, which compile.ts checks, or an object's), which shows nothing if the media haven't got it
  // or it's blank.
  const cardImage = (slug: string, name: string | null, frame: number, where: string | null): CardImage | null => {
    if (name === null) return null;
    const s = sprite(name, frame);
    if (typeof s === "string") {
      if (where !== null) errors.push(`${where}: ${s}`);
      return null;
    }
    if (!s) return null;
    spriteNames.set(slug, { sprite: name, frame });
    const sized = (box: number) => {
      const scale = cardScale(Math.max(s.width, s.height), box);
      return { src: s.src, width: s.width * scale, height: s.height * scale };
    };
    tileImages.set(slug, sized(TILE_BOX));
    return sized(CARD_BOX);
  };
  const areaSlugs = new Set(known.areas.map((a) => a.slug));

  const cards: HubCard[] = pages.map(({ meta, foundIn, card }) => ({
    slug: meta.slug,
    title: meta.title,
    href: `/wiki/${meta.slug}`,
    image: cardImage(meta.slug, card?.sprite ?? meta.image?.sprite ?? null, card?.frame ?? 0, card && `${meta.slug}.mdx: ${meta.image ? "infobox.image.card" : "card"}`),
    areas: foundIn.filter((slug) => areaSlugs.has(slug)),
    categories: [...meta.categories],
  }));
  // A page that's written says its own card; CARD_SPRITES is for those to come.
  const writtenSlugs = new Set(pages.map(({ meta }) => meta.slug));
  const unwrittenSlugs = new Set(known.unwritten.map((page) => page.slug));
  for (const slug of Object.keys(cardSprites)) {
    if (writtenSlugs.has(slug)) {
      errors.push(`hubs.ts: CARD_SPRITES has ${slug}, which has a page now: give its infobox image a card if it needs one, and remove it`);
    } else if (!unwrittenSlugs.has(slug)) {
      errors.push(`hubs.ts: CARD_SPRITES has ${slug}, which is no page to come of wiki-data's: remove it`);
    }
  }
  for (const { slug, title, type, category: given, objects, card: planned } of known.unwritten) {
    const kind = known.kinds[slug];
    const category = given ?? (kind === undefined ? type && TYPE_CATEGORIES[type] : KIND_CATEGORIES[kind]);
    // Its card's sprite: CARD_SPRITES's or wiki-planned.yaml's, else its first object's own.
    const card = cardSprites[slug] ?? planned;
    const where = cardSprites[slug] ? `hubs.ts: CARD_SPRITES.${slug}` : `wiki-planned.yaml: ${slug}.card`;
    if (given && type && TYPE_CATEGORIES[type] === given) errors.push(`wiki-planned.yaml: ${slug}.category: ${given} is its type's hub already: remove it`);
    if (!category) {
      // A planned page without a type or a category has no hub, as intended; an entity of a kind we don't
      // know should.
      if (kind !== undefined) warnings.push(`hubs: no hub for ${slug}, of wiki-data's kind ${kind}: add it to hubs.ts`);
      if (card) errors.push(`${where}: ${slug} is on no hub, so it has no card`);
      continue;
    }
    const name = card?.sprite ?? (objects.length === 0 ? null : objectSprite(objects[0]));
    if (card && name === null) errors.push(`${where}: no sprite: ${slug} has no object to take one from`);
    const image = cardImage(slug, name, card?.frame ?? 0, card ? where : null);
    // Where the data says its objects are, until its page says (a written page's found_in is curated).
    const found = new Set(known.foundIn(objects));
    const areas = [...areaSlugs].filter((area) => found.has(area));
    cards.push({ slug, title, href: null, image, areas, categories: [category] });
  }
  cards.sort(byTitle);
  // A quirk's card: what it involves, by the top-level hubs (Items for a weapon) of the cards its page
  // relates to, and its lede; a planned quirk's from its wiki-planned.yaml entry, which relations.ts has
  // checked, until its page says.
  const bySlug = new Map(cards.map((card) => [card.slug, card]));
  const topHub = (category: Category): HubId => hubById(category)?.parent ?? category;
  const quirk = (slug: string, related: string[], lede: string | undefined) => {
    const card = bySlug.get(slug)!;
    const hubs = new Set(related.flatMap((slug) => (bySlug.get(slug)?.categories ?? []).map(topHub)));
    card.involves = HUBS.map((hub) => hub.id).filter((id) => id !== "quirks" && hubs.has(id));
    if (lede !== undefined) card.lede = lede;
    return card.involves;
  };
  for (const { meta, related } of pages) if (meta.type === "quirk") quirk(meta.slug, related, meta.lede);
  for (const { slug, type, related, lede } of known.unwritten) {
    // A planned quirk involving nothing on a hub would be hidden by every filter, the one thing its
    // `related` is for.
    if (type === "quirk" && bySlug.has(slug) && related && quirk(slug, related, lede).length === 0) {
      errors.push(`wiki-planned.yaml: ${slug}.related: none of its pages is on a hub, so /wiki/quirks's filter hides it`);
    }
  }

  const verified = new Map(pages.map(({ meta }) => [meta.slug, meta.verified.date]));
  const hubs = HUBS.map((hub): HubData => {
    const listed = cards.filter((card) => inHub(hub, card.categories));
    return {
      id: hub.id,
      cards: listed,
      box: cardBox(listed),
      areas: known.areas.filter((area) => listed.some((card) => card.areas.includes(area.slug))),
      involves: HUBS.filter((h) => listed.some((card) => card.involves?.includes(h.id))).map((h) => ({ id: h.id, name: h.name })),
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
  // Each hub's tile shows a written page's sprite if any has one, so it shows what's there.
  const tiles = hubs.map(({ cards }) => cards.find((card) => card.href !== null && card.image) ?? cards.find((card) => card.image));
  const home: HomeData = {
    hubs: hubs.map(({ id, cards }, i) => ({
      id,
      written: cards.filter((card) => card.href !== null).length,
      total: cards.length,
      image: (tiles[i] && tileImages.get(tiles[i].slug)) ?? null,
    })),
    featured,
    featuredBox: cardBox(featured),
    written: written.length,
    total: cards.length,
    og: null,
  };
  const sprites: Hubs["sprites"] = {};
  hubs.forEach(({ id }, i) => {
    if (tiles[i]) sprites[id] = spriteNames.get(tiles[i].slug);
  });
  return { hubs, home, sprites };
}

// By title, "The Mines" under M, as an index has it.
const sortKey = (title: string) => title.replace(/^The /, "");
const byTitle = (a: { title: string }, b: { title: string }) => sortKey(a.title).localeCompare(sortKey(b.title), "en");

