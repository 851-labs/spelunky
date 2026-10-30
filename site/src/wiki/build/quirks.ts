// A quirk page's rows in docs/quirks.md (docs/wiki-style.md, "Quirk pages"). The row is the source of truth
// and the page its readable version, so the page names the rows it's about by how they start, and the
// build fails on one docs/quirks.md hasn't got: a row reworded or removed takes its page with it.
import { readFileSync } from "node:fs";

import { repoUrl } from "./repo.ts";

// A numbered section of docs/quirks.md ("## 1. Bugs and leftovers in the 1.1 source"): its title, where it
// is on GitHub, and its table's rows.
export interface QuirkSection {
  title: string;
  href: string;
  rows: QuirkRow[];
}

// A row: its text, plain (plainRow), its line in docs/quirks.md, and that line on GitHub.
export interface QuirkRow {
  text: string;
  line: number;
  href: string;
}

const QUIRKS_FILE = "docs/quirks.md";

// What a row and a page's `rows` are compared as: without Markdown's bold and code marks, links (their text
// kept) and table escapes, so a page's plain-text row matches a row that bolds its first sentence or writes a
// name in code.
export const plainRow = (text: string) =>
  text
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replaceAll("\\|", "|")
    .replaceAll("**", "")
    .replaceAll("`", "")
    .replace(/\s+/g, " ")
    .trim();

// GitHub's anchor for a heading: lowercase, punctuation but hyphens and underscores dropped, spaces as
// hyphens. (The sections are numbered, so no two headings are the same and none takes GitHub's -1 suffix.)
const anchor = (heading: string) =>
  heading
    .toLowerCase()
    .replace(/[^\p{L}\p{N} _-]/gu, "")
    .replaceAll(" ", "-");

// docs/quirks.md's numbered sections, by number, linked on GitHub at `commit` (repo.ts).
export function readQuirks(path: string, pin: (path: string) => string): Map<string, QuirkSection> {
  const commit = pin(QUIRKS_FILE);
  const sections = new Map<string, QuirkSection>();
  let current: QuirkSection | null = null;
  for (const [i, line] of readFileSync(path, "utf8").split("\n").entries()) {
    const heading = /^## (\d+)\. (.+)$/.exec(line);
    if (heading) {
      current = { title: heading[2].trim(), href: `${repoUrl(commit, QUIRKS_FILE)}#${anchor(`${heading[1]}. ${heading[2]}`)}`, rows: [] };
      sections.set(heading[1], current);
    } else if (line.startsWith("## ")) {
      current = null;
    } else if (current && line.startsWith("| ") && !/^\|[-| ]+\|$/.test(line)) {
      current.rows.push({ text: plainRow(line.slice(2)), line: i + 1, href: repoUrl(commit, QUIRKS_FILE, i + 1) });
    }
  }
  // Each section's first row is its table's header.
  for (const section of sections.values()) section.rows.shift();
  return sections;
}

// The row of a section a page names by its opening (a quirk box's `rows`, a citation's `row=`), or what's wrong
// with the name: no row starts with it, or more than one does.
export function findRow(section: QuirkSection, entry: string, name: string): QuirkRow | string {
  const found = section.rows.filter((row) => row.text.startsWith(plainRow(name)));
  if (found.length === 1) return found[0];
  return found.length === 0
    ? `no row of docs/quirks.md §${entry} starts ${JSON.stringify(name)}`
    : `${found.length} rows of docs/quirks.md §${entry} start ${JSON.stringify(name)} (lines ${lines(found)}): quote more of it`;
}

// The first few rows' lines, and how many more.
const lines = (rows: QuirkRow[]) => rows.slice(0, 3).map((row) => row.line).join(", ") + (rows.length > 3 ? ` and ${rows.length - 3} more` : "");

// What's wrong with a quirk page's entry and rows, if anything.
export function checkQuirk(quirk: { entry: string; rows: string[] }, sections: Map<string, QuirkSection>): string[] {
  const section = sections.get(quirk.entry);
  if (!section) return [`quirk.entry: ${noSection(quirk.entry, sections)}`];
  return quirk.rows.flatMap((name) => {
    const row = findRow(section, quirk.entry, name);
    return typeof row === "string" ? [`quirk.rows: ${row}`] : [];
  });
}

export const noSection = (entry: string, sections: Map<string, QuirkSection>) =>
  `docs/quirks.md has no section ${entry} (${[...sections.keys()].join(", ")})`;
