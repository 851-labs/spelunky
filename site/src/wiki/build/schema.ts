// The wiki page's frontmatter (docs/wiki-style.md, "The page model"), as a zod schema the build validates
// every page against: a page that doesn't fit fails the build.
import { z } from "zod";
import { CATEGORIES } from "./categories.ts";

export const PAGE_TYPES = [
  "monster",
  "item",
  "treasure",
  "trap",
  "place",
  "shop",
  "character",
  "mechanic",
  "guide",
  "quirk",
] as const;
export type PageType = (typeof PAGE_TYPES)[number];

// Slugs no page may take: the hubs' (`/wiki/monsters`, `/wiki/items`...), `media`, where the wiki's sprites
// and clips are served (`/wiki/media/`, site/wiki-media.sh), `og`, where its Open Graph images are, and
// `source`, the source viewer's (`/wiki/source`, source.ts).
export const RESERVED_SLUGS = new Set([
  ...CATEGORIES.map((c) => c.split("/")[0]),
  "media",
  "og",
  "source",
]);

// Lowercase kebab case: letters and digits, words joined by `-`.
const SLUG = /^[a-z0-9]+(-[a-z0-9]+)*$/;
export const slug = z.string().regex(SLUG, "a slug is lowercase kebab case (giant-spider)");
// A path in the gml/ tree, as pages write it (objects/oSnake/step.gml), and a citation of its lines.
export const GML_FILE = /^(objects|scripts)\/[A-Za-z0-9_/]+\.gml$/;
export const GML_CITE = /^(objects|scripts)\/[A-Za-z0-9_/]+\.gml:\d+(-\d+)?$/;
const gmlFile = z.string().regex(GML_FILE, "a source is a path in the gml/ tree (objects/oSnake/step.gml)");
const gmlCite = z.string().regex(GML_CITE, "a citation is a gml/ path and lines (objects/oSnake/step.gml:39-72)");
// A clip's id is its spec's path under wiki/clips/, without .toml.
const clipId = z.string().regex(/^[a-z0-9-]+(\/[a-z0-9-]+)+$/, "a clip id is a wiki/clips/ path (snake/bite)");
// Objects and sprites are GM names (oSnake, sSnakeWalkL); rooms too (rOlmec).
export const gmName = (prefix: string) => z.string().regex(new RegExp(`^${prefix}[A-Za-z0-9_]+$`), `a ${prefix}... name`);
// Inline MDX (docs/wiki-style.md: lede, hatnote, infobox text, FAQ answers): compiled like the body.
const inline = z.string().trim().min(1);

// An infobox value: a stat from stats.json, an object's create-event value, hand-written text with its
// citation, or (the image only) a sprite.
const statValue = z.strictObject({ stat: z.string() });
const initValue = z.strictObject({ init: z.string() });
const textValue = z.strictObject({ text: inline, cite: gmlCite.optional() });
// What a hub's card shows instead of the image's first frame, where that reads badly at card size: another
// sprite, another frame, or both (docs/wiki-style.md, "Infobox values").
export const cardSprite = z
  .strictObject({ sprite: gmName("s").optional(), frame: z.number().int().nonnegative().optional() })
  .refine((card) => card.sprite !== undefined || card.frame !== undefined, "a card names a sprite, a frame or both");
export type CardSprite = z.infer<typeof cardSprite>;
const spriteValue = z.strictObject({ sprite: gmName("s"), anim: z.boolean().optional(), alt: z.string(), card: cardSprite.optional() });
const dataOrText = z.union([statValue, initValue, textValue]);
export type InfoboxValue = z.infer<typeof dataOrText> | z.infer<typeof spriteValue> | string;

// Each type's infobox fields (docs/wiki-style.md, "Infobox fields"). Relations shown in it come from
// `relations`, so they aren't here. Every field is optional but the image.
const infobox = <T extends z.ZodRawShape>(fields: T) => z.strictObject({ image: spriteValue, ...fields });
export const INFOBOXES = {
  monster: infobox({
    health: dataOrText.optional(),
    damage: dataOrText.optional(),
    speed: dataOrText.optional(),
    behaviour: textValue.optional(),
  }),
  item: infobox({
    kind: z.enum(["equipment", "weapon", "carryable"]),
    effect: textValue.optional(),
    damage: dataOrText.optional(),
    price: dataOrText.optional(),
  }),
  treasure: infobox({ value: dataOrText.optional() }),
  trap: infobox({ damage: dataOrText.optional(), trigger: textValue.optional() }),
  place: infobox({ levels: textValue.optional(), chance: textValue.optional() }),
  shop: infobox({ chance: textValue.optional(), shopkeeper: textValue.optional() }),
  character: infobox({ role: textValue.optional(), health: dataOrText.optional() }),
} as const;

// A relation's entry: a slug the data backs, or one with its citation (and a note) that it doesn't.
const relationEntry = z.union([
  slug,
  z.strictObject({ slug, cite: gmlCite, note: z.string().optional() }),
]);
export type RelationEntry = z.infer<typeof relationEntry>;
// Which page types each relation goes on (docs/wiki-style.md, "Relations"): written on one side only.
export const RELATIONS: Record<string, readonly PageType[] | "any"> = {
  found_in: ["monster", "item", "treasure", "trap", "character", "shop"],
  part_of: ["place"],
  sold_in: ["item"],
  // A place's are its objects' (Kali's altar's gifts and punishments).
  drops: ["monster", "item", "trap", "place"],
  damaged_by: ["monster", "character"],
  related: "any",
};

// The relations each type's infobox shows, in order (docs/wiki-style.md, "Infobox fields"), and what it
// calls them. The rest are the body's and the generated lists'.
export const INFOBOX_RELATIONS: Partial<Record<PageType, readonly string[]>> = {
  monster: ["found_in", "drops"],
  item: ["sold_in", "found_in"],
  treasure: ["found_in"],
  trap: ["found_in", "drops"],
  place: ["part_of", "drops"],
  shop: ["found_in"],
  character: ["found_in"],
};
export const RELATION_LABELS: Record<string, string> = {
  found_in: "Found in",
  part_of: "Part of",
  sold_in: "Sold in",
  drops: "Drops",
  damaged_by: "Damaged by",
  related: "Related",
};

const faqEntry = z.strictObject({ q: z.string().trim().min(1), a: inline });

export const frontmatter = z
  .strictObject({
    slug,
    title: z.string().trim().min(1),
    type: z.enum(PAGE_TYPES),
    categories: z.array(z.enum(CATEGORIES)).min(1),
    objects: z.array(gmName("o")).optional(),
    rooms: z.array(gmName("r")).optional(),
    lede: inline,
    aliases: z.array(slug).optional(),
    hatnote: inline.optional(),
    infobox: z.record(z.string(), z.unknown()).optional(),
    // A page without an infobox (a mechanic, a guide, a quirk) has no image for its hub card: this names one.
    card: cardSprite.optional(),
    quirk: z
      .strictObject({
        // As the row has it, with what each status covers where it differs: "GML + Oracle (the Jar); GML (the Skull)".
        status: z.string().regex(/^(GML|OpenGMK|Oracle|GM8|DLL)( \+ (GML|OpenGMK|Oracle|GM8|DLL))*( \([^()]+\))?(; (GML|OpenGMK|Oracle|GM8|DLL)( \+ (GML|OpenGMK|Oracle|GM8|DLL))*( \([^()]+\))?)*$/),
        where: z.string().regex(/^[A-Za-z0-9_/.]+(:\d+(-\d+)?)?$/),
        pinned_by: z.string().min(1),
        entry: z.string().min(1),
        // The docs/quirks.md rows the page is about, each by its opening, a sentence or more of it as the row
        // has it, without its bold and code marks (quirks.ts checks them).
        rows: z
          .array(z.string().trim().regex(/\.$/, "a row's opening: a sentence or more, ending with its full stop"))
          .min(1)
          .refine((rows) => new Set(rows).size === rows.length, "a row is listed twice"),
      })
      .optional(),
    relations: z.record(z.string(), z.array(relationEntry)).default({}),
    clips: z.array(clipId).default([]),
    faq: z.array(faqEntry).min(3).max(5).optional(),
    sources: z.array(gmlFile).min(1),
    spoiler: z.boolean().optional(),
    verified: z.strictObject({
      port: z.string().regex(/^v\d+\.\d+\.\d+$/, "a port release (v0.1.7)"),
      date: z
        .string()
        .regex(/^\d{4}-\d{2}-\d{2}$/, "a date (2026-10-27)")
        // A real one: the layout writes its month out, and the sitemap and JSON-LD carry it.
        .refine((date) => !Number.isNaN(Date.parse(date)) && new Date(date).toISOString().startsWith(date), "a date that exists"),
    }),
  })
  .superRefine((page, ctx) => {
    const issue = (path: (string | number)[], message: string) => ctx.addIssue({ code: "custom", path, message });
    const t = page.type;
    const hasInfobox = t in INFOBOXES;
    // The page model's per-type rules (docs/wiki-style.md: Frontmatter, Page types).
    if (hasInfobox && page.infobox === undefined) issue(["infobox"], `a ${t} page has an infobox`);
    if (!hasInfobox && page.infobox !== undefined) issue(["infobox"], `a ${t} page has no infobox`);
    if (page.infobox !== undefined && hasInfobox) {
      const parsed = INFOBOXES[t as keyof typeof INFOBOXES].safeParse(page.infobox);
      if (!parsed.success) {
        for (const i of parsed.error.issues) issue(["infobox", ...(i.path as (string | number)[])], i.message);
      }
    }
    if (page.card !== undefined && hasInfobox) issue(["card"], "a page with an infobox gives its card as infobox.image.card");
    if (page.card !== undefined && !hasInfobox && page.card.sprite === undefined) {
      issue(["card"], "a page without an infobox names its card's sprite: it has no image to take a frame of");
    }
    if ((t === "quirk") !== (page.quirk !== undefined)) {
      issue(["quirk"], t === "quirk" ? "a quirk page has a quirk box" : "only a quirk page has a quirk box");
    }
    const needsObjects = ["monster", "item", "treasure", "trap", "character"].includes(t);
    if (needsObjects && !page.objects?.length) issue(["objects"], `a ${t} page names its objects`);
    // A place that's a thing in a level (Kali's altar) names the objects the data knows it by; an area or a
    // special level has none (relations.ts checks it, as it knows the relations' slugs).
    if (["shop", "guide"].includes(t) && page.objects !== undefined) {
      issue(["objects"], `a ${t} page has no objects`);
    }
    if (page.rooms !== undefined && t !== "place") issue(["rooms"], "only a place page lists rooms");
    if (t === "guide" && page.faq !== undefined) issue(["faq"], "a guide has no FAQ: it's the answer");
    if (t !== "guide" && page.faq === undefined) issue(["faq"], `a ${t} page has three to five FAQ entries`);
    for (const [relation, entries] of Object.entries(page.relations)) {
      const on = RELATIONS[relation];
      if (on === undefined) {
        issue(["relations", relation], `no relation ${relation} (${Object.keys(RELATIONS).join(", ")})`);
      } else if (on !== "any" && !on.includes(t)) {
        issue(["relations", relation], `${relation} goes on ${on.join(", ")} pages, not ${t}`);
      }
      const seen = new Set<string>();
      entries.forEach((e, i) => {
        const target = typeof e === "string" ? e : e.slug;
        if (target === page.slug) issue(["relations", relation, i], "a page doesn't relate to itself");
        if (seen.has(target)) issue(["relations", relation, i], `${relation} lists ${target} twice`);
        seen.add(target);
      });
    }
    if (new Set(page.clips).size !== page.clips.length) issue(["clips"], "a clip is listed twice");
  });

export type Frontmatter = z.infer<typeof frontmatter>;
