// The wiki as Markdown, for answer engines and LLMs (docs/wiki-style.md, "Markdown and llms.txt"): each
// written page's Markdown twin, /wiki/<slug>.md, and /llms.txt, an index of them (https://llmstxt.org), and
// /llms-full.txt, every page in one file. A twin is made of the same trees as the page's HTML (compile.ts),
// once every <Stat>, <Cite>, <Clip> and [[link]] is resolved, so it says what the page says. Read without the
// pages' header and footer, each file starts with the footer's notice, its link absolute, and the audio
// credit where a page's clips have sound (docs/wiki-license.md, "Per-page notice").
//
// Links go to the HTML pages, not to the twins: the page is the canonical URL, which an answer engine cites
// and a reader follows, and which keeps the twins from competing with it (the twins' Link header names it:
// src/server.ts). An agent after more Markdown finds every twin in /llms.txt, or adds .md to the URL.
import { gfmToMarkdown } from "mdast-util-gfm";
import { toMarkdown } from "mdast-util-to-markdown";
import { absoluteUrl, SITE_URL } from "../../seo/seo.ts";
import { AUDIO_CREDIT, NOTICE } from "../notices.ts";
import { CATEGORIES, CATEGORY_NAMES } from "./categories.ts";
import type { ClipInfo, PageMeta, Reference, RelationLink } from "./compile.ts";

// An mdast node: what compile.ts's trees are made of, and what mdast-util-to-markdown writes.
export interface MdNode {
  type: string;
  children?: MdNode[];
  value?: string;
  [key: string]: unknown;
}

// A page's fragments as Markdown trees (roots), as compile.ts compiled them.
export interface PageMarkdown {
  hatnote: MdNode | null;
  lede: MdNode;
  // A `text` field's, by field.
  infobox: Record<string, MdNode>;
  body: MdNode;
  answers: MdNode[];
}

const LICENSE_URL = `${SITE_URL}/LICENSE.txt`;
const SITE_TITLE = "spelunky.ai: the Spelunky Classic fan wiki";
const SITE_SUMMARY =
  "An unofficial fan wiki for Spelunky Classic, Derek Yu's 2008 roguelike platformer (the original Spelunky), " +
  "written from the game's source code: its numbers and behaviour are read from the GML, and cite the lines " +
  "they come from.";

const text = (value: string): MdNode => ({ type: "text", value });
const paragraph = (...children: MdNode[]): MdNode => ({ type: "paragraph", children });
const heading = (depth: number, value: string): MdNode => ({ type: "heading", depth, children: [text(value)] });
const link = (url: string, ...children: MdNode[]): MdNode => ({ type: "link", url, title: null, children });
const code = (value: string): MdNode => ({ type: "inlineCode", value });
const strong = (...children: MdNode[]): MdNode => ({ type: "strong", children });
const list = (items: MdNode[][]): MdNode => ({
  type: "list",
  ordered: false,
  spread: false,
  children: items.map((children) => ({ type: "listItem", spread: false, children })),
});
const footnote = (n: number): MdNode => ({ type: "footnoteReference", identifier: String(n), label: String(n) });
// A gml/ path or citation, linking to its lines in the source viewer where the page's does (components.tsx's
// SourceLink).
const source = (target: string, href: string | undefined): MdNode => (href ? link(absoluteUrl(href), code(target)) : code(target));

// A site path or a page's own anchor as an absolute URL: the files are read away from the site.
function absolute(url: string, page: string): string {
  return absoluteUrl(url.startsWith("#") ? page + url : url);
}

// ---------------------------------------------------------------------------------------------------------
// From the compiled trees

// A fragment's tree (compile.ts's, after its transform) as plain mdast: the wiki's components become what
// they show, a [[link]] a link to the page (a red link its text), and links absolute. `page` is the page's
// path, for its own anchors.
export function markdownTree(root: MdNode, page: string): MdNode {
  return { type: "root", children: convertAll(root.children ?? [], page) };
}

function convertAll(nodes: MdNode[], page: string): MdNode[] {
  const out: MdNode[] = [];
  for (const node of nodes.flatMap((n) => convert(n, page))) {
    // A note follows its word, as the page's superscript does: "ledge[^5]", not "ledge [^5]", which is
    // the space and line break the source has before a <Cite>.
    const last = out.at(-1);
    if (node.type === "footnoteReference" && last?.type === "text") {
      last.value = (last.value ?? "").trimEnd();
      if (last.value === "") out.pop();
    }
    out.push(node);
  }
  return out;
}

function convert(node: MdNode, page: string): MdNode[] {
  switch (node.type) {
    case "mdxjsEsm":
    case "mdxFlowExpression":
    case "mdxTextExpression":
      return [];
    case "mdxJsxFlowElement":
    case "mdxJsxTextElement": {
      const out = component(node, page);
      // Inline content where a block goes (a <Stat /> alone on its line) is a paragraph.
      return node.type === "mdxJsxFlowElement" && out.length > 0 && out.every(isPhrasing) ? [paragraph(...out)] : out;
    }
    default: {
      // Without its position and data (the heading's anchor, which Markdown makes itself).
      const { position: _position, data: _data, children, ...rest } = node;
      const out: MdNode = { ...rest };
      if (typeof out.url === "string") out.url = absolute(out.url, page);
      if (children) out.children = convertAll(children, page);
      return [out];
    }
  }
}

const PHRASING = new Set(["text", "link", "inlineCode", "strong", "emphasis", "delete", "break", "image", "footnoteReference"]);
const isPhrasing = (node: MdNode) => PHRASING.has(node.type);

// The wiki's components (compile.ts's `component`, with the attributes it gave them), and HTML elements.
function component(node: MdNode, page: string): MdNode[] {
  const attributes = (node.attributes ?? []) as { name?: string; value?: unknown }[];
  const attr = (name: string) => {
    const value = attributes.find((a) => a.name === name)?.value;
    return typeof value === "string" ? value : undefined;
  };
  const children = convertAll(node.children ?? [], page);
  switch (node.name) {
    case "WikiLink": {
      const href = attr("href");
      return href ? [link(absolute(href, page), ...children)] : children;
    }
    case "Stat":
      return [text(attr("text") ?? "")];
    case "Cite":
      return [footnote(Number(attr("n")))];
    case "Clip":
      return clip({ title: attr("title") ?? "", description: attr("description") ?? "", mp4: attr("mp4") });
    case "Sprite": {
      const src = attr("src");
      const alt = attr("alt") ?? "";
      if (src) return [{ type: "image", url: absolute(src, page), alt, title: null }];
      return alt ? [text(alt)] : [];
    }
    case "GmlExcerpt": {
      const cite = attr("cite") ?? "";
      return [{ type: "code", lang: "gml", meta: cite, value: attr("code") ?? "" }, paragraph(...gmlCaption(cite, attr("href")))];
    }
    case "Toc":
      return [];
    case "br":
      return [{ type: "break" }];
    default:
      // Other HTML (<sub>, a fragment): what's in it.
      return children;
  }
}

// A clip: its title and description, as under the video, and the video itself when the media have it.
function clip({ title, description, mp4 }: { title: string; description: string; mp4?: string }): MdNode[] {
  return [
    strong(text(`Clip: ${title}.`)),
    text(` ${description}`),
    ...(mp4 ? [text(" ("), link(absolute(mp4, "/"), text("video")), text(")")] : []),
  ];
}

// The license's caption under a GML excerpt, as the page's (components.tsx's GmlExcerpt;
// docs/wiki-license.md, "GML excerpt caption").
function gmlCaption(cite: string, href: string | undefined): MdNode[] {
  return [
    source(cite, href),
    text(", from "),
    code("spelunky.gmk"),
    text(", Spelunky Classic 1.1's source, © 2008, 2009 Derek Yu and Mossmouth, LLC ("),
    link(LICENSE_URL, text(NOTICE.license)),
    text(")."),
  ];
}

// ---------------------------------------------------------------------------------------------------------
// A page

// A fragment's content inline: its paragraphs' (a lede, an answer, an infobox value are one).
function inline(root: MdNode): MdNode[] {
  return (root.children ?? []).flatMap((node) => (node.type === "paragraph" ? (node.children ?? []) : [node]));
}

// The page, in the layout's order (layout.tsx): the title and where it's from, the hatnote, the lede, the
// infobox as a table, the body, then the FAQ, the Gallery, See also and References. Not the navigation (the
// table of contents, What links here, the navboxes).
export function pageMarkdown(meta: PageMeta, parts: PageMarkdown): MdNode[] {
  const path = `/wiki/${meta.slug}`;
  const url = SITE_URL + path;
  const nodes: MdNode[] = [
    heading(1, meta.title),
    paragraph(text("From the spelunky.ai wiki: "), link(url, text(url)), text(".")),
  ];
  if (meta.spoiler) nodes.push(paragraph(text("This page describes a secret.")));
  if (parts.hatnote) nodes.push(paragraph({ type: "emphasis", children: inline(parts.hatnote) }));
  nodes.push(...(parts.lede.children ?? []));
  nodes.push(...infobox(meta, parts.infobox));
  nodes.push(...(parts.body.children ?? []));
  if (meta.questions.length > 0) {
    nodes.push(heading(2, "FAQ"));
    meta.questions.forEach((question, i) => nodes.push(heading(3, question), ...(parts.answers[i].children ?? [])));
  }
  if (meta.gallery.length > 0) {
    nodes.push(heading(2, "Gallery"));
    const clips = new Map(meta.clips.map((c) => [c.id, c]));
    for (const id of meta.gallery) nodes.push(paragraph(...galleryClip(clips.get(id)!)));
  }
  if (meta.seeAlso.length > 0) {
    nodes.push(heading(2, "See also"));
    nodes.push(list(meta.seeAlso.map(({ label, links }) => [paragraph(strong(text(`${label}:`)), text(" "), ...relationLinks(links))])));
  }
  nodes.push(heading(2, "References"));
  for (const ref of meta.references) {
    nodes.push({
      type: "footnoteDefinition",
      identifier: String(ref.n),
      label: String(ref.n),
      children: [paragraph(...reference(ref))],
    });
  }
  nodes.push(heading(3, "Source code"));
  nodes.push(list(meta.sources.map(({ path, href }) => [paragraph(source(path, href))])));
  return nodes;
}

const galleryClip = (c: ClipInfo) => clip({ title: c.title, description: c.description, mp4: c.files?.mp4 });

// The infobox: its sprite, then a table of its fields (each with its notes), the objects, a quirk's fields
// and the relations it shows.
function infobox(meta: PageMeta, fragments: Record<string, MdNode>): MdNode[] {
  const rows: MdNode[][][] = [];
  for (const row of meta.infobox) {
    const fragment = fragments[row.field];
    const value = fragment ? inline(fragment) : [text(row.text ?? "")];
    rows.push([[text(row.label)], [...value, ...row.notes.map(footnote)]]);
  }
  if (meta.objects.length > 0) {
    const objects = meta.objects.flatMap((object, i) => [...(i > 0 ? [text(", ")] : []), code(object)]);
    rows.push([[text(meta.objects.length === 1 ? "Object" : "Objects")], objects]);
  }
  // A quirk box's fields, as the layout's QuirkRows has them: its rows quoted after their section's link.
  if (meta.quirk) {
    const { status, where, whereHref, pinned_by, entry, rows: quoted, section, href } = meta.quirk;
    rows.push([[text("Status")], [text(status)]], [[text("Where")], [source(where, whereHref)]], [[text("Pinned by")], [code(pinned_by)]]);
    const quotes = quoted.map((row) => text(` “${row}”`));
    rows.push([[text(quoted.length === 1 ? "Row" : "Rows")], [link(href, text(`docs/quirks.md §${entry}, ${section}`)), text(":"), ...quotes]]);
  }
  for (const relation of meta.relations) rows.push([[text(relation.label)], relationLinks(relation.links)]);
  const image: MdNode[] = meta.image?.src
    ? [paragraph({ type: "image", url: absolute(meta.image.src, "/"), alt: meta.image.alt, title: null })]
    : [];
  if (rows.length === 0) return image;
  const cell = (children: MdNode[]): MdNode => ({ type: "tableCell", children });
  const head = [[text(meta.quirk ? "Quirk" : meta.title)], []];
  return [
    ...image,
    {
      type: "table",
      align: [null, null],
      children: [head, ...rows].map((cells) => ({ type: "tableRow", children: cells.map(cell) })),
    },
  ];
}

// A relation's links, one after another: each a link to its page (a red link, its title), its note and its
// citations.
function relationLinks(links: RelationLink[]): MdNode[] {
  return links.flatMap((l, i) => [
    ...(i > 0 ? [text(", ")] : []),
    l.href ? link(absoluteUrl(l.href), text(l.text)) : text(l.text),
    ...(l.note === undefined ? [] : [text(` (${l.note})`)]),
    ...l.notes.map(footnote),
  ]);
}

// A citation, as References lists it (layout.tsx's Reference).
function reference({ kind, target, href }: Reference): MdNode[] {
  switch (kind) {
    case "gml":
      return [source(target, href)];
    case "replay":
      return [text("Replay "), code(target)];
    case "clip":
      return [text(`Clip ${target}`)];
    case "quirk":
      return [text(`Quirk ${target} (docs/quirks.md)`)];
    case "room":
      return [code(target), text(", as the room editor placed it")];
  }
}

// ---------------------------------------------------------------------------------------------------------
// The files

const notice = (audio: boolean): MdNode[] => [
  paragraph(text(NOTICE.before), link(LICENSE_URL, text(NOTICE.license)), text(NOTICE.after)),
  ...(audio ? [paragraph(text(AUDIO_CREDIT))] : []),
];

function write(nodes: MdNode[]): string {
  const root = { type: "root", children: nodes } as Parameters<typeof toMarkdown>[0];
  return toMarkdown(root, { extensions: [gfmToMarkdown()], bullet: "-", rule: "-" });
}

// Its footnotes' labels prefixed with the page's slug, so that pages put together don't share them.
function prefixFootnotes(node: MdNode, slug: string): MdNode {
  const out: MdNode = { ...node };
  if (node.type === "footnoteReference" || node.type === "footnoteDefinition") {
    out.identifier = `${slug}-${node.identifier}`;
    out.label = `${slug}-${node.label}`;
  }
  if (node.children) out.children = node.children.map((child) => prefixFootnotes(child, slug));
  return out;
}

export interface MarkdownPage {
  meta: PageMeta;
  markdown: MdNode[];
}

// The files, by their path on the site: wiki/<slug>.md for each page, llms.txt and llms-full.txt.
export function markdownFiles(pages: MarkdownPage[]): Map<string, string> {
  const files = new Map<string, string>();
  for (const { meta, markdown } of pages) files.set(`wiki/${meta.slug}.md`, write([...notice(meta.audio), ...markdown]));

  // By category, in the hubs' order, each by title: a page in two categories is under both.
  const byTitle = [...pages].sort((a, b) => a.meta.title.localeCompare(b.meta.title, "en"));
  const sections = CATEGORIES.map((category) => ({
    category,
    pages: byTitle.filter(({ meta }) => meta.categories.includes(category)),
  })).filter((section) => section.pages.length > 0);
  const intro = [heading(1, SITE_TITLE), { type: "blockquote", children: [paragraph(text(SITE_SUMMARY))] }];

  // https://llmstxt.org: the site's name, a summary, what the links are, then a list of them per section,
  // each a page's twin and its lede (its first sentence, a direct answer: docs/wiki-style.md, "The lede").
  files.set(
    "llms.txt",
    write([
      ...notice(false),
      ...intro,
      paragraph(
        text("Each link below is a wiki page's Markdown twin: the page at "),
        code(`${SITE_URL}/wiki/<slug>`),
        text(" without the site around it, with its infobox, sections, FAQ and references. "),
        link(`${SITE_URL}/llms-full.txt`, text("Every page in one file")),
        text(
          ". The wiki is part of spelunky.ai, an unofficial tick-for-tick Rust port of Spelunky Classic 1.1, " +
            "not affiliated with or endorsed by Mossmouth.",
        ),
      ),
      ...sections.flatMap(({ category, pages }) => [
        heading(2, CATEGORY_NAMES[category]),
        list(pages.map(({ meta }) => [paragraph(link(`${SITE_URL}/wiki/${meta.slug}.md`, text(meta.title)), text(`: ${meta.lede}`))])),
      ]),
      heading(2, "Optional"),
      list([
        [paragraph(link(`${SITE_URL}/play/`, text("Play Spelunky Classic")), text(": the spelunky.ai port, in the browser."))],
        [paragraph(link(LICENSE_URL, text(NOTICE.license)), text(": the license the game and its assets are used under."))],
      ]),
    ]),
  );

  // Every page once, in llms.txt's order, under the site's title.
  const seen = new Set<string>();
  const all = sections.flatMap((section) => section.pages).filter(({ meta }) => !seen.has(meta.slug) && seen.add(meta.slug));
  files.set(
    "llms-full.txt",
    write([
      ...notice(all.some(({ meta }) => meta.audio)),
      ...intro,
      ...all.flatMap(({ meta, markdown }) => [{ type: "thematicBreak" }, ...markdown.map((node) => prefixFootnotes(node, meta.slug))]),
    ]),
  );
  return files;
}
