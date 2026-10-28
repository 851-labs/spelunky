// What a link or a relation may point at, and the relations each page shows (docs/wiki-style.md, "Relations"
// and "Links and navboxes"): the pages, and the pages not written yet that the wiki plans, whose links are
// red links; each page's relations checked against the committed snapshot of wiki-data's relations.json
// (docs/wiki-data.md, "The site's snapshot"); and their inverses, on the pages they point at.
import { readFileSync } from "node:fs";
import { parse as parseYaml } from "yaml";
import { z } from "zod";
import { cardSprite, gmName, INFOBOX_RELATIONS, PAGE_TYPES, RELATION_LABELS, RELATIONS, RESERVED_SLUGS, slug } from "./schema.ts";
import type { CardSprite, Frontmatter, PageType } from "./schema.ts";
import type { Snapshot } from "./stats.ts";

// The snapshot's shape (crates/tools' wiki-data, `Relations::site_snapshot`).
export interface RelationsSnapshot {
  format: number;
  places: Record<string, { name: string; kind: "area" | "level" }>;
  shops: Record<string, string>;
  // Per object: where generation makes it (areas and special levels, by slug), and the rooms the room
  // editor placed it in.
  found_in: Record<string, { places: string[]; rooms: string[] }>;
  sold_in: Record<string, string[]>;
  // Per object: the objects its code makes during play, and those that hurt it. `own` is what the data
  // says of the object by name, `inherited` what it gets from a parent (oEnemy's code, which the Snake
  // runs; a collision with oEnemy).
  drops: Record<string, Split>;
  damaged_by: Record<string, Split>;
}

interface Split {
  own: string[];
  inherited: string[];
}

export function readRelations(path: string): RelationsSnapshot {
  const data = JSON.parse(readFileSync(path, "utf8")) as RelationsSnapshot;
  if (data.format !== 1) throw new Error(`${path}: format ${data.format}, not 1: update site/src/wiki/build/relations.ts`);
  return data;
}

// The pages the wiki plans but hasn't written (content/wiki-planned.yaml): the red links' allow-list. A
// planned page's `type` is what a relation to it is checked by, and its `objects` and `rooms` what a plain
// relation to it is checked by, as a page's are.
const planned = z.record(
  slug,
  z.strictObject({
    title: z.string().trim().min(1),
    type: z.enum(PAGE_TYPES).optional(),
    objects: z.array(gmName("o")).optional(),
    rooms: z.array(gmName("r")).optional(),
    // Its card's sprite on the hubs, where it has no object or its object's reads badly (hubs.ts).
    card: cardSprite.optional(),
  }),
);
export type Planned = z.infer<typeof planned>;

export function readPlanned(path: string, errors: string[]): Planned {
  const name = path.split("/").pop();
  let yaml: unknown;
  try {
    yaml = parseYaml(readFileSync(path, "utf8")) ?? {};
  } catch (e) {
    errors.push(`${name}: ${(e as Error).message}`);
    return {};
  }
  const result = planned.safeParse(yaml);
  if (!result.success) {
    for (const issue of result.error.issues) errors.push(`${name}: ${issue.path.join(".")}: ${issue.message}`);
    return {};
  }
  return result.data;
}

// A page a link can point at: one that's written, or one that isn't yet (a red link), which the wiki plans
// (wiki-planned.yaml) or wiki-data names (its entities', areas', special levels' and shops' slugs are their
// pages', docs/wiki-data.md).
export interface Target {
  slug: string;
  title: string;
  // Its page type, where it's known: a written page's, or what the data says it is.
  type: PageType | null;
  written: boolean;
  objects: string[];
  rooms: string[];
  // A planned page's card sprite (wiki-planned.yaml's `card`).
  card?: CardSprite;
}

// wiki-data's entity kinds, as page types.
const KIND_TYPES: Record<string, PageType> = {
  monster: "monster",
  character: "character",
  weapon: "item",
  item: "item",
  equipment: "item",
  treasure: "treasure",
  trap: "trap",
  mechanic: "mechanic",
};

// Every slug and alias a link may name, to its target.
export function targets(pages: Frontmatter[], plan: Planned, plannedName: string, stats: Snapshot, data: RelationsSnapshot, errors: string[]): Map<string, Target> {
  const map = new Map<string, Target>();
  for (const page of pages) {
    const target: Target = {
      slug: page.slug,
      title: page.title,
      type: page.type,
      written: true,
      objects: page.objects ?? [],
      rooms: page.rooms ?? [],
    };
    for (const name of [page.slug, ...(page.aliases ?? [])]) if (!map.has(name)) map.set(name, target);
  }
  const unwritten = (slug: string, title: string, type: PageType | null, objects: string[] = [], rooms: string[] = [], card?: CardSprite) => {
    const page = map.get(slug);
    // A page's alias can't take a slug wiki-data has for another page.
    if (page && page.slug !== slug) errors.push(`${page.slug}.mdx: alias ${slug} is wiki-data's slug for ${title}`);
    else if (!page) map.set(slug, { slug, title, type, written: false, objects, rooms, ...(card && { card }) });
  };
  for (const [slug, entity] of Object.entries(stats.entities)) {
    unwritten(slug, entity.name, KIND_TYPES[entity.kind] ?? null, entity.object ? [entity.object] : []);
  }
  for (const [slug, place] of Object.entries(data.places)) unwritten(slug, place.name, "place");
  for (const [slug, name] of Object.entries(data.shops)) unwritten(slug, name, "shop");
  for (const [slug, entry] of Object.entries(plan)) {
    const other = map.get(slug);
    if (other?.written && other.slug !== slug) errors.push(`${plannedName}: ${slug} is ${other.slug}.mdx's alias`);
    else if (other?.written) errors.push(`${plannedName}: ${slug} has a page now: remove it`);
    else if (other) errors.push(`${plannedName}: ${slug} is wiki-data's already (${other.title}): remove it`);
    else if (RESERVED_SLUGS.has(slug)) errors.push(`${plannedName}: slug ${slug} is reserved`);
    else unwritten(slug, entry.title, entry.type ?? null, entry.objects, entry.rooms, entry.card);
  }
  // An object (or a room) belongs to one page, written or not: that's how the data's objects map to pages.
  for (const kind of ["objects", "rooms"] as const) {
    const owners = new Map<string, string>();
    for (const target of new Set(map.values())) {
      for (const name of target[kind]) {
        const other = owners.get(name);
        if (other === undefined) owners.set(name, target.slug);
        // Two written pages with one object are compile.ts's error already.
        else if (kind === "rooms" || !(target.written && map.get(other)?.written)) {
          errors.push(`${name} is ${other}'s and ${target.slug}'s: an object or room belongs to one page`);
        }
      }
    }
  }
  return map;
}

// Which pages each relation points at, and its inverse: what it's called, and whether the infobox shows it
// for what the thing is and has, See also for the rest (851-3335 lays them out).
const RELATION_INFO: Record<string, { points: readonly PageType[] | null; inverse: string; inverseLabel: string; inverseInInfobox: boolean }> = {
  found_in: { points: ["place"], inverse: "found_here", inverseLabel: "Found here", inverseInInfobox: false },
  part_of: { points: ["place"], inverse: "contains", inverseLabel: "Contains", inverseInInfobox: true },
  sold_in: { points: ["shop"], inverse: "sells", inverseLabel: "Sells", inverseInInfobox: true },
  // A container's or a monster's drops can be monsters (a jar's Snake, a yellow ball's Bat).
  drops: { points: ["item", "treasure", "monster"], inverse: "dropped_by", inverseLabel: "Dropped by", inverseInInfobox: true },
  damaged_by: { points: null, inverse: "damages", inverseLabel: "Damages", inverseInInfobox: false },
  related: { points: null, inverse: "related", inverseLabel: "Related", inverseInInfobox: false },
};

// A relation's (or an inverse's) place in RELATION_INFO's order.
export function relationRank(relation: string): number {
  return Object.entries(RELATION_INFO).findIndex(([r, info]) => r === relation || info.inverse === relation);
}

export interface RelationLink {
  slug: string;
  title: string;
  // Whether its page exists: if not, it's a red link.
  written: boolean;
  note?: string;
  // A gml/ citation, which the page numbers among its references.
  cite?: string;
}

export interface RelationList {
  relation: string;
  label: string;
  // Whether it's an inverse (Dropped by), generated from other pages'.
  inverse: boolean;
  // Where the layout shows it: in the infobox (a page's own relations in schema.ts's INFOBOX_RELATIONS, and
  // the inverses of what it has: Dropped by, Sells, Contains), or under See also.
  infobox: boolean;
  links: RelationLink[];
}

export interface PageRelations {
  // Its own relations, in its frontmatter's order, then the inverses, in RELATION_INFO's.
  lists: RelationList[];
  // The slugs its own relations point at, for What links here and the red links' list.
  targets: Set<string>;
}

// Each page's relations: its own, checked, and the inverses of the others' that point at it.
export function resolveRelations(pages: Frontmatter[], map: Map<string, Target>, data: RelationsSnapshot, errors: string[], warnings: string[]): Map<string, PageRelations> {
  const out = new Map<string, PageRelations>(pages.map((p) => [p.slug, { lists: [], targets: new Set() }]));
  const inverses = new Map<string, Map<string, RelationLink[]>>();
  const backing = dataBacking(data, map);
  for (const page of pages) {
    const name = `${page.slug}.mdx`;
    const mine = out.get(page.slug)!;
    for (const [relation, entries] of Object.entries(page.relations)) {
      const info = RELATION_INFO[relation];
      // The schema has already failed the page on a relation that isn't one.
      if (!info) continue;
      const links: RelationLink[] = [];
      for (const entry of entries) {
        const { slug, cite, note } = typeof entry === "string" ? { slug: entry, cite: undefined, note: undefined } : entry;
        const where = `${name}: relations.${relation}: ${slug}`;
        const target = map.get(slug);
        if (!target) {
          errors.push(`${where}: no page and no planned page of that slug (wiki-planned.yaml lists those not written yet)`);
          continue;
        }
        if (target.slug === page.slug) {
          // The schema has already failed one to the page's own slug, but not to its alias.
          if (slug !== page.slug) errors.push(`${where}: a page doesn't relate to itself`);
          continue;
        }
        if (links.some((l) => l.slug === target.slug)) errors.push(`${where} is listed twice`);
        if (info.points && target.type && !info.points.includes(target.type)) {
          errors.push(`${where}: ${relation} points at ${info.points.join(" or ")} pages, and ${target.slug} is a ${target.type} page`);
        }
        // A plain slug is one the data backs; another needs its citation (docs/wiki-style.md, "Relations").
        if (cite === undefined && relation !== "related") {
          if (!page.objects?.length) errors.push(`${where}: a page without objects cites each relation ({ slug, cite })`);
          else if (!backing[relation]?.backs(page.objects, target)) {
            errors.push(`${where}: relations.json doesn't say so for ${page.objects.join(", ")}: cite the code ({ slug, cite })`);
          }
        }
        links.push({ slug: target.slug, title: target.title, written: target.written, note, cite });
        mine.targets.add(target.slug);
        if (target.written) {
          const theirs = inverses.get(target.slug) ?? new Map<string, RelationLink[]>();
          inverses.set(target.slug, theirs);
          const list = theirs.get(relation) ?? [];
          theirs.set(relation, list);
          list.push({ slug: page.slug, title: page.title, written: true, note, cite });
        }
      }
      const infobox = INFOBOX_RELATIONS[page.type]?.includes(relation) ?? false;
      if (links.length > 0) mine.lists.push({ relation, label: RELATION_LABELS[relation], inverse: false, infobox, links });
    }
    // What the data has that the page leaves out: a warning, as the page curates (but can't contradict). Not
    // a target the relation can't point at, which the page couldn't list (the Web Cannon's webs, a trap).
    if (page.objects?.length) {
      for (const [relation, check] of Object.entries(backing)) {
        const on = RELATIONS[relation];
        if (on !== "any" && !on.includes(page.type)) continue;
        const points = RELATION_INFO[relation]?.points;
        const listed = new Set((page.relations[relation] ?? []).map((e) => map.get(typeof e === "string" ? e : e.slug)?.slug));
        const missing = check.has(page.objects).filter((s) => {
          const type = map.get(s)?.type;
          return s !== page.slug && !listed.has(s) && (!points || !type || points.includes(type));
        });
        if (missing.length > 0) warnings.push(`${name}: relations.${relation} leaves out ${missing.join(", ")}, which relations.json has`);
      }
    }
  }
  for (const page of pages) {
    const mine = out.get(page.slug)!;
    const theirs = inverses.get(page.slug);
    if (!theirs) continue;
    for (const [relation, info] of Object.entries(RELATION_INFO)) {
      const links = (theirs.get(relation) ?? []).sort((a, b) => a.title.localeCompare(b.title, "en"));
      if (links.length === 0) continue;
      // Related is both ways: one list, the page's own first, each page once.
      const own = relation === "related" ? mine.lists.find((l) => l.relation === "related") : undefined;
      if (own) own.links.push(...links.filter((l) => !own.links.some((o) => o.slug === l.slug)));
      else mine.lists.push({ relation: info.inverse, label: info.inverseLabel, inverse: true, infobox: info.inverseInInfobox, links });
    }
  }
  return out;
}

// For each relation the data has: whether it backs a page's (by its objects) pointing at a target, and the
// targets it has for a page. Backing counts what the objects inherit, as the game does (the snapshot has
// followed it): the Snake is hurt by what hurts oEnemy, whose step it runs, and by any thrown item. What a
// page leaves out counts only what the data says of its objects by name, or every monster page would be
// told it leaves out every item and what oEnemy's code makes (Kali's gifts for a body on her altar).
interface Backing {
  backs(objects: string[], target: Target): boolean;
  has(objects: string[]): string[];
}

const unique = (slugs: (string | undefined)[]) => [...new Set(slugs.filter((s): s is string => s !== undefined))];

// Where the data says objects are found, by slug: the places generation makes them in, and the pages whose
// rooms the room editor placed them in. A plain found_in is checked against it, and the hubs filter a page
// not written yet by it.
export function placesFound(data: RelationsSnapshot, map: Map<string, Target>): (objects: string[]) => string[] {
  const roomOwners = new Map<string, string>();
  for (const target of new Set(map.values())) for (const r of target.rooms) roomOwners.set(r, target.slug);
  return (objects) =>
    unique(objects.flatMap((o) => [...(data.found_in[o]?.places ?? []), ...(data.found_in[o]?.rooms ?? []).map((r) => roomOwners.get(r))]));
}

function dataBacking(data: RelationsSnapshot, map: Map<string, Target>): Record<string, Backing> {
  const owners = new Map<string, string>();
  for (const target of new Set(map.values())) for (const o of target.objects) owners.set(o, target.slug);
  const foundIn = placesFound(data, map);
  const own = (split: Record<string, Split>, objects: string[]) => objects.flatMap((o) => split[o]?.own ?? []);
  const all = (split: Record<string, Split>, objects: string[]) => objects.flatMap((o) => [...(split[o]?.own ?? []), ...(split[o]?.inherited ?? [])]);
  const either = (split: Record<string, Split>): Backing => ({
    backs: (objects, target) => all(split, objects).some((o) => target.objects.includes(o)),
    has: (objects) => unique(own(split, objects).map((o) => owners.get(o))),
  });
  return {
    found_in: {
      backs: (objects, target) => foundIn(objects).includes(target.slug),
      has: foundIn,
    },
    sold_in: {
      backs: (objects, target) => objects.some((o) => data.sold_in[o]?.includes(target.slug)),
      has: (objects) => unique(objects.flatMap((o) => data.sold_in[o] ?? [])),
    },
    drops: either(data.drops),
    damaged_by: either(data.damaged_by),
  };
}
