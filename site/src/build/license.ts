// "virtual:site/license": the Spelunky User License, for the /license page (src/routes/license.tsx), read from the
// repo's LICENSE when the build starts, so the page is the file and there's no copy to keep in step (851-4026).
// LICENSE is the original's COPYING.txt, in windows-1252 (its ® is the byte AE), which a `?raw` import would read
// as UTF-8, so it's decoded here.
//
// The file is plain text laid out by hand: its title between two rules of =, a section's heading between two of -,
// paragraphs a blank line apart, and the conditions of "Rights Granted" numbered (1) to (4), each a paragraph of
// its own. The parse keeps every word as the file has it, only the line breaks and the rules go, and checks it:
// a file it can't read whole fails the build, rather than put a license with a part missing on the page.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import type { Plugin } from "vite";

// A condition: its number as the file writes it, "(1)", and its text.
export interface Condition {
  marker: string;
  text: string;
}

export type Block = { kind: "paragraph"; text: string } | { kind: "conditions"; items: Condition[] };

export interface LicenseSection {
  // "1. Preamble".
  heading: string;
  blocks: Block[];
}

export interface License {
  // "Spelunky® User License v1.1b".
  title: string;
  // What's between the title and the first section: the copyright line and the date it was last updated.
  header: string[];
  sections: LicenseSection[];
}

const ID = "virtual:site/license";
const FILE = fileURLToPath(new URL("../../../LICENSE", import.meta.url));

export function siteLicense(file = FILE): Plugin {
  let license: License | null = null;
  return {
    name: "site-license",
    // Once, as the build starts, so a file it can't read fails it before anything is rendered. `vite dev` reads it
    // again on a restart, not as it changes: the file hasn't since 2009.
    buildStart() {
      license = parseLicense(new TextDecoder("windows-1252").decode(readFileSync(file)), file);
    },
    resolveId: (id) => (id === ID ? `\0${ID}` : undefined),
    load: (id) => (id === `\0${ID}` ? `export const license = ${JSON.stringify(license)};` : undefined),
  };
}

const RULE = /^(=+|-+)$/;
const CONDITION = /^\((\d+)\)\s+/;

export function parseLicense(text: string, name = "LICENSE"): License {
  const fail = (what: string): never => {
    throw new Error(`${name}: ${what}, so the /license page can't show it (src/build/license.ts)`);
  };
  const lines = text.replace(/\r\n?/g, "\n").split("\n").map((line) => line.trimEnd());
  if (!/^=+$/.test(lines[0] ?? "") || !lines[1]?.trim() || !/^=+$/.test(lines[2] ?? "")) {
    fail("no title between two rules of = at the top");
  }
  const title = lines[1].trim();
  const header: string[] = [];
  const sections: LicenseSection[] = [];
  // The paragraph being read, a line at a time.
  let paragraph: string[] = [];
  const end = () => {
    if (paragraph.length === 0) return;
    const joined = paragraph.join(" ");
    paragraph = [];
    const condition = CONDITION.exec(joined);
    const section = sections.at(-1);
    if (!section) {
      if (condition) fail(`a condition, ${condition[0].trim()}, before the first section`);
      header.push(joined);
      return;
    }
    if (!condition) {
      section.blocks.push({ kind: "paragraph", text: joined });
      return;
    }
    const item = { marker: `(${condition[1]})`, text: joined.slice(condition[0].length) };
    const last = section.blocks.at(-1);
    if (last?.kind === "conditions") last.items.push(item);
    else section.blocks.push({ kind: "conditions", items: [item] });
  };
  for (let i = 3; i < lines.length; i++) {
    const line = lines[i];
    // A heading: a line between two rules of -.
    if (/^-+$/.test(line) && lines[i + 1]?.trim() && /^-+$/.test(lines[i + 2] ?? "")) {
      end();
      sections.push({ heading: lines[i + 1].trim(), blocks: [] });
      i += 2;
    } else if (line.trim() === "") {
      end();
    } else if (RULE.test(line)) {
      fail(`line ${i + 1}, a rule that's no heading's`);
    } else {
      // A condition's lines after its first are indented under its text: the indent goes, as the line breaks do.
      paragraph.push(line.trim());
    }
  }
  end();
  if (sections.length === 0) fail("no sections");
  // Every word the file has, in its order, and nothing else: the parse dropped and changed none.
  const words = (s: string) => s.split(/\s+/).filter(Boolean);
  const source = words(lines.filter((line) => !RULE.test(line)).join(" "));
  const parsed = words(
    [
      title,
      ...header,
      ...sections.flatMap((section) => [
        section.heading,
        ...section.blocks.flatMap((block) =>
          block.kind === "paragraph" ? [block.text] : block.items.map((item) => `${item.marker} ${item.text}`),
        ),
      ]),
    ].join(" "),
  );
  const differs = source.findIndex((word, i) => parsed[i] !== word);
  if (differs !== -1 || parsed.length !== source.length) {
    const at = differs === -1 ? Math.min(source.length, parsed.length) : differs;
    fail(`the parse doesn't have the file's words, from word ${at + 1} ("${source.slice(at, at + 6).join(" ")}")`);
  }
  return { title, header, sections };
}
