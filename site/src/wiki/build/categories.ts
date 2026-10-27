// The wiki's categories: the hubs a page is listed in (docs/wiki-style.md, `categories`), their names, and
// where each hub is. No dependencies, since the pages' layout imports it as well as the build.

export const CATEGORIES = [
  "monsters",
  "characters",
  "items/equipment",
  "items/weapons",
  "items/carryables",
  "treasure",
  "traps",
  "places",
  "shops",
  "mechanics",
  "guides",
  "quirks",
  "the-game",
] as const;
export type Category = (typeof CATEGORIES)[number];

// What a navbox and a breadcrumb call each.
export const CATEGORY_NAMES: Record<Category, string> = {
  monsters: "Monsters",
  characters: "Characters",
  "items/equipment": "Equipment",
  "items/weapons": "Weapons",
  "items/carryables": "Carryables",
  treasure: "Treasure",
  traps: "Traps",
  places: "Places",
  shops: "Shops",
  mechanics: "Mechanics",
  guides: "Guides",
  quirks: "Quirks and glitches",
  "the-game": "The game",
};

// A category's hub page: /wiki/monsters, /wiki/items/weapons.
export function hubPath(category: Category): string {
  return `/wiki/${category}`;
}

// The hubs (src/routes/wiki/): one per category, and /wiki/items, which lists the three items' categories
// together and is the breadcrumbs' step above each.
export type HubId = Category | "items";

export interface Hub {
  id: HubId;
  name: string;
  path: string;
  // The categories whose pages it lists.
  categories: readonly Category[];
  // The hub above it, between it and Wiki in the breadcrumbs.
  parent: HubId | null;
  // Its meta description and the sentence under its title: what's in it, for a reader and a search engine.
  description: string;
}

const DESCRIPTIONS: Record<HubId, string> = {
  monsters:
    "Every monster in Spelunky Classic, from the Mines' snakes and bats to Olmec, with health, damage and " +
    "behaviour read from the game's code.",
  characters: "The characters of Spelunky Classic: the Spelunker, the Damsel, the Shopkeeper, the Tunnel Man and Kali.",
  items:
    "Every item in Spelunky Classic: the equipment the player wears, the weapons, and the things he can pick " +
    "up and throw.",
  "items/equipment":
    "The equipment of Spelunky Classic: shoes, gloves, the cape, the jetpack and the rest the player keeps once " +
    "picked up.",
  "items/weapons": "The weapons of Spelunky Classic, from the whip to the shotgun, with the damage each does.",
  "items/carryables":
    "The things the player can carry and throw in Spelunky Classic: rocks, pots, jars, crates, bombs and more.",
  treasure: "The treasure of Spelunky Classic: gold, gems and idols, and what each is worth.",
  traps: "The traps of Spelunky Classic: arrow traps, spikes, boulders and the rest, with what sets them off.",
  places: "The places of Spelunky Classic: its four areas, Olmec's Lair and the special levels between them.",
  shops: "The shops of Spelunky Classic, what each sells, and what happens to a thief.",
  mechanics: "How Spelunky Classic works underneath: health, stunning, throwing, money, Kali's favour and more.",
  guides: "Guides to Spelunky Classic's harder goals: the Black Market, the City of Gold, beating Olmec.",
  quirks:
    "The bugs and oddities of Spelunky Classic 1.1 that the spelunky.ai port keeps, each traced to the code " +
    "that causes it.",
  "the-game": "Spelunky Classic itself: its controls, settings, level editor, scoring, endings and versions.",
};

const ITEMS: readonly Category[] = ["items/equipment", "items/weapons", "items/carryables"];

// In the order /wiki lists them, /wiki/items before its own.
const ORDER: readonly HubId[] = [
  "monsters",
  "characters",
  "items",
  ...ITEMS,
  "treasure",
  "traps",
  "places",
  "shops",
  "mechanics",
  "guides",
  "quirks",
  "the-game",
];

export const HUBS: readonly Hub[] = ORDER.map((id) =>
  id === "items"
    ? { id, name: "Items", path: "/wiki/items", categories: ITEMS, parent: null, description: DESCRIPTIONS.items }
    : {
        id,
        name: CATEGORY_NAMES[id],
        path: hubPath(id),
        categories: [id],
        parent: ITEMS.includes(id) ? "items" : null,
        description: DESCRIPTIONS[id],
      },
);

export const hubById = (id: string): Hub | undefined => HUBS.find((hub) => hub.id === id);
// The hubs under one: /wiki/items's.
export const childrenOf = (id: HubId): Hub[] => HUBS.filter((hub) => hub.parent === id);
// Whether a hub lists a page of these categories.
export const inHub = (hub: Hub, categories: readonly Category[]) => categories.some((c) => hub.categories.includes(c));
