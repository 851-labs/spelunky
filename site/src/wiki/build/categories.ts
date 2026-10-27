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

// A category's hub page (/wiki/monsters), or null while it hasn't got one. None has yet: 851-3336 builds
// them, and returning their paths here puts them in the pages' breadcrumbs and navbox titles.
export function hubPath(_category: Category): string | null {
  return null;
}
