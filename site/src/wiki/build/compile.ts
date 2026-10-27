// Reads the wiki's pages (content/wiki/<slug>.mdx), validates them, and compiles each to a JavaScript
// module the routes render (docs/wiki-style.md is the page model, and its "The pipeline" this). Everything a
// page refers to is resolved here, at build time: its numbers (`<Stat>`, from the committed stats snapshot),
// its citations' numbers, its clips' titles (from wiki/clips/) and files and its sprites (from the pinned
// wiki media), its links' targets. Anything that doesn't resolve is an error, and any error fails the build.
import { readdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { compile } from "@mdx-js/mdx";
import remarkGfm from "remark-gfm";
import { parse as parseToml } from "smol-toml";
import { parse as parseYaml } from "yaml";
import type { Category } from "./categories.ts";
import {
  frontmatter as frontmatterSchema,
  GML_CITE,
  INFOBOX_RELATIONS,
  INFOBOXES,
  RELATION_LABELS,
  RESERVED_SLUGS,
} from "./schema.ts";
import type { Frontmatter, InfoboxValue, RelationEntry } from "./schema.ts";
import { readSnapshot, resolveStat } from "./stats.ts";
import type { Snapshot } from "./stats.ts";

export interface WikiOptions {
  // The pages: content/wiki/*.mdx.
  contentDir: string;
  // The stats snapshot: content/wiki-data/stats.json.
  statsFile: string;
  // The clip specs: the repo's wiki/clips/.
  clipsDir: string;
  // The relations' spec, the repo's wiki/data/relations.toml: the areas the navboxes group pages by, and
  // the names of the places and shops that haven't a page yet.
  relationsSpec: string;
  // The pin of the wiki media (wiki-media.txt), whose tag names the port release the clips were recorded
  // from, for their caption when the media themselves aren't here.
  mediaPin: string;
  // The unpacked media and where they're served (build.sh's WIKI_MEDIA_DIR and WIKI_MEDIA_URL), or null
  // (`npm run dev`, or no pin): then clips and sprites render as placeholders, with a warning.
  media: { dir: string; url: string } | null;
}

export interface Reference {
  n: number;
  kind: "gml" | "replay" | "clip" | "quirk" | "room";
  target: string;
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

// A link the layout draws: to a page, or (`href` absent) to one that doesn't exist yet, as its name.
export interface Link {
  slug: string;
  text: string;
  href?: string;
  // Its citations' notes, for a relation's entry that cites its line.
  notes: number[];
}

// A relation the infobox shows (schema.ts's INFOBOX_RELATIONS): "Found in" the Mines...
export interface InfoboxRelation {
  relation: string;
  label: string;
  links: Link[];
}

// A heading of the body's, for the table of contents: its anchor is the page's stable link to it.
export interface Section {
  depth: 2 | 3;
  id: string;
  title: string;
}

// A navbox at the foot of the page: every page in one of its categories, grouped by the area they're found
// in where any is (a group's `area` being null for the rest).
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
  relations: InfoboxRelation[];
  quirk: Frontmatter["quirk"] | null;
  clips: ClipInfo[];
  // The clips the body doesn't place, for the Gallery.
  gallery: string[];
  clipCaption: string;
  questions: string[];
  sections: Section[];
  navboxes: Navbox[];
  references: Reference[];
  sources: string[];
  verified: Frontmatter["verified"];
}

export interface CompiledPage {
  file: string;
  meta: PageMeta;
  // The module: `export default { meta, Body, Lede, Hatnote, Answers, Infobox }`, the components compiled
  // MDX functions taking `components` (src/wiki/components.tsx).
  code: string;
}

export interface Wiki {
  pages: CompiledPage[];
  errors: string[];
  // What the build says but doesn't fail on: media it hasn't got.
  warnings: string[];
}

// The components a page may use (docs/wiki-style.md, "Body components"); GmlExcerpt and WikiLink are what
// a ```gml fence and a [[link]] become.
const COMPONENTS = new Set(["Stat", "Cite", "Clip", "Sprite"]);
// HTML a page mustn't write: it's content, not code or another page.
const UNSAFE_ELEMENTS = new Set(["script", "style", "iframe", "object", "embed", "link", "meta", "base", "form"]);
// Sections the layout generates, which a page doesn't write.
const GENERATED_SECTIONS = new Set(["contents", "table of contents", "faq", "gallery", "see also", "references", "what links here"]);

// The clip caption the license requires (docs/wiki-license.md, "Clip caption"), naming the port release
// the pinned media were recorded from: a game release's tag. A wiki-media-* release (made by hand between
// releases) doesn't say which, so its caption leaves the version out.
export function clipCaption(tag: string | null): string {
  const port = tag && /^v\d+\.\d+\.\d+$/.test(tag) ? `the spelunky.ai port ${tag}` : "the spelunky.ai port";
  return (
    `Recorded from ${port}, an unofficial port (a mod) of Spelunky Classic 1.1. ` +
    "Spelunky © 2008, 2009 Derek Yu and Mossmouth, LLC."
  );
}

function pinnedTag(path: string): string | null {
  const tag = /^tag\s+(\S+)/m.exec(readFileSync(path, "utf8"))?.[1];
  return tag && tag !== "none" ? tag : null;
}

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
  const media = options.media ? readMedia(options.media) : null;
  if (!media) warnings.push("no wiki media (WIKI_MEDIA_DIR): clips and sprites render as placeholders");
  // The media's own caption names the release that rendered them; the pin's tag is the next best thing.
  const caption = media?.caption ?? clipCaption(pinnedTag(options.mediaPin));

  // First every page's frontmatter, so that links and the checks across pages can see them all.
  const parsed: Parsed[] = [];
  for (const slug of pageSlugs(options.contentDir)) {
    const file = join(options.contentDir, `${slug}.mdx`);
    const page = parsePage(file, readFileSync(file, "utf8"), errors);
    if (page) parsed.push(page);
  }
  checkAcrossPages(parsed, errors);
  const titles = new Map<string, string>();
  for (const { page } of parsed) {
    titles.set(page.slug, page.title);
    for (const alias of page.aliases ?? []) titles.set(alias, page.title);
  }
  const hrefs = new Map<string, string>();
  for (const { page } of parsed) {
    hrefs.set(page.slug, page.slug);
    for (const alias of page.aliases ?? []) hrefs.set(alias, page.slug);
  }

  const spec = readRelationsSpec(options.relationsSpec);
  const navboxes = makeNavboxes(parsed, spec.areas);

  const pages: CompiledPage[] = [];
  for (const p of parsed) {
    const ctx = { data, titles, hrefs, names: spec.names, navboxes, caption, clipsDir: options.clipsDir, media, warnings };
    const page = await compilePage(p, ctx, errors);
    if (page) pages.push(page);
  }
  return { pages, errors, warnings };
}

// What the layout takes from the relations' spec (docs/wiki-data.md, "Relations"): its areas, in order, and
// the names of its areas, special levels and shops, which a link to one without a page yet shows.
function readRelationsSpec(path: string) {
  const spec = parseToml(readFileSync(path, "utf8")) as Record<string, { slug: string; name: string }[] | undefined>;
  const areas = (spec.area ?? []).map(({ slug, name }) => ({ slug, name }));
  const names = new Map<string, string>();
  for (const kind of ["area", "level", "shop"]) for (const { slug, name } of spec[kind] ?? []) names.set(slug, name);
  return { areas, names };
}

const entrySlug = (entry: RelationEntry) => (typeof entry === "string" ? entry : entry.slug);

// Each category's navbox: its pages by title, grouped by the areas they're `found_in` (a page in two is in
// both) when any of them is in one, the rest after them as a group of their own (docs/wiki-style.md,
// "Links and navboxes").
function makeNavboxes(parsed: Parsed[], areas: { slug: string; name: string }[]): Map<Category, Navbox> {
  const navboxes = new Map<Category, Navbox>();
  const byTitle = [...parsed].sort((a, b) => a.page.title.localeCompare(b.page.title, "en"));
  for (const { page } of byTitle) {
    for (const category of page.categories) if (!navboxes.has(category)) navboxes.set(category, { category, groups: [] });
  }
  for (const [category, navbox] of navboxes) {
    const members = byTitle.map((p) => p.page).filter((page) => page.categories.includes(category));
    const link = (page: Frontmatter) => ({ slug: page.slug, title: page.title });
    const foundIn = (page: Frontmatter) => new Set((page.relations.found_in ?? []).map(entrySlug));
    const grouped = areas
      .map((area) => ({ area: area.name, links: members.filter((page) => foundIn(page).has(area.slug)).map(link) }))
      .filter((group) => group.links.length > 0);
    const rest = members.filter((page) => !areas.some((area) => foundIn(page).has(area.slug))).map(link);
    navbox.groups = grouped.length === 0 ? [{ area: null, links: rest }] : [...grouped, ...(rest.length > 0 ? [{ area: null, links: rest }] : [])];
  }
  return navboxes;
}

// The parts of the media's indexes the pages use (docs/wiki-media.md).
interface Media {
  url: string;
  caption: string | null;
  clips: Map<string, { width: number; height: number; files: { mp4: string; webm: string; poster: string } }>;
  sprites: Map<string, { frames: number; width: number; height: number }>;
}

// The sprites' upscaled files, nearest neighbour (docs/wiki-style.md: integer scales only).
const SPRITE_SCALE = 4;

function readMedia({ dir, url }: { dir: string; url: string }): Media {
  const read = (path: string) => JSON.parse(readFileSync(join(dir, path), "utf8"));
  const clipIndex = read("clips/index.json");
  const spriteIndex = read("sprites/index.json");
  const media: Media = { url, caption: clipIndex.caption ?? null, clips: new Map(), sprites: new Map() };
  for (const c of clipIndex.clips ?? []) {
    // Its files' paths are relative to clips/.
    const href = (file: string) => mediaHref(url, `clips/${file}`);
    const files = { mp4: href(c.files.mp4), webm: href(c.files.webm), poster: href(c.files.poster) };
    media.clips.set(c.id, { width: c.width, height: c.height, files });
  }
  for (const s of spriteIndex.sprites ?? []) {
    media.sprites.set(s.name, { frames: s.frames, width: s.width, height: s.height });
  }
  return media;
}

// A file in the media, by its path in the package. The asset store redirects `@` to `%40` (docs/web-deploy.md,
// "Wiki media"), so each segment is escaped.
function mediaHref(url: string, path: string): string {
  return `${url}/${path.split("/").map(encodeURIComponent).join("/")}`;
}

// A sprite as a page shows it: its animation (if it has several frames and the page asks) or first frame.
function spriteImage(media: Media | null, sprite: string, anim: boolean, alt: string): SpriteImage | string {
  if (!media) return { sprite, anim, alt, src: null, still: null, width: 16 * SPRITE_SCALE, height: 16 * SPRITE_SCALE };
  const s = media.sprites.get(sprite);
  if (!s) return `no sprite ${sprite} in the wiki media`;
  const still = mediaHref(media.url, `sprites/${sprite}/frames/0@${SPRITE_SCALE}x.png`);
  const animated = anim && s.frames > 1;
  return {
    sprite,
    anim: animated,
    alt,
    src: animated ? mediaHref(media.url, `sprites/${sprite}/anim@${SPRITE_SCALE}x.png`) : still,
    still,
    width: s.width * SPRITE_SCALE,
    height: s.height * SPRITE_SCALE,
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
  if (RESERVED_SLUGS.has(page.slug)) errors.push(`${name}: slug ${page.slug} is reserved (a hub's, or /wiki/media/)`);
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
  titles: Map<string, string>;
  hrefs: Map<string, string>;
  // The relations' spec's names, for a place or shop without a page yet.
  names: Map<string, string>;
  navboxes: Map<Category, Navbox>;
  caption: string;
  clipsDir: string;
  media: Media | null;
  warnings: string[];
}

// One page's compile: its fragments (the inline MDX of the frontmatter, and the body) share its citation
// numbers and the clips it has placed.
interface PageState {
  name: string;
  page: Frontmatter;
  clips: Map<string, ClipInfo>;
  placed: Set<string>;
  references: Reference[];
  // Headings' anchors, which are unique on a page, starting with the layout's own.
  anchors: Set<string>;
  // The body's sections, for the table of contents.
  sections: Section[];
  errors: string[];
}

// The anchors of the sections the layout generates, and of its own parts: the table of contents and the
// page's main content (the skip link's target).
const LAYOUT_ANCHORS = ["faq", "gallery", "references", "toc", "content"];
const isLayoutAnchor = (id: string) => LAYOUT_ANCHORS.includes(id) || /^(cite|clip|navbox|toc|infobox)-/.test(id);

async function compilePage(p: Parsed, ctx: Context, errors: string[]): Promise<CompiledPage | null> {
  const { page } = p;
  const name = basename(p.file);
  const state: PageState = {
    name,
    page,
    clips: new Map(),
    placed: new Set(),
    references: [],
    anchors: new Set(LAYOUT_ANCHORS),
    sections: [],
    errors,
  };
  const before = errors.length;

  for (const id of page.clips) {
    const clip = readClip(ctx.clipsDir, id, ctx.media);
    if (typeof clip === "string") errors.push(`${name}: clips: ${clip}`);
    else {
      // A new spec's clip is rendered by the next release (docs/wiki-media.md, "Publishing"), so until then
      // the page shows its title and description where it goes.
      if (ctx.media && !clip.files) ctx.warnings.push(`${name}: clip ${id} isn't in the wiki media yet`);
      state.clips.set(id, clip);
    }
  }

  // In reading order, so that citations are numbered as they appear.
  const fragments: string[] = [];
  const fragment = async (field: string, source: string) => {
    const out = await compileFragment(source, field, ctx, state);
    fragments.push(out.code);
    return { ref: `f${fragments.length - 1}`, text: out.text };
  };
  const hatnote = page.hatnote === undefined ? null : await fragment("hatnote", page.hatnote);
  const lede = await fragment("lede", page.lede);
  if (lede.text.length > 160) {
    errors.push(`${name}: lede: ${lede.text.length} characters rendered, more than 160 (it's the meta description)`);
  }

  const infobox: InfoboxRow[] = [];
  const infoboxText: Record<string, string> = {};
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
          infoboxText[field] = (await fragment(`infobox.${field}`, value.text)).ref;
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
  // After the fields, as the infobox shows them, so that their notes are numbered in that order.
  const relations: InfoboxRelation[] = [];
  for (const relation of INFOBOX_RELATIONS[page.type] ?? []) {
    const entries = page.relations[relation] ?? [];
    if (entries.length === 0) continue;
    const links = entries.map((entry): Link => {
      const slug = entrySlug(entry);
      const target = ctx.hrefs.get(slug);
      const text = ctx.titles.get(slug) ?? ctx.names.get(slug) ?? slug[0].toUpperCase() + slug.slice(1).replaceAll("-", " ");
      const notes = typeof entry === "string" ? [] : [reference(state, "gml", entry.cite).n];
      return { slug, text, ...(target === undefined ? {} : { href: `/wiki/${target}` }), notes };
    });
    relations.push({ relation, label: RELATION_LABELS[relation] ?? relation, links });
  }

  const body = await fragment("body", p.body);
  const answers: string[] = [];
  for (const [i, { a }] of (page.faq ?? []).entries()) answers.push((await fragment(`faq[${i}].a`, a)).ref);
  if (body.text === "" && errors.length === before) errors.push(`${name}: the page has no body`);
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
    quirk: page.quirk ?? null,
    clips: [...state.clips.values()],
    gallery: page.clips.filter((id) => !state.placed.has(id)),
    clipCaption: ctx.caption,
    questions: (page.faq ?? []).map((f) => f.q),
    sections: state.sections,
    navboxes: page.categories.map((category) => ctx.navboxes.get(category)!),
    references: state.references,
    sources: page.sources,
    verified: page.verified,
  };
  const code = [
    'import * as runtime from "react/jsx-runtime";',
    // Each fragment is MDX's function-body output, which takes the JSX runtime as its argument: run here,
    // once, as the module loads, rather than evaluated from a string at render (no eval in a Worker).
    ...fragments.map((f, i) => `const f${i} = (function () {\n${f}\n}).call(undefined, runtime).default;`),
    `export default {`,
    `  meta: ${JSON.stringify(meta)},`,
    `  Body: ${body.ref},`,
    `  Lede: ${lede.ref},`,
    `  Hatnote: ${hatnote ? hatnote.ref : "null"},`,
    `  Answers: [${answers.join(", ")}],`,
    `  Infobox: {${Object.entries(infoboxText).map(([k, v]) => `${JSON.stringify(k)}: ${v}`).join(", ")}},`,
    `};`,
  ].join("\n");
  return { file: p.file, meta, code };
}

const pick = ({ text, value, cite }: { text: string; value: string; cite: string[] }) => ({ text, value, cite });

// A citation's note: the same source cited twice is the same note.
function reference(state: PageState, kind: Reference["kind"], target: string): Reference {
  let ref = state.references.find((r) => r.kind === kind && r.target === target);
  if (!ref) {
    ref = { n: state.references.length + 1, kind, target };
    state.references.push(ref);
  }
  return ref;
}

function readClip(clipsDir: string, id: string, media: Media | null): ClipInfo | string {
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
  const rendered = media?.clips.get(id);
  return {
    id,
    title,
    description,
    loop: spec.loop !== false,
    width: rendered?.width ?? 320 * scale,
    height: rendered?.height ?? 240 * scale,
    files: rendered?.files ?? null,
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
  const transform = () => (tree: Node) => {
    const fail = (node: Node, message: string) => state.errors.push(`${where(node.position?.start.line)}: ${message}`);
    rewrite(tree, (node) => transformNode(node, fail, ctx, state));
    if (field === "lede") {
      const blocks = tree.children ?? [];
      if (blocks.length !== 1 || blocks[0].type !== "paragraph") fail(tree, "the lede is one sentence: a single paragraph");
    }
    text = plainText(tree);
    if (field === "body") tableOfContents(tree, state);
  };
  try {
    const out = await compile(
      { value: source, path: state.name },
      { outputFormat: "function-body", remarkPlugins: [remarkGfm, transform], development: false },
    );
    return { code: String(out), text };
  } catch (e) {
    const err = e as { line?: number; reason?: string; message: string };
    state.errors.push(`${where(err.line)}: ${err.reason ?? err.message}`);
    return { code: "return { default: () => null };", text: "" };
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
    case "code":
      if (node.lang !== "gml") return undefined;
      // A GML excerpt: its info string is `gml <file>:<lines>` (docs/wiki-style.md, "Body components").
      if (!GML_CITE.test(node.meta ?? "")) fail(node, "a ```gml excerpt cites its lines: ```gml objects/oSnake/step.gml:73-76");
      return [element("mdxJsxFlowElement", "GmlExcerpt", [attribute("cite", node.meta ?? ""), attribute("code", node.value ?? "")])];
    case "text":
      return wikiLinks(node, fail, ctx);
    case "mdxJsxFlowElement":
    case "mdxJsxTextElement":
      return component(node, fail, ctx, state);
    default:
      return undefined;
  }
}

// [[slug]], [[slug|text]] and [[slug#anchor]] in a text node (a table cell's `\|` is a `|` by now).
// 851-3334 makes a link to a page that doesn't exist an error; until then it's text.
function wikiLinks(node: Node, fail: (node: Node, message: string) => void, ctx: Context): Node[] | undefined {
  const value = node.value ?? "";
  if (!value.includes("[[")) return undefined;
  const out: Node[] = [];
  let last = 0;
  for (const m of value.matchAll(WIKILINK)) {
    const [whole, slug, anchor, label] = m;
    if (m.index > last) out.push({ type: "text", value: value.slice(last, m.index) });
    last = m.index + whole.length;
    const target = ctx.hrefs.get(slug);
    const text = label ?? ctx.titles.get(slug) ?? slug.replaceAll("-", " ");
    const attributes = [attribute("slug", slug)];
    if (target !== undefined) attributes.push(attribute("href", `/wiki/${target}${anchor ? `#${anchor}` : ""}`));
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
      allow("id", "as", "bare");
      const id = attrs.get("id");
      if (!id) {
        fail(node, '<Stat> needs id="<slug>.<stat>"');
        return [];
      }
      const as = attrs.get("as");
      try {
        const stat = resolveStat(ctx.data, id, { as: (as ?? undefined) as "seconds" | undefined, bare: attrs.has("bare") });
        return [element("mdxJsxTextElement", "Stat", [attribute("text", stat.text), attribute("value", stat.value), attribute("cite", stat.cite.join(" "))])];
      } catch (e) {
        fail(node, (e as Error).message);
        return [];
      }
    }
    case "Cite": {
      const kinds = ["gml", "replay", "clip", "quirk", "room"] as const;
      allow(...kinds);
      const given = kinds.filter((k) => attrs.has(k));
      if (given.length !== 1) {
        fail(node, `<Cite> takes one of ${kinds.join("=, ")}=`);
        return [];
      }
      const kind = given[0];
      const target = attrs.get(kind) ?? "";
      if (kind === "gml" && !GML_CITE.test(target)) fail(node, `<Cite gml="${target}">: a gml/ path and lines (objects/oSnake/step.gml:41-54)`);
      if (kind === "clip" && !state.clips.has(target)) fail(node, `<Cite clip="${target}">: the page's clips don't list it`);
      const ref = reference(state, kind, target);
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
          attribute("caption", ctx.caption),
          attribute("width", String(clip.width)),
          attribute("height", String(clip.height)),
          ...(clip.loop ? [attribute("loop", null)] : []),
          ...(clip.files
            ? [attribute("mp4", clip.files.mp4), attribute("webm", clip.files.webm), attribute("poster", clip.files.poster)]
            : []),
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

// What a fragment says, as text: the lede's meta description, a heading's anchor.
function plainText(node: Node): string {
  const walk = (n: Node): string => {
    if (n.type === "text" || n.type === "inlineCode") return n.value ?? "";
    if (n.type === "break") return " ";
    if (n.type === "mdxFlowExpression" || n.type === "mdxTextExpression") return "";
    if (n.type === "mdxJsxTextElement" || n.type === "mdxJsxFlowElement") {
      if (n.name === "Cite") return "";
      if (n.name === "Stat") return String(n.attributes?.find((a) => a.name === "text")?.value ?? "");
    }
    const inner = (n.children ?? []).map(walk);
    return n.type === "paragraph" || n.type === "heading" ? `${inner.join("")} ` : inner.join("");
  };
  return walk(node)
    .replace(/\s+/g, " ")
    .replace(/ ([.,;:!?])/g, "$1")
    .trim();
}
