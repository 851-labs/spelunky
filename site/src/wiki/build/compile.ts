// Reads the wiki's pages (content/wiki/<slug>.mdx), validates them, and compiles each to a JavaScript
// module the routes render (docs/wiki-style.md is the page model, and its "The pipeline" this). Everything a
// page refers to is resolved here, at build time: its numbers (`<Stat>`, from the committed stats snapshot),
// its citations' numbers, its clips' titles (from wiki/clips/) and files and its sprites (from the
// wiki media), its links' targets and its relations (relations.ts). Anything that doesn't resolve is an
// error, and any error fails the build.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { compile } from "@mdx-js/mdx";
import remarkGfm from "remark-gfm";
import { parse as parseToml } from "smol-toml";
import { parse as parseYaml } from "yaml";
import { faqPage } from "../../seo/jsonld.ts";
import type { JsonLd } from "../../seo/jsonld.ts";
import type { SeoImage } from "../../seo/seo.ts";
import { clipVideo, mediaUrl, spriteFile, spriteImage as spriteImageLd } from "../../seo/media.ts";
import type { WikiMedia } from "../../seo/media.ts";
import { readMediaPackage, spriteWithPixels, wikiMedia } from "../../seo/media-package.ts";
import type { MediaPackage } from "../../seo/media-package.ts";
import { keyStats } from "./cards.ts";
import { CATEGORY_NAMES, hubById } from "./categories.ts";
import type { Category } from "./categories.ts";
import { placesFound, readPlanned, readRelations, relationRank, resolveRelations, targets as wikiTargets } from "./relations.ts";
import type { PageRelations, RelationList, Target } from "./relations.ts";
import { makeHubs } from "./hubs.ts";
import { checkQuirk, findRow, noSection, readQuirks } from "./quirks.ts";
import type { Hubs } from "./hubs.ts";
import type { QuirkRow, QuirkSection } from "./quirks.ts";
import { repoUrl } from "./repo.ts";
import { markdownTree, pageMarkdown } from "./markdown.ts";
import type { MdNode } from "./markdown.ts";
import { ogRenderer } from "./og.ts";
import type { OgCard } from "./og.ts";
import {
  frontmatter as frontmatterSchema,
  GML_CITE,
  GML_FILE,
  INFOBOX_RELATIONS,
  INFOBOXES,
  RESERVED_SLUGS,
} from "./schema.ts";
import type { Frontmatter, InfoboxValue, RelationEntry } from "./schema.ts";
import { roomNames, sourcePaths } from "./source.ts";
import { readSnapshot, resolveStat } from "./stats.ts";
import type { Snapshot } from "./stats.ts";
import { roomUrl, sourceUrl } from "../source-url.ts";
import { clipAnchor } from "../anchors.ts";

export interface WikiOptions {
  // The pages: content/wiki/*.mdx.
  contentDir: string;
  // The stats snapshot: content/wiki-data/stats.json.
  statsFile: string;
  // The relations snapshot: content/wiki-data/relations.json.
  relationsFile: string;
  // The pages planned but not written yet, which links may point at as red links:
  // content/wiki-planned.yaml.
  plannedFile: string;
  // The clip specs: the repo's wiki/clips/.
  clipsDir: string;
  // The relations' spec, the repo's wiki/data/relations.toml: the areas the navboxes group pages by.
  relationsSpec: string;
  // The repo's docs/quirks.md, whose rows the quirk pages and quirk citations name (quirks.ts).
  quirksFile: string;
  // The repo's checkout, which has the replays the pages cite, and the commit it's at (build.sh's WIKI_COMMIT),
  // which the pages' links to its files on GitHub are pinned to (repo.ts): "main" where there's none to pin to
  // (`npm run dev`).
  repoDir: string;
  commit: string;
  // The media, where they're served, when they last changed and the git log that dates each clip (build.sh's
  // WIKI_MEDIA_DIR, WIKI_MEDIA_URL, WIKI_MEDIA_DATE and WIKI_MEDIA_LOG), or null (`npm run dev`): then clips and
  // sprites render as placeholders, with a warning, and the pages have no media in their JSON-LD.
  media: { dir: string; url: string; date: string; log: string | null } | null;
  // The media's gml/ tree, the source viewer's (build/source.ts), which citations link to: null without the
  // media, and missing from stand-in media without one. Then citations are text, as the viewer has
  // no pages.
  sourceDir: string | null;
  // The media's rooms/ (build/source.ts), which room citations link to, and are checked against: a room it
  // hasn't got fails the build. Null without the media, and missing from stand-in media without one: then
  // room citations are text, unchecked.
  roomsDir: string | null;
  // Whether the web build at /play/ can be embedded (build.sh's WIKI_LIVE_CLIPS, from its page), whether it
  // seeks (WIKI_LIVE_SEEK), whether phones can play it (WIKI_LIVE_PHONES), whether it plays the clips cut from
  // committed replays (WIKI_LIVE_REPLAYS), whether it reports its start-up run (WIKI_LIVE_PROGRESS), whether it
  // reports its state at a clip's first frame (WIKI_LIVE_STATE) and whether it runs a clip through first, reporting
  // its states at the clip's frames (WIKI_LIVE_STATES), or null: then a clip whose replay the media have can play in
  // it (src/wiki/live-clip.tsx), a scene's or, with `replays`, any.
  liveClips: LiveClips | null;
}

// What the web build at /play/ can do, past being embedded (WikiOptions.liveClips).
export interface LiveClips {
  seek: boolean;
  phones: boolean;
  replays: boolean;
  progress: boolean;
  state: boolean;
  states: boolean;
}

export interface Reference {
  n: number;
  kind: "gml" | "replay" | "clip" | "quirk" | "room";
  target: string;
  // The docs/quirks.md row a quirk citation names by its opening (row=), if it names one.
  row?: string;
  // Where it links: a gml citation's lines in the source viewer, when the media's gml/ has them (sourceHref); a
  // replay on GitHub, at the commit the site is built from (repo.ts); a room's page in the source viewer, when
  // the media have rooms/.
  href?: string;
  // A replay's clips on the page, the ones cut from it, which play it.
  clips?: { id: string; title: string }[];
  // A quirk citation's rows of docs/quirks.md, and where the section's name links (quirkReference).
  quirk?: { href: string; rows: QuirkRowLink[] };
}

// A docs/quirks.md row a citation cites: its opening, as the page names it, the row on GitHub, and the quirk
// page that names it in its `rows`, if there is one and it isn't the citing page.
export interface QuirkRowLink {
  name: string;
  href: string;
  page?: { slug: string; title: string };
}

export interface ClipInfo {
  id: string;
  title: string;
  description: string;
  loop: boolean;
  width: number;
  height: number;
  // Its files' URLs, or null when the media don't have it (not rendered yet).
  files: { mp4: string; webm: string; poster: string } | null;
  // Whether its files have the game's sound (clip.json's has_audio).
  audio: boolean;
  // Its replay's URL and the replay's frame it starts on, when the media have the replay and the web build can
  // be embedded and play it (a scene's, or with LiveClips.replays one cut from a committed replay), to play it in
  // the game (src/wiki/live-clip.tsx), and whether the build seeks, plays on phones and reports its start-up run.
  // `state` is the game's state at the clip's first frame, which the player checks the build gets to, when it
  // reports it (LiveClips.state), and `states` its states at each of the clip's frames, when the build runs the clip
  // through and reports them (LiveClips.states).
  live:
    | ({ replay: string; from: number; state: string | null; states: string | null } & Pick<LiveClips, "seek" | "phones" | "progress">)
    | null;
}

export interface SpriteImage {
  sprite: string;
  anim: boolean;
  alt: string;
  // The @4x file's URL and its size, or null without the media. `still` is its first frame, for
  // `prefers-reduced-motion`; the same file when it's still.
  src: string | null;
  still: string | null;
  width: number;
  height: number;
}

export interface InfoboxRow {
  field: string;
  label: string;
  // Data (`stat`, `init`) and plain values come resolved; a `text` is compiled (the module's `infobox`).
  text?: string;
  // A data value's number and the lines it came from, as a <Stat>'s.
  value?: string;
  cite?: string[];
  // Its citations' notes in References: a data value's lines, or a `text`'s `cite`.
  notes: number[];
}

// A link in a relation's list: to a page, or (`href` absent) a red link to one that isn't written yet.
export interface RelationLink {
  slug: string;
  text: string;
  href?: string;
  // An entry's note, shown after it.
  note?: string;
  // Its citations' notes, for an entry that cites its line.
  notes: number[];
}

// A relation, or an inverse one, as a page shows it: "Found in" the Mines, "Dropped by" a Jar...
export interface Relation {
  // The relation (found_in) or its inverse (found_here).
  relation: string;
  label: string;
  links: RelationLink[];
}

// A heading of the body's, for the table of contents: its anchor is the page's stable link to it.
export interface Section {
  depth: 2 | 3;
  id: string;
  title: string;
}

// A navbox at the foot of the page: every page in one of its categories, grouped by the area they're found
// in (or a place's `part_of`) where any is, then the rest: the areas' own pages among them as "Areas", the
// others as a group whose `area` is null.
export interface Navbox {
  category: Category;
  groups: { area: string | null; links: { slug: string; title: string }[] }[];
}

// What a page's module exports besides its compiled parts (src/wiki/pages.ts): JSON.
export interface PageMeta {
  slug: string;
  title: string;
  type: Frontmatter["type"];
  categories: Frontmatter["categories"];
  aliases: string[];
  objects: string[];
  // The lede as plain text: the meta description and the answer engines' snippet.
  lede: string;
  spoiler: boolean;
  image: SpriteImage | null;
  infobox: InfoboxRow[];
  // The relations the infobox shows (relations.ts), and those See also does.
  relations: Relation[];
  seeAlso: Relation[];
  // A quirk page's quirk box, with its docs/quirks.md section's title and link, its rows' links, its `where`'s
  // lines in the source viewer (sourceHref) and its `pinned_by` on GitHub when it's a file (a replay).
  quirk:
    | (NonNullable<Frontmatter["quirk"]> & { section: string; href: string; rowHrefs: string[]; whereHref?: string; pinnedHref?: string })
    | null;
  clips: ClipInfo[];
  // The clips the body doesn't place, for the Gallery.
  gallery: string[];
  // Whether a clip it shows has sound: then its footer has the audio credit (docs/wiki-license.md).
  audio: boolean;
  questions: string[];
  sections: Section[];
  navboxes: Navbox[];
  references: Reference[];
  // Each with its page in the source viewer (sourceHref).
  sources: { path: string; href?: string }[];
  // The pages whose text or relations link to it, by title.
  linksHere: { slug: string; title: string }[];
  verified: Frontmatter["verified"];
  // Its JSON-LD besides the route's Article and BreadcrumbList, for its head: its FAQ's FAQPage and its
  // media's (mediaJsonLd).
  jsonLd: JsonLd[];
  // Its Open Graph image (og.ts), or null without the media: then seo()'s default.
  og: SeoImage | null;
}

export interface CompiledPage {
  file: string;
  meta: PageMeta;
  // The module: `export default { meta, Body, Lede, Hatnote, Answers, Infobox }`, the components compiled
  // MDX functions taking `components` (src/wiki/components.tsx).
  code: string;
  // What the page says, as plain text: its body, then its FAQ's questions and answers. The search index's
  // (src/wiki/search.ts), which the page's module doesn't carry.
  text: string;
  // The page as Markdown, its twin's content without the notice (markdown.ts). Nor does the module carry it.
  markdown: MdNode[];
}

export interface Wiki {
  pages: CompiledPage[];
  // The hubs' contents (hubs.ts).
  hubs: Hubs;
  // The pages' and hubs' Open Graph images (og.ts), by the path each is served at: none without the media.
  og: Map<string, Buffer>;
  errors: string[];
  // What the build says but doesn't fail on: media it hasn't got, relations a page leaves out.
  warnings: string[];
  // The red links: each page not written yet (planned, or wiki-data's) that pages link to, with the pages
  // that do, most linked first.
  wanted: { slug: string; title: string; from: string[] }[];
}

// The components a page may use (docs/wiki-style.md, "Body components"); GmlExcerpt and WikiLink are what
// a ```gml fence and a [[link]] become.
const COMPONENTS = new Set(["Stat", "Cite", "Clip", "Sprite"]);
// The ones that only make sense inside a sentence.
const INLINE_COMPONENTS = new Set(["Stat", "Cite"]);
// HTML a page mustn't write: it's content, not code or another page.
const UNSAFE_ELEMENTS = new Set(["script", "style", "iframe", "object", "embed", "link", "meta", "base", "form"]);
// Sections the layout generates, which a page doesn't write.
const GENERATED_SECTIONS = new Set(["contents", "table of contents", "faq", "gallery", "see also", "references", "what links here"]);

// The pages' files, by slug (their names): what vite.config.ts pre-renders.
export function pageSlugs(contentDir: string): string[] {
  return readdirSync(contentDir)
    .filter((f) => f.endsWith(".mdx"))
    .map((f) => basename(f, ".mdx"))
    .sort();
}

interface Parsed {
  file: string;
  page: Frontmatter;
  // The body, padded with the frontmatter's lines so that MDX's line numbers are the file's.
  body: string;
}

export async function loadWiki(options: WikiOptions): Promise<Wiki> {
  const errors: string[] = [];
  const warnings: string[] = [];
  const data = readSnapshot(options.statsFile);
  const relationsData = readRelations(options.relationsFile);
  const media = options.media ? readMediaPackage(options.media.dir, options.media.url, options.media.date, options.media.log) : null;
  // The media as the JSON-LD's builders see them (src/seo/media.ts), for each page's own.
  const seoMedia = media && wikiMedia(media);
  if (!media) warnings.push("no wiki media (WIKI_MEDIA_DIR): clips and sprites render as placeholders");
  const source = sourceLines(options.sourceDir);
  if (media && !source) warnings.push("the wiki media have no gml/: citations aren't linked");
  const rooms = roomNames(options.roomsDir ?? null);
  if (media && !rooms) warnings.push("the wiki media have no rooms/: room citations aren't linked or checked");

  // First every page's frontmatter, so that links and the checks across pages can see them all.
  const parsed: Parsed[] = [];
  for (const slug of pageSlugs(options.contentDir)) {
    const file = join(options.contentDir, `${slug}.mdx`);
    const page = parsePage(file, readFileSync(file, "utf8"), errors);
    if (page) parsed.push(page);
  }
  checkAcrossPages(parsed, errors);
  const quirks = readQuirks(options.quirksFile, options.commit);
  // The quirk page that names each row, by the row, which a citation of the row links to.
  const quirkPages = new Map<QuirkRow, { slug: string; title: string }>();
  for (const { page } of parsed) {
    if (!page.quirk) continue;
    const quirkErrors = checkQuirk(page.quirk, quirks);
    for (const error of quirkErrors) errors.push(`${page.slug}.mdx: ${error}`);
    if (quirkErrors.length > 0) continue;
    for (const name of page.quirk.rows) {
      const row = findRow(quirks.get(page.quirk.entry)!, page.quirk.entry, name) as QuirkRow;
      const other = quirkPages.get(row);
      if (other?.slug === page.slug) errors.push(`${page.slug}.mdx: quirk.rows: two of them name docs/quirks.md line ${row.line}`);
      else if (other) errors.push(`${page.slug}.mdx: quirk.rows: ${other.slug}.mdx names docs/quirks.md line ${row.line} too: one page a row`);
      else quirkPages.set(row, { slug: page.slug, title: page.title });
    }
  }
  const frontmatters = parsed.map((p) => p.page);
  const plan = readPlanned(options.plannedFile, errors);
  const targets = wikiTargets(frontmatters, plan, basename(options.plannedFile), data, relationsData, errors);
  const relations = resolveRelations(frontmatters, targets, relationsData, errors, warnings);

  const areas = readAreas(options.relationsSpec);
  const navboxes = makeNavboxes(parsed, areas);

  const compiled: Compiled[] = [];
  for (const p of parsed) {
    const ctx = {
      data,
      targets,
      relations,
      navboxes,
      clipsDir: options.clipsDir,
      liveClips: options.liveClips,
      quirks,
      quirkPages,
      repoDir: options.repoDir,
      commit: options.commit,
      media,
      seoMedia,
      source,
      rooms: rooms && new Set(rooms),
      warnings,
    };
    const page = await compilePage(p, ctx, errors);
    if (page) compiled.push(page);
  }

  // What links where, now that every page's links are known.
  const linksHere = new Map<string, Set<string>>();
  const wanted = new Map<string, Set<string>>();
  for (const { meta, links } of compiled) {
    for (const slug of new Set([...links, ...(relations.get(meta.slug)?.targets ?? [])])) {
      const target = targets.get(slug)!;
      const into = target.written ? linksHere : wanted;
      if (!into.has(slug)) into.set(slug, new Set());
      into.get(slug)!.add(meta.slug);
    }
  }
  const byTitle = (slugs: Iterable<string>) =>
    [...slugs].map((slug) => ({ slug, title: targets.get(slug)!.title })).sort((a, b) => a.title.localeCompare(b.title, "en"));
  const bySlug = new Map(compiled.map((c) => [c.meta.slug, c]));
  // The Open Graph images, which need the media's sprites and fonts.
  const renderOg = media ? ogRenderer(media) : null;
  const ogFiles = new Map<string, Buffer>();
  // A card's image, or null with an error: a character the game's font hasn't got, say.
  const og = (card: OgCard, id: string) => {
    try {
      const { image, png } = renderOg!(card, id);
      ogFiles.set(image.path, png);
      return image;
    } catch (e) {
      errors.push((e as Error).message);
      return null;
    }
  };
  const bySlugPage = new Map(parsed.map(({ page }) => [page.slug, page]));
  const pages: CompiledPage[] = [];
  for (const page of compiled) {
    const { meta } = page;
    for (const link of page.anchorLinks) {
      const target = bySlug.get(link.slug);
      if (target && !renderedAnchors(target).has(link.anchor)) {
        link.fail(`[[${link.slug}#${link.anchor}]]: ${link.slug} has no #${link.anchor}`);
      }
    }
    meta.linksHere = byTitle(linksHere.get(meta.slug) ?? []);
    if (renderOg) {
      // A page without an infobox image shows its own `card`, sprite and frame, as its hub card does.
      const own = meta.image ? undefined : bySlugPage.get(meta.slug)!.card;
      meta.og = og({
        kicker: meta.categories.map((category) => CATEGORY_NAMES[category]).join(" · "),
        title: meta.title,
        stats: keyStats(meta),
        sprite: meta.image?.sprite ?? own?.sprite ?? null,
        frame: own?.frame,
      }, meta.slug);
    }
    pages.push({ file: page.file, meta, code: page.code(), text: page.text, markdown: page.markdown });
  }
  // A planned page is one that's linked (by a page, or a planned quirk's `related`), or on a hub (by its type
  // or its category), the wiki's scope: one that's neither any more comes off the list.
  const plannedLinks = new Set([...targets.values()].flatMap((target) => target.related ?? []));
  for (const [slug, entry] of Object.entries(plan)) {
    if (targets.get(slug)?.written === false && !wanted.has(slug) && !plannedLinks.has(slug) && !entry.type && !entry.category) {
      errors.push(`${basename(options.plannedFile)}: no page links to ${slug}, and it has no type or category to be on a hub: remove it`);
    }
  }
  const hubs = makeHubs(
    pages.map(({ meta }) => {
      const page = bySlugPage.get(meta.slug)!;
      // Its infobox image's `card`, or a page without an infobox's own (schema.ts allows one or the other).
      const image = page.infobox?.image as InfoboxValue | undefined;
      const card = (typeof image === "object" && "sprite" in image && image.card) || page.card || null;
      return { meta, places: hubPlaces(page), card, related: (page.relations.related ?? []).map((e) => targets.get(entrySlug(e))?.slug ?? entrySlug(e)) };
    }),
    {
      unwritten: [...new Set(targets.values())].filter((target) => !target.written),
      kinds: Object.fromEntries(Object.entries(data.entities).map(([slug, entity]) => [slug, entity.kind])),
      areas,
      foundIn: placesFound(relationsData, targets),
    },
    (sprite, frame) => spriteFrame(media, sprite, frame),
    (object) => (media && Object.hasOwn(media.objects, object) ? media.objects[object] : null),
    errors,
    warnings,
  );
  if (renderOg) {
    // A hub's card has no stats: how many of its pages are written would read as a progress tracker.
    for (const data of hubs.hubs) {
      const hub = hubById(data.id)!;
      data.og = og({
        kicker: hub.parent ? hubById(hub.parent)!.name : "Category",
        title: hub.name,
        stats: [],
        sprite: hubs.sprites[data.id]?.sprite ?? null,
        frame: hubs.sprites[data.id]?.frame,
      }, data.id);
    }
    const { home } = hubs;
    home.og = og({
      kicker: "Spelunky Classic",
      title: "Wiki",
      stats: [],
      // The Spelunker.
      sprite: "sStandLeft",
    }, "wiki");
  }
  return {
    pages,
    hubs,
    og: ogFiles,
    errors,
    warnings,
    wanted: [...wanted]
      .map(([slug, from]) => ({ slug, title: targets.get(slug)!.title, from: [...from].sort() }))
      .sort((a, b) => b.from.length - a.from.length || a.slug.localeCompare(b.slug)),
  };
}

// The areas in the relations' spec (docs/wiki-data.md, "Relations"), in order, which the navboxes group
// pages by.
function readAreas(path: string) {
  const spec = parseToml(readFileSync(path, "utf8")) as { area?: { slug: string; name: string }[] };
  return (spec.area ?? []).map(({ slug, name }) => ({ slug, name }));
}

// A quirk page's box: its fields, its docs/quirks.md section's title and link and its rows' (loadWiki has
// checked they're there), and its `pinned_by` on GitHub when it's a file of the repo's (compilePage has checked
// it's there).
function quirkBox(quirk: Frontmatter["quirk"], ctx: Context, href: (cite: string) => string | undefined): PageMeta["quirk"] {
  const section = quirk && ctx.quirks.get(quirk.entry);
  if (!quirk || !section) return null;
  const whereHref = href(quirk.where);
  const rowHrefs = quirk.rows.map((name) => (findRow(section, quirk.entry, name) as QuirkRow).href);
  const pinned = quirk.pinned_by.includes("/");
  return {
    ...quirk,
    section: section.title,
    href: section.href,
    rowHrefs,
    ...(whereHref ? { whereHref } : {}),
    ...(pinned ? { pinnedHref: repoUrl(ctx.commit, quirk.pinned_by) } : {}),
  };
}

// A committed replay's path, as pages cite it (fidelity/replays/, or the settings program's fidelity/settings/).
const REPLAY = /^fidelity\/(replays|settings)\/(?!.*\.\.)[\w./-]+\.txt$/;
// A file of the repo's, as a quirk box's `pinned_by` names one (a replay, or a test's file).
const REPO_FILE = /^(?!.*\.\.)[\w-]+(\/[\w.-]+)+$/;

// What's wrong with a replay a page cites, if anything: not a committed replay's path, or not in the checkout
// (renamed or removed), which would link to nothing.
function replayError(path: string, repoDir: string): string | null {
  if (!REPLAY.test(path)) return "a committed replay's path (fidelity/replays/<name>.txt or fidelity/settings/<name>.txt)";
  if (!existsSync(join(repoDir, path))) return "the repo has no such replay";
  return null;
}

// A quirk citation's rows (Reference.quirk) and its section, linked: the row it names (row=); or, on a quirk page
// citing its own section, the page's rows, which its text is about. A row a quirk page names links to it, unless
// it's this page, and then the section's name links to the row on GitHub; otherwise the rows link there, and the
// name to the section's heading. compileFragment has checked the section and the row, and that a citation
// without one is its quirk page's own.
function quirkReference(ref: Reference, page: Frontmatter, ctx: Context): NonNullable<Reference["quirk"]> {
  const section = ctx.quirks.get(ref.target)!;
  const names = ref.row !== undefined ? [ref.row] : page.quirk!.rows;
  const rows = names.map((name): QuirkRowLink => {
    const row = findRow(section, ref.target, name) as QuirkRow;
    const named = ctx.quirkPages.get(row);
    return { name, href: row.href, ...(named && named.slug !== page.slug ? { page: named } : {}) };
  });
  const href = rows.length === 1 && rows[0].page ? rows[0].href : section.href;
  return { href, rows };
}

// The media's gml/ tree's files and how many lines each has, by path in it (objects/oSnake/step.gml), or
// null without one. Lines counted as the viewer numbers them (gml-highlight.ts) and wiki-cites checks them:
// split at each "\n", with no line after a final one.
function sourceLines(dir: string | null): Map<string, number> | null {
  const paths = sourcePaths(dir);
  if (paths.length === 0) return null;
  return new Map(
    paths.map((path) => {
      const lines = readFileSync(join(dir!, path), "utf8").split("\n");
      return [path, lines.at(-1) === "" ? lines.length - 1 : lines.length];
    }),
  );
}

// Where a citation (objects/oSnake/step.gml, or :47-54 of it) opens in the source viewer, or undefined
// where the media's gml/ hasn't got the file or its lines: the pages may cite another extraction than the
// media's (wiki-cites checks them against the extraction, and fails a line 0 or a reversed range,
// which this doesn't link either). Then it's text, and noted in MISSING with why, for the page's warning.
// Without any gml/, it's text with no warning (loadWiki says so once), as is a quirk's `where` that names
// no gml/ file.
function sourceHref(cite: string, source: Map<string, number> | null, missing: Set<string>): string | undefined {
  const [path, range] = cite.split(":");
  const span = range === undefined ? [] : /^(\d+)(?:-(\d+))?$/.exec(range);
  if (!source || !GML_FILE.test(path) || !span) return undefined;
  const lines = source.get(path);
  const [first, last] = span.slice(1).map((n) => (n === undefined ? undefined : Number(n)));
  if (lines === undefined) missing.add(`${cite} (no such file)`);
  else if (first !== undefined && (first < 1 || (last ?? first) < first || (last ?? first) > lines)) {
    missing.add(`${cite} (it has ${lines} lines)`);
  } else return sourceUrl(path, first, last);
  return undefined;
}

const entrySlug = (entry: RelationEntry) => (typeof entry === "string" ? entry : entry.slug);
// The areas and special levels a page is found in, by slug, what a hub filters it by and the navboxes group
// it by: its `found_in`, or a place's `part_of`, which says where it's found as `found_in` does a monster's
// (Kali's altar, in the Mines' side rooms; the Snake Pit, a Mines level).
const hubPlaces = (page: Frontmatter) => [...(page.relations.found_in ?? []), ...(page.relations.part_of ?? [])].map(entrySlug);

// Each category's navbox: its pages by title, grouped by the areas they're in (`hubPlaces`: a page in two is
// in both) when any of them is in one, the rest after them as a group of their own (docs/wiki-style.md,
// "Links and navboxes"). An area's own page (the Mines) is part of none, so it's in the rest: the areas' pages
// there are a group of their own, Areas, and the others after them, Other (the places': the Tutorial).
function makeNavboxes(parsed: Parsed[], areas: { slug: string; name: string }[]): Map<Category, Navbox> {
  const navboxes = new Map<Category, Navbox>();
  const byTitle = [...parsed].sort((a, b) => a.page.title.localeCompare(b.page.title, "en"));
  for (const { page } of byTitle) {
    for (const category of page.categories) if (!navboxes.has(category)) navboxes.set(category, { category, groups: [] });
  }
  const isArea = (page: { slug: string }) => areas.some((area) => area.slug === page.slug);
  for (const [category, navbox] of navboxes) {
    const members = byTitle.map((p) => p.page).filter((page) => page.categories.includes(category));
    const link = (page: Frontmatter) => ({ slug: page.slug, title: page.title });
    const grouped = areas
      .map((area) => ({ area: area.name, links: members.filter((page) => hubPlaces(page).includes(area.slug)).map(link) }))
      .filter((group) => group.links.length > 0);
    const rest = members.filter((page) => !areas.some((area) => hubPlaces(page).includes(area.slug))).map(link);
    const restGroups = [
      { area: "Areas", links: rest.filter(isArea) },
      { area: null, links: rest.filter((page) => !isArea(page)) },
    ].filter((group) => group.links.length > 0);
    navbox.groups = [...grouped, ...restGroups];
  }
  return navboxes;
}

// The placeholder's size without the media: a 16-pixel sprite upscaled as the media's are (docs/wiki-style.md:
// integer scales only).
const SPRITE_SCALE = 4;

// A sprite's frame's upscaled file, with its size in game pixels (a hub's card): null without the media,
// what's wrong without the sprite, its pixels or the frame.
function spriteFrame(media: MediaPackage | null, sprite: string, frame: number): { src: string; width: number; height: number } | string | null {
  if (!media) return null;
  const shown = spriteWithPixels(media, sprite);
  if (typeof shown === "string") return shown;
  const { width, height, frames } = media.sprites[sprite];
  // A sprite with pixels has a file for each frame.
  const file = frames[frame]?.scaled;
  if (!file) return `${sprite} has ${frames.length} frame${frames.length === 1 ? "" : "s"}, so no frame ${frame}`;
  return { src: mediaUrl(media, `sprites/${file}`), width, height };
}

// A sprite as a page shows it: its animation (if it has several frames and the page asks) or first frame, as
// its JSON-LD's ImageObject has it (mediaJsonLd).
function spriteImage(media: MediaPackage | null, sprite: string, anim: boolean, alt: string): SpriteImage | string {
  if (!media) return { sprite, anim, alt, src: null, still: null, width: 16 * SPRITE_SCALE, height: 16 * SPRITE_SCALE };
  const s = spriteWithPixels(media, sprite);
  if (typeof s === "string") return s;
  return {
    sprite,
    anim: anim && s.anim !== null,
    alt,
    src: mediaUrl(media, `sprites/${spriteFile(s, anim)}`),
    still: mediaUrl(media, `sprites/${s.image}`),
    width: s.width,
    height: s.height,
  };
}

// Splits off and validates the frontmatter.
export function parsePage(file: string, source: string, errors: string[]): Parsed | null {
  const name = basename(file);
  const match = /^---\r?\n([\s\S]*?)\r?\n---[ \t]*(?:\r?\n|$)/.exec(source);
  if (!match) {
    errors.push(`${name}: no frontmatter: a page starts with a --- line, its YAML, and another ---`);
    return null;
  }
  let yaml: unknown;
  try {
    yaml = parseYaml(match[1]);
  } catch (e) {
    errors.push(`${name}: frontmatter: ${(e as Error).message}`);
    return null;
  }
  const result = frontmatterSchema.safeParse(yaml);
  if (!result.success) {
    for (const issue of result.error.issues) {
      errors.push(`${name}: frontmatter${issue.path.map((p) => (typeof p === "number" ? `[${p}]` : `.${String(p)}`)).join("")}: ${issue.message}`);
    }
    return null;
  }
  const page = result.data;
  if (`${page.slug}.mdx` !== name) errors.push(`${name}: slug ${page.slug}: a page's file is named <slug>.mdx`);
  if (RESERVED_SLUGS.has(page.slug)) errors.push(`${name}: slug ${page.slug} is reserved (a hub's, /wiki/media/, /wiki/og/ or /wiki/source)`);
  const lines = match[0].split("\n").length - 1;
  return { file, page, body: "\n".repeat(lines) + source.slice(match[0].length) };
}

// Slugs and aliases are unique across the wiki, and an object belongs to one page at most.
function checkAcrossPages(parsed: Parsed[], errors: string[]) {
  const names = new Map<string, string>();
  const claim = (name: string, by: string) => {
    const other = names.get(name);
    if (other !== undefined) errors.push(`${by}: ${name} is already ${other}`);
    else names.set(name, by);
  };
  for (const { page } of parsed) claim(page.slug, `${page.slug}.mdx's slug`);
  for (const { page } of parsed) {
    for (const alias of page.aliases ?? []) {
      if (RESERVED_SLUGS.has(alias)) errors.push(`${page.slug}.mdx: alias ${alias} is reserved`);
      claim(alias, `${page.slug}.mdx's alias`);
    }
  }
  const objects = new Map<string, string>();
  for (const { page } of parsed) {
    for (const object of page.objects ?? []) {
      const other = objects.get(object);
      if (other !== undefined) errors.push(`${page.slug}.mdx: ${object} is already ${other}.mdx's: an object belongs to one page`);
      else objects.set(object, page.slug);
    }
  }
}

interface Context {
  data: Snapshot;
  // What a link's slug names (relations.ts).
  targets: Map<string, Target>;
  relations: Map<string, PageRelations>;
  navboxes: Map<Category, Navbox>;
  clipsDir: string;
  liveClips: LiveClips | null;
  quirks: Map<string, QuirkSection>;
  quirkPages: Map<QuirkRow, { slug: string; title: string }>;
  repoDir: string;
  commit: string;
  media: MediaPackage | null;
  // The JSON-LD's view of the media (mediaJsonLd).
  seoMedia: WikiMedia | null;
  // The media's gml/ files' lines (sourceLines).
  source: Map<string, number> | null;
  // The media's rooms (roomNames).
  rooms: Set<string> | null;
  warnings: string[];
}

// One page's compile: its fragments (the inline MDX of the frontmatter, and the body) share its citation
// numbers and the clips it has placed.
interface PageState {
  name: string;
  page: Frontmatter;
  clips: Map<string, ClipInfo>;
  // Its clips' replays (their specs' `replay`), by clip.
  clipReplays: Map<string, string>;
  placed: Set<string>;
  references: Reference[];
  // What tells its quirk citations apart (reference): the row each names, by its line.
  keys: Map<Reference, string | undefined>;
  // Headings' anchors, which are unique on a page, starting with the layout's own.
  anchors: Set<string>;
  // The body's sections, for the table of contents.
  sections: Section[];
  // The slugs its [[links]] point at, and the [[slug#anchor]]s, checked once every page is compiled.
  links: Set<string>;
  anchorLinks: AnchorLink[];
  // Its citations the media's gml/ hasn't got (sourceHref).
  missing: Set<string>;
  errors: string[];
}

interface AnchorLink {
  slug: string;
  anchor: string;
  // Reports an error where the link is.
  fail: (message: string) => void;
}

// A compiled page before What links here, which needs every page's links.
interface Compiled {
  file: string;
  meta: PageMeta;
  links: Set<string>;
  anchors: Set<string>;
  anchorLinks: AnchorLink[];
  // The module, once meta is complete.
  code: () => string;
  text: string;
  markdown: MdNode[];
}

// The anchors of the sections the layout generates, and of its own parts: the table of contents (and its
// rail, #toc-rail) and the page's main content (the skip link's target).
const LAYOUT_ANCHORS = ["faq", "gallery", "see-also", "references", "what-links-here", "toc", "content"];
const isLayoutAnchor = (id: string) => LAYOUT_ANCHORS.includes(id) || /^(cite|clip|navbox|toc|infobox)-/.test(id);

// The anchors a [[slug#anchor]] may name on a compiled page: its headings', and the generated sections it
// shows (src/wiki/layout.tsx). Not the layout's own parts: a link to another page's table of contents is
// one to its top.
function renderedAnchors({ meta, anchors }: Compiled): Set<string> {
  const shown = new Set(anchors);
  const hidden = [
    meta.questions.length === 0 && "faq",
    meta.gallery.length === 0 && "gallery",
    meta.seeAlso.length === 0 && "see-also",
    "toc",
    "content",
  ];
  for (const anchor of hidden) if (anchor) shown.delete(anchor);
  // Its clips (placed, or in the Gallery) and navboxes have anchors too.
  for (const clip of meta.clips) shown.add(clipAnchor(clip.id));
  for (const navbox of meta.navboxes) shown.add(`navbox-${navbox.category.replace("/", "-")}`);
  return shown;
}

async function compilePage(p: Parsed, ctx: Context, errors: string[]): Promise<Compiled | null> {
  const { page } = p;
  const name = basename(p.file);
  const state: PageState = {
    name,
    page,
    clips: new Map(),
    clipReplays: new Map(),
    placed: new Set(),
    references: [],
    keys: new Map(),
    anchors: new Set(LAYOUT_ANCHORS),
    sections: [],
    links: new Set(),
    anchorLinks: [],
    missing: new Set(),
    errors,
  };
  const before = errors.length;
  const href = (cite: string) => sourceHref(cite, ctx.source, state.missing);

  for (const id of page.clips) {
    const read = readClip(ctx.clipsDir, id, ctx.media, ctx.liveClips);
    if (typeof read === "string") errors.push(`${name}: clips: ${read}`);
    else {
      const { clip, replay } = read;
      // A spec whose clip isn't rendered yet (scripts/check fails a pull request for that: docs/wiki-media.md)
      // shows its title and description where it goes.
      if (ctx.media && !clip.files) ctx.warnings.push(`${name}: clip ${id} isn't in the wiki media yet`);
      state.clips.set(id, clip);
      if (replay) state.clipReplays.set(id, replay);
    }
  }

  // In reading order, so that citations are numbered as they appear.
  const fragments: string[] = [];
  const fragment = async (field: string, source: string) => {
    const out = await compileFragment(source, field, ctx, state);
    fragments.push(out.code);
    return { ref: `f${fragments.length - 1}`, text: out.text, markdown: out.markdown };
  };
  const hatnote = page.hatnote === undefined ? null : await fragment("hatnote", page.hatnote);
  const lede = await fragment("lede", page.lede);
  if (lede.text.length > 160) {
    errors.push(`${name}: lede: ${lede.text.length} characters rendered, more than 160 (it's the meta description)`);
  }

  const infobox: InfoboxRow[] = [];
  const infoboxText: Record<string, string> = {};
  const infoboxMarkdown: Record<string, MdNode> = {};
  let image: PageMeta["image"] = null;
  if (page.infobox && page.type in INFOBOXES) {
    for (const [field, value] of Object.entries(page.infobox) as [string, InfoboxValue][]) {
      if (typeof value === "object" && "sprite" in value) {
        const resolved = spriteImage(ctx.media, value.sprite, value.anim ?? false, value.alt);
        if (typeof resolved === "string") errors.push(`${name}: infobox.${field}: ${resolved}`);
        else image = resolved;
        continue;
      }
      const row: InfoboxRow = { field, label: field[0].toUpperCase() + field.slice(1).replaceAll("_", " "), notes: [] };
      try {
        if (typeof value === "string") row.text = value[0].toUpperCase() + value.slice(1);
        else if ("stat" in value) {
          if (/^o[A-Z]/.test(value.stat)) throw new Error(`stat "${value.stat}" is an object's: write { init: ${value.stat} }`);
          Object.assign(row, pick(resolveStat(ctx.data, value.stat)));
        } else if ("init" in value) {
          if (!/^o[A-Z]/.test(value.init)) throw new Error(`init "${value.init}" isn't <object>.<variable>`);
          Object.assign(row, pick(resolveStat(ctx.data, value.init)));
        } else {
          const text = await fragment(`infobox.${field}`, value.text);
          infoboxText[field] = text.ref;
          infoboxMarkdown[field] = text.markdown;
          if (value.cite) row.cite = [value.cite];
        }
        // A data value cites the lines it came from, as a <Stat> does, but visibly: an infobox's numbers
        // are what a reader checks first.
        row.notes = (row.cite ?? []).map((cite) => reference(state, "gml", cite).n);
      } catch (e) {
        errors.push(`${name}: infobox.${field}: ${(e as Error).message}`);
      }
      infobox.push(row);
    }
  }
  // The relations, the infobox's after its fields and See also's after the FAQ, so that their notes are
  // numbered in reading order.
  const lists = ctx.relations.get(page.slug)?.lists ?? [];
  const shown = (list: RelationList): Relation => ({
    relation: list.relation,
    label: list.label,
    links: list.links.map(({ slug, title, written, note, cite }) => ({
      slug,
      text: title,
      ...(written ? { href: `/wiki/${slug}` } : {}),
      ...(note === undefined ? {} : { note }),
      notes: cite === undefined ? [] : [reference(state, "gml", cite).n],
    })),
  });
  // The page's own in INFOBOX_RELATIONS' order, then the inverses.
  const order = INFOBOX_RELATIONS[page.type] ?? [];
  const rank = (list: RelationList) => (order.includes(list.relation) ? order.indexOf(list.relation) : order.length);
  const relations = lists
    .filter((list) => list.infobox)
    .sort((a, b) => rank(a) - rank(b))
    .map(shown);
  const body = await fragment("body", p.body);
  const answers: { ref: string; text: string; markdown: MdNode }[] = [];
  const text = [body.text];
  for (const [i, { q, a }] of (page.faq ?? []).entries()) {
    const answer = await fragment(`faq[${i}].a`, a);
    answers.push(answer);
    text.push(q, answer.text);
  }
  // See also's in RELATION_INFO's order, whatever the frontmatter's (the inverses come after, in it).
  const seeAlso = lists
    .filter((list) => !list.infobox)
    .sort((a, b) => Number(a.inverse) - Number(b.inverse) || relationRank(a.relation) - relationRank(b.relation))
    .map(shown);
  if (body.text === "" && errors.length === before) errors.push(`${name}: the page has no body`);
  // A quirk box's replay (or test's file) links to it, as a citation of a replay does; a test it names by its
  // name alone is text.
  const pinned = page.quirk?.pinned_by;
  if (pinned?.includes("/")) {
    const error = pinned.startsWith("fidelity/")
      ? replayError(pinned, ctx.repoDir)
      : !REPO_FILE.test(pinned) || !existsSync(join(ctx.repoDir, pinned))
        ? "the repo has no such file"
        : null;
    if (error) errors.push(`${name}: quirk.pinned_by: ${pinned}: ${error}`);
  }
  if (errors.length > before) return null;

  const meta: PageMeta = {
    slug: page.slug,
    title: page.title,
    type: page.type,
    categories: page.categories,
    aliases: page.aliases ?? [],
    objects: page.objects ?? [],
    lede: lede.text,
    spoiler: page.spoiler ?? false,
    image,
    infobox,
    relations,
    seeAlso,
    quirk: quirkBox(page.quirk, ctx, href),
    clips: [...state.clips.values()],
    gallery: page.clips.filter((id) => !state.placed.has(id)),
    audio: [...state.clips.values()].some((clip) => clip.audio),
    questions: (page.faq ?? []).map((f) => f.q),
    sections: state.sections,
    navboxes: page.categories.map((category) => ctx.navboxes.get(category)!),
    references: state.references.map((ref): Reference => {
      if (ref.kind === "replay") {
        const clips = [...state.clips.values()].filter((clip) => state.clipReplays.get(clip.id) === ref.target);
        return {
          ...ref,
          href: repoUrl(ctx.commit, ref.target),
          ...(clips.length > 0 ? { clips: clips.map(({ id, title }) => ({ id, title })) } : {}),
        };
      }
      if (ref.kind === "quirk") return { ...ref, quirk: quirkReference(ref, page, ctx) };
      // The Cite has failed the build if the media's rooms/ hasn't got it.
      if (ref.kind === "room") return ctx.rooms ? { ...ref, href: roomUrl(ref.target) } : ref;
      const to = ref.kind === "gml" ? href(ref.target) : undefined;
      return to ? { ...ref, href: to } : ref;
    }),
    sources: page.sources.map((path) => {
      const to = href(path);
      return to ? { path, href: to } : { path };
    }),
    linksHere: [],
    verified: page.verified,
    jsonLd: [
      // The FAQ the page shows, as plain text (docs/wiki-style.md, "FAQ blocks").
      ...(page.faq ? [faqPage(page.faq.map(({ q }, i) => ({ question: q, answer: answers[i].text })))] : []),
      ...mediaJsonLd(ctx.seoMedia, [...state.clips.values()], image),
    ],
    // Made once every page is compiled (loadWiki).
    og: null,
  };
  // The first few: media older than the pages miss the same files on many of them.
  if (state.missing.size > 0) {
    const missing = [...state.missing];
    const more = missing.length > 3 ? ` and ${missing.length - 3} more` : "";
    ctx.warnings.push(`${name}: the wiki media's gml/ hasn't got ${missing.slice(0, 3).join(", ")}${more}: not linked to the source viewer`);
  }
  const code = () => [
    'import * as runtime from "react/jsx-runtime";',
    // Each fragment is MDX's function-body output, which takes the JSX runtime as its argument: run here,
    // once, as the module loads, rather than evaluated from a string at render (no eval in a Worker).
    ...fragments.map((f, i) => `const f${i} = (function () {\n${f}\n}).call(undefined, runtime).default;`),
    `export default {`,
    `  meta: ${JSON.stringify(meta)},`,
    `  Body: ${body.ref},`,
    `  Lede: ${lede.ref},`,
    `  Hatnote: ${hatnote ? hatnote.ref : "null"},`,
    `  Answers: [${answers.map((answer) => answer.ref).join(", ")}],`,
    `  Infobox: {${Object.entries(infoboxText).map(([k, v]) => `${JSON.stringify(k)}: ${v}`).join(", ")}},`,
    `};`,
  ].join("\n");
  const markdown = pageMarkdown(meta, {
    hatnote: hatnote?.markdown ?? null,
    lede: lede.markdown,
    infobox: infoboxMarkdown,
    body: body.markdown,
    answers: answers.map((answer) => answer.markdown),
  });
  return { file: p.file, meta, links: state.links, anchors: state.anchors, anchorLinks: state.anchorLinks, code, text: text.join(" "), markdown };
}

// The page's media as search engines see them: a VideoObject per clip, in its `clips`' order (the main one
// first), and an ImageObject of the infobox's sprite as the page shows it, by the builders the media
// sitemaps use (src/seo/media.ts), so they say the same. Resolved here, at build time, so that the page's
// module carries only its own: the route's head would otherwise need the whole media index
// (virtual:wiki-media), and the route's code is the browser's too. A clip the media haven't got yet is left
// out, as the video sitemap leaves it out. The image's sprite is one with pixels (spriteImage), so the
// JSON-LD's view has it.
function mediaJsonLd(media: WikiMedia | null, clips: ClipInfo[], image: SpriteImage | null): JsonLd[] {
  if (!media) return [];
  const videos = clips.filter(({ id }) => Object.hasOwn(media.clips, id)).map(({ id }) => clipVideo(media, id));
  const images = image ? [spriteImageLd(media, image.sprite, { anim: image.anim, caption: image.alt || undefined })] : [];
  return [...videos, ...images];
}

const pick = ({ text, value, cite }: { text: string; value: string; cite: string[] }) => ({ text, value, cite });

// A citation's note: the same source cited twice is the same note, and so is a quirk row named by two openings
// (`line`, the row's in docs/quirks.md).
function reference(state: PageState, kind: Reference["kind"], target: string, row?: string, line?: number): Reference {
  const key = line === undefined ? row : `L${line}`;
  let ref = state.references.find((r) => r.kind === kind && r.target === target && state.keys.get(r) === key);
  if (!ref) {
    ref = { n: state.references.length + 1, kind, target, ...(row === undefined ? {} : { row }) };
    state.references.push(ref);
    state.keys.set(ref, key);
  }
  return ref;
}

// A clip's spec as the page shows it, and the committed replay it's cut from (its `replay`), if it says.
function readClip(
  clipsDir: string,
  id: string,
  media: MediaPackage | null,
  liveClips: LiveClips | null,
): { clip: ClipInfo; replay: string | null } | string {
  let spec: Record<string, unknown>;
  try {
    spec = parseToml(readFileSync(join(clipsDir, `${id}.toml`), "utf8"));
  } catch (e) {
    return (e as NodeJS.ErrnoException).code === "ENOENT"
      ? `no clip ${id}: wiki/clips/${id}.toml doesn't exist`
      : `wiki/clips/${id}.toml: ${(e as Error).message}`;
  }
  const { title, description } = spec;
  if (typeof title !== "string" || typeof description !== "string") return `wiki/clips/${id}.toml has no title or description`;
  // docs/wiki-media.md: 320x240 game pixels at `scale` (2 unless it says 4), looping unless it says not.
  const scale = typeof spec.scale === "number" ? spec.scale : 2;
  const rendered = media && Object.hasOwn(media.clips, id) ? media.clips[id] : undefined;
  // Its files' paths are relative to clips/.
  const files = media && rendered && {
    mp4: mediaUrl(media, `clips/${rendered.mp4}`),
    webm: mediaUrl(media, `clips/${rendered.webm}`),
    poster: mediaUrl(media, `clips/${rendered.poster}`),
  };
  const clip: ClipInfo = {
    id,
    title,
    description,
    loop: spec.loop !== false,
    width: rendered?.width ?? 320 * scale,
    height: rendered?.height ?? 240 * scale,
    files: files || null,
    audio: rendered?.audio ?? false,
    live: media && rendered ? clipLive(rendered.live, liveClips, (path) => mediaUrl(media, `clips/${path}`)) : null,
  };
  return { clip, replay: typeof spec.replay === "string" ? spec.replay : null };
}

// A rendered clip's live play (ClipInfo.live), its replay's path made a URL by `url`: when the media have its
// replay, and the web build at /play/ can be embedded and plays it. Any such build plays a scene's; only one
// that says so plays a clip cut from a committed replay (LiveClips.replays), as an older one could play the
// replay unlike its video. A build that reports its state at the clip's first frame (LiveClips.state) is held to
// the clip's, and shows the video when it plays the clip otherwise: a release from before a fix on the main branch,
// which the media are rendered from, can. One that runs the clip through first (LiveClips.states) is held to its
// states at every frame of it too.
export function clipLive(
  live: MediaPackage["clips"][string]["live"],
  liveClips: LiveClips | null,
  url: (path: string) => string,
): ClipInfo["live"] {
  if (!live || !liveClips || !(live.scene || liveClips.replays)) return null;
  return {
    replay: url(live.replay),
    from: live.from,
    state: liveClips.state ? live.state : null,
    states: liveClips.states ? live.states : null,
    seek: liveClips.seek,
    phones: liveClips.phones,
    progress: liveClips.progress,
  };
}

// ---------------------------------------------------------------------------------------------------------
// MDX

// The bits of mdast and its MDX extensions this touches.
interface Attribute {
  type: string;
  name?: string;
  value?: string | null | { type: string; value: string };
}
interface Node {
  type: string;
  children?: Node[];
  value?: string;
  name?: string | null;
  attributes?: Attribute[];
  lang?: string | null;
  meta?: string | null;
  depth?: number;
  data?: { hProperties?: Record<string, string> };
  position?: { start: { line: number; column: number } };
}

const attribute = (name: string, value: string | null): Attribute => ({ type: "mdxJsxAttribute", name, value });
const element = (type: "mdxJsxTextElement" | "mdxJsxFlowElement", name: string, attributes: Attribute[], children: Node[] = []): Node => ({
  type,
  name,
  attributes,
  children,
});

// Compiles the body, or a frontmatter field's inline MDX. Errors name the body's line (it's padded to the
// file's), or the field.
async function compileFragment(source: string, field: string, ctx: Context, state: PageState) {
  const where = (line?: number) =>
    field === "body" ? `${state.name}${line ? `:${line}` : ""}` : `${state.name}: ${field}`;
  let text = "";
  let markdown: MdNode = { type: "root", children: [] };
  const transform = () => (tree: Node) => {
    // Once per line: two of the same mistake on a line (two <Cite>s alone on it) are one to fix.
    const fail = (node: Node, message: string) => {
      const error = `${where(node.position?.start.line)}: ${message}`;
      if (!state.errors.includes(error)) state.errors.push(error);
    };
    rewrite(tree, (node) => transformNode(node, fail, ctx, state));
    if (field === "lede") {
      const blocks = tree.children ?? [];
      if (blocks.length !== 1 || blocks[0].type !== "paragraph") fail(tree, "the lede is one sentence: a single paragraph");
    }
    text = plainText(tree);
    // The Markdown twin's (markdown.ts), from the same tree as the HTML.
    markdown = markdownTree(tree as MdNode, `/wiki/${state.page.slug}`);
    if (field === "body") tableOfContents(tree, state);
  };
  try {
    const out = await compile(
      { value: source, path: state.name },
      { outputFormat: "function-body", remarkPlugins: [remarkGfm, transform], development: false },
    );
    return { code: String(out), text, markdown };
  } catch (e) {
    const err = e as { line?: number; reason?: string; message: string };
    state.errors.push(`${where(err.line)}: ${err.reason ?? err.message}`);
    return { code: "return { default: () => null };", text: "", markdown };
  }
}

// Lists the body's sections (its top-level headings: one in a list or a quote isn't a section) and puts the
// table of contents where Wikipedia's goes, after the summary, before the first of them (at the end if
// there's none). Its <Toc /> is the layout's, not a component a page can write.
function tableOfContents(tree: Node, state: PageState) {
  const blocks = tree.children ?? [];
  for (const node of blocks) {
    if (node.type !== "heading" || (node.depth !== 2 && node.depth !== 3)) continue;
    state.sections.push({ depth: node.depth, id: node.data?.hProperties?.id ?? "", title: plainText(node) });
  }
  const first = blocks.findIndex((node) => node.type === "heading");
  blocks.splice(first === -1 ? blocks.length : first, 0, element("mdxJsxFlowElement", "Toc", []));
  tree.children = blocks;
}

// Calls `f` on every node, depth first; when it returns nodes, they replace the node (and aren't visited).
function rewrite(node: Node, f: (node: Node) => Node[] | undefined) {
  if (!node.children) return;
  const out: Node[] = [];
  for (const child of node.children) {
    const replaced = f(child);
    if (replaced) out.push(...replaced);
    else {
      rewrite(child, f);
      out.push(child);
    }
  }
  node.children = out;
}

const WIKILINK = /\[\[([^\[\]|#]+?)(?:#([^\[\]|]+?))?(?:\|([^\[\]]+?))?\]\]/g;

function transformNode(node: Node, fail: (node: Node, message: string) => void, ctx: Context, state: PageState): Node[] | undefined {
  switch (node.type) {
    case "mdxjsEsm":
      fail(node, "a page can't import or export: its components are Stat, Cite, Clip and Sprite");
      return [];
    case "mdxFlowExpression":
    case "mdxTextExpression":
      // Comments only ({/* ... */}, as the templates have): a page is content, not code.
      if ((node.value ?? "").replace(/\/\*[\s\S]*?\*\//g, "").trim() !== "") {
        fail(node, "a page has no {expressions}, only {/* comments */}");
      }
      return undefined;
    case "heading": {
      // Its links and stats first, so that its anchor is made of the text it shows.
      rewrite(node, (child) => transformNode(child, fail, ctx, state));
      const title = plainText(node);
      // Stable anchors (/wiki/snake#behaviour): the heading in lowercase kebab case.
      const id = title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
      if (GENERATED_SECTIONS.has(title.toLowerCase())) fail(node, `the layout generates "${title}": don't write it`);
      else if (id === "") fail(node, `"${title}" has no letters or digits to make its anchor of`);
      else if (isLayoutAnchor(id)) {
        fail(node, `#${id} is one of the layout's anchors (a section's, a citation's, a clip's, a navbox's...)`);
      } else if (state.anchors.has(id)) fail(node, `two headings have the anchor #${id}`);
      state.anchors.add(id);
      node.data = { ...node.data, hProperties: { ...node.data?.hProperties, id } };
      return [node];
    }
    case "code": {
      if (node.lang !== "gml") return undefined;
      // A GML excerpt: its info string is `gml <file>:<lines>` (docs/wiki-style.md, "Body components"), and
      // its caption links them in the source viewer.
      if (!GML_CITE.test(node.meta ?? "")) fail(node, "a ```gml excerpt cites its lines: ```gml objects/oSnake/step.gml:73-76");
      const href = sourceHref(node.meta ?? "", ctx.source, state.missing);
      const attributes = [attribute("cite", node.meta ?? ""), attribute("code", node.value ?? "")];
      return [element("mdxJsxFlowElement", "GmlExcerpt", href ? [...attributes, attribute("href", href)] : attributes)];
    }
    case "text":
      return wikiLinks(node, fail, ctx, state);
    case "mdxJsxFlowElement":
    case "mdxJsxTextElement":
      return component(node, fail, ctx, state);
    default:
      return undefined;
  }
}

// [[slug]], [[slug|text]] and [[slug#anchor]] in a text node (a table cell's `\|` is a `|` by now). The
// slug (or an alias) is a page's, or a planned page's, which is a red link until it's written; any other
// fails the build, as a typo would otherwise be a red link for good.
function wikiLinks(node: Node, fail: (node: Node, message: string) => void, ctx: Context, state: PageState): Node[] | undefined {
  const value = node.value ?? "";
  if (!value.includes("[[")) return undefined;
  const out: Node[] = [];
  let last = 0;
  for (const m of value.matchAll(WIKILINK)) {
    const [whole, slug, anchor, label] = m;
    if (m.index > last) out.push({ type: "text", value: value.slice(last, m.index) });
    last = m.index + whole.length;
    const target = ctx.targets.get(slug);
    const text = label ?? target?.title ?? slug;
    if (!target) {
      fail(node, `[[${slug}]]: no page and no planned page of that slug (wiki-planned.yaml lists those not written yet)`);
    } else if (target.slug === state.page.slug) {
      fail(node, `[[${slug}]] is this page: link a section with [text](#anchor)`);
    } else {
      state.links.add(target.slug);
      if (anchor && target.written) {
        state.anchorLinks.push({ slug: target.slug, anchor, fail: (message) => fail(node, message) });
      }
    }
    const attributes = [attribute("slug", target?.slug ?? slug)];
    if (target?.written) attributes.push(attribute("href", `/wiki/${target.slug}${anchor ? `#${anchor}` : ""}`));
    out.push(element("mdxJsxTextElement", "WikiLink", attributes, [{ type: "text", value: text }]));
  }
  if (last < value.length) out.push({ type: "text", value: value.slice(last) });
  if (out.some((n) => n.type === "text" && n.value?.includes("[["))) fail(node, "a [[ that isn't a [[slug]] link");
  return out;
}

function component(node: Node, fail: (node: Node, message: string) => void, ctx: Context, state: PageState): Node[] | undefined {
  const name = node.name;
  // Fragments and HTML elements (<br />, <sub>) are fine, with string attributes and no scripts; a
  // capitalized name is one of the wiki's.
  if (!name || !/^[A-Z]/.test(name)) {
    if (name && UNSAFE_ELEMENTS.has(name.toLowerCase())) {
      fail(node, `a page has no <${name}>`);
      return [];
    }
    for (const a of node.attributes ?? []) {
      if (a.type !== "mdxJsxAttribute" || (a.value !== null && typeof a.value === "object")) {
        fail(node, `<${name ?? ""}>'s attributes are strings ("..."), not {expressions}`);
        return [];
      }
      if (/^on/i.test(a.name ?? "")) {
        fail(node, `<${name}> has an event handler, ${a.name}=: a page is content, not code`);
        return [];
      }
      if (a.name === "id" && isLayoutAnchor(String(a.value ?? ""))) {
        fail(node, `<${name} id="${a.value}">: #${a.value} is one of the layout's anchors`);
        return [];
      }
    }
    return undefined;
  }
  if (!COMPONENTS.has(name)) {
    fail(node, `no component <${name}> (a page has ${[...COMPONENTS].map((c) => `<${c}>`).join(", ")})`);
    return [];
  }
  // MDX makes a line holding nothing but components a block of its own, so a citation or a number that
  // starts its own source line ends the paragraph it was in: "[13] [14]" alone, the rest a new paragraph.
  if (node.type === "mdxJsxFlowElement" && INLINE_COMPONENTS.has(name)) {
    fail(node, `<${name}> starts a line with nothing else on it, which MDX makes a block that ends the paragraph: move it up to the end of the line before, or start its line with a word`);
  }
  const attrs = new Map<string, string | null>();
  for (const a of node.attributes ?? []) {
    if (a.type !== "mdxJsxAttribute" || a.name === undefined || (a.value !== null && typeof a.value === "object")) {
      fail(node, `<${name}>'s attributes are strings ("..."), not {expressions}`);
      return [];
    }
    attrs.set(a.name, (a.value as string | null | undefined) ?? null);
  }
  const allow = (...names: string[]) => {
    for (const a of attrs.keys()) if (!names.includes(a)) fail(node, `<${name}> has no ${a}= (it takes ${names.join(", ")})`);
  };
  switch (name) {
    case "Stat": {
      allow("id", "as", "bare", "at");
      const id = attrs.get("id");
      if (!id) {
        fail(node, '<Stat> needs id="<slug>.<stat>"');
        return [];
      }
      const as = attrs.get("as");
      // A table's row (a price on level 5), by the value of the variable the table runs over.
      const at = attrs.get("at");
      if (at !== undefined && !/^-?\d+$/.test(at ?? "")) {
        fail(node, `<Stat>'s at= is a whole number, not ${JSON.stringify(at)}`);
        return [];
      }
      try {
        const options = { as: (as ?? undefined) as "seconds" | undefined, bare: attrs.has("bare"), at: at === undefined ? undefined : Number(at) };
        const stat = resolveStat(ctx.data, id, options);
        return [element("mdxJsxTextElement", "Stat", [attribute("text", stat.text), attribute("value", stat.value), attribute("cite", stat.cite.join(" "))])];
      } catch (e) {
        fail(node, (e as Error).message);
        return [];
      }
    }
    case "Cite": {
      const kinds = ["gml", "replay", "clip", "quirk", "room"] as const;
      allow(...kinds, "row");
      const given = kinds.filter((k) => attrs.has(k));
      if (given.length !== 1) {
        fail(node, `<Cite> takes one of ${kinds.join("=, ")}=`);
        return [];
      }
      const kind = given[0];
      const target = attrs.get(kind) ?? "";
      const row = attrs.has("row") ? (attrs.get("row") ?? "") : undefined;
      if (kind === "gml" && !GML_CITE.test(target)) fail(node, `<Cite gml="${target}">: a gml/ path and lines (objects/oSnake/step.gml:41-54)`);
      if (kind === "clip" && !state.clips.has(target)) fail(node, `<Cite clip="${target}">: the page's clips don't list it`);
      if (kind === "replay") {
        const error = replayError(target, ctx.repoDir);
        if (error) fail(node, `<Cite replay="${target}">: ${error}`);
      }
      if (kind === "room" && !/^r[A-Za-z0-9_]+$/.test(target)) fail(node, `<Cite room="${target}">: a room's name (rOlmec)`);
      else if (kind === "room" && ctx.rooms && !ctx.rooms.has(target)) {
        fail(node, `<Cite room="${target}">: spelunky.gmk has no room ${JSON.stringify(target)} (the wiki media's rooms/ hasn't got it)`);
      }
      if (row !== undefined && kind !== "quirk") fail(node, `<Cite ${kind}>: only a quirk citation names a row=`);
      if (kind === "quirk") {
        // The section, and the row it names by its opening, as a quirk box's `rows` does (quirks.ts). Only a quirk
        // page's citation of its own section may leave the row out, as it cites the page's rows: a section as a
        // whole is a table of up to 150 rows, which tells a reader nothing.
        const section = ctx.quirks.get(target);
        if (!section) fail(node, `<Cite quirk="${target}">: ${noSection(target, ctx.quirks)}`);
        else if (row !== undefined) {
          const found = findRow(section, target, row);
          if (typeof found === "string") fail(node, `<Cite quirk="${target}" row=...>: ${found}`);
        } else if (state.page.quirk?.entry !== target) {
          fail(node, `<Cite quirk="${target}">: name the row it means, row="<the row's opening>" (only a quirk page citing its own section may leave it out)`);
        }
      }
      // Two openings of one row are one note.
      const found = kind === "quirk" && row !== undefined && ctx.quirks.has(target) ? findRow(ctx.quirks.get(target)!, target, row) : null;
      const ref = reference(state, kind, target, row, typeof found === "object" ? found?.line : undefined);
      return [element(node.type as "mdxJsxTextElement", "Cite", [attribute("n", String(ref.n))])];
    }
    case "Clip": {
      allow("id");
      const id = attrs.get("id") ?? "";
      const clip = state.clips.get(id);
      if (!clip) {
        fail(node, `<Clip id="${id}">: list it in the page's clips`);
        return [];
      }
      if (state.placed.has(id)) fail(node, `<Clip id="${id}"> is placed twice`);
      state.placed.add(id);
      return [
        element(node.type as "mdxJsxFlowElement", "Clip", [
          attribute("id", id),
          attribute("title", clip.title),
          attribute("description", clip.description),
          attribute("width", String(clip.width)),
          attribute("height", String(clip.height)),
          ...(clip.loop ? [attribute("loop", null)] : []),
          ...(clip.audio ? [attribute("audio", null)] : []),
          ...(clip.files
            ? [attribute("mp4", clip.files.mp4), attribute("webm", clip.files.webm), attribute("poster", clip.files.poster)]
            : []),
          ...(clip.live ? [attribute("replay", clip.live.replay), attribute("from", String(clip.live.from))] : []),
          ...(clip.live?.seek ? [attribute("seek", null)] : []),
          ...(clip.live?.phones ? [attribute("phones", null)] : []),
          ...(clip.live?.progress ? [attribute("progress", null)] : []),
          ...(clip.live?.state ? [attribute("state", clip.live.state)] : []),
          ...(clip.live?.states ? [attribute("states", clip.live.states)] : []),
        ]),
      ];
    }
    case "Sprite": {
      allow("name", "anim", "alt");
      const sprite = attrs.get("name") ?? "";
      if (!/^s[A-Za-z0-9_]+$/.test(sprite)) fail(node, '<Sprite> needs name="s..."');
      const alt = attrs.get("alt");
      if (alt === undefined || alt === null) {
        fail(node, '<Sprite> needs alt="..." (alt="" for an icon beside its name)');
        return [];
      }
      const image = spriteImage(ctx.media, sprite, attrs.has("anim"), alt);
      if (typeof image === "string") {
        fail(node, image);
        return [];
      }
      return [
        element(node.type as "mdxJsxTextElement", "Sprite", [
          attribute("name", sprite),
          attribute("alt", alt),
          attribute("width", String(image.width)),
          attribute("height", String(image.height)),
          ...(image.src ? [attribute("src", image.src)] : []),
          ...(image.still && image.still !== image.src ? [attribute("still", image.still)] : []),
        ]),
      ];
    }
  }
  return undefined;
}

// Where a fragment's node said nothing: a <Cite>, an expression, a component without text. (No text has
// it: Markdown's parser makes a NUL in the source U+FFFD.)
const DROPPED = "\u0000";
// The space before what said nothing, and before what follows it if that's more of the same, when
// punctuation comes next.
const SPACE_BEFORE_DROPPED = new RegExp(`(?: ?${DROPPED})+(?=[.,;:!?)])`, "g");

// What a fragment says, as text: the lede's meta description, a heading's anchor, the search index's text.
function plainText(node: Node): string {
  const walk = (n: Node): string => {
    if (n.type === "text" || n.type === "inlineCode") return n.value ?? "";
    if (n.type === "break") return " ";
    if (n.type === "mdxFlowExpression" || n.type === "mdxTextExpression") return DROPPED;
    if (n.type === "mdxJsxTextElement" || n.type === "mdxJsxFlowElement") {
      if (n.name === "Cite") return DROPPED;
      if (n.name === "Stat") return String(n.attributes?.find((a) => a.name === "text")?.value ?? "");
    }
    const inner = (n.children ?? []).map(walk).join("");
    if (inner === "" && (n.type === "mdxJsxTextElement" || n.type === "mdxJsxFlowElement")) return DROPPED;
    // A block's words end with it: a table's cells too, which the search index would otherwise run together.
    return n.type === "paragraph" || n.type === "heading" || n.type === "tableCell" ? `${inner} ` : inner;
  };
  // The space before what said nothing goes when punctuation follows it, as the page's superscript follows
  // its word: "ledge <Cite />." is "ledge.", not "ledge .". Only that space: one the text has stays, as in
  // "starting with `!`", and none is added after code or a link.
  return walk(node)
    .replace(/\s+/g, " ")
    .replace(SPACE_BEFORE_DROPPED, "")
    .replaceAll(DROPPED, "")
    .replace(/ {2,}/g, " ")
    .trim();
}
