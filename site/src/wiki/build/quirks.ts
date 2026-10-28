// A quirk page's rows in docs/quirks.md (docs/wiki-style.md, "Quirk pages"). The row is the source of truth
// and the page its readable version, so the page names the rows it's about by how they start, and the
// build fails on one docs/quirks.md hasn't got: a row reworded or removed takes its page with it.
import { readFileSync } from "node:fs";

// A numbered section of docs/quirks.md ("## 1. Bugs and leftovers in the 1.1 source"): its title, where it
// is on GitHub, and its table's rows as plain text.
export interface QuirkSection {
  title: string;
  href: string;
  rows: string[];
}

const QUIRKS_URL = "https://github.com/851-labs/spelunky/blob/main/docs/quirks.md";

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

// docs/quirks.md's numbered sections, by number.
export function readQuirks(path: string): Map<string, QuirkSection> {
  const sections = new Map<string, QuirkSection>();
  let current: QuirkSection | null = null;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const heading = /^## (\d+)\. (.+)$/.exec(line);
    if (heading) {
      current = { title: heading[2].trim(), href: `${QUIRKS_URL}#${anchor(`${heading[1]}. ${heading[2]}`)}`, rows: [] };
      sections.set(heading[1], current);
    } else if (line.startsWith("## ")) {
      current = null;
    } else if (current && line.startsWith("| ") && !/^\|[-| ]+\|$/.test(line)) {
      current.rows.push(plainRow(line.slice(2)));
    }
  }
  // Each section's first row is its table's header.
  for (const section of sections.values()) section.rows.shift();
  return sections;
}

// What's wrong with a quirk page's entry and rows, if anything.
export function checkQuirk(quirk: { entry: string; rows: string[] }, sections: Map<string, QuirkSection>): string[] {
  const section = sections.get(quirk.entry);
  if (!section) return [`quirk.entry: docs/quirks.md has no section ${quirk.entry} (${[...sections.keys()].join(", ")})`];
  return quirk.rows
    .filter((row) => !section.rows.some((text) => text.startsWith(plainRow(row))))
    .map((row) => `quirk.rows: no row of docs/quirks.md §${quirk.entry} starts ${JSON.stringify(row)}`);
}
