// The wiki's keyline (docs/wiki-style.md, "The keyline"), which site/test.sh runs: every block in a column shows
// its left edge on the column's. The site's tests have no browser to ask where a block's edge is, so this
// reads src/wiki/wiki.css instead, for the three ways a block has come off the line:
//
//   - a ring or frame drawn outside its box (box-shadow: 0 0 0 var(--ring) ..., or var(--stone)), with the box
//     on the keyline and the ring past it. A block with one is inset by it, margin-inline: var(--ring) or
//     var(--frame), or sits in one that is (INSET_BY), or isn't in a column at all (OUT_OF_COLUMN).
//   - a left margin of its own (the /wiki search box's and card grid's were 8px): only the tokens, or a list
//     item's marker room in ch, unless it's one of SIDE_MARGINS. (A right margin alone is a gap in a row.)
//   - a list's markers indented by the browser's 40px: lists have their markers' width, in ch.
//
//   node test-keyline.mjs
import { readFileSync } from "node:fs";

// Selector → where it is instead: a box drawn with a ring whose ring isn't on a column's keyline.
const OUT_OF_COLUMN = {
  ".search-popup": "Base UI's positioner places it, under the header's search box",
  ".hover-card": "Base UI's positioner places it, by its link",
  ".clip-progress-track": "centred over the clip's poster",
};
// Selector → the block whose inset holds it on the keyline: one of a row or grid of boxes with rings.
const INSET_BY = {
  ".category": ".categories",
  ".clip-slider-track": ".clip-slider",
};
// Selector → why its left margin isn't a token.
const SIDE_MARGINS = {
  ".infobox": "floated right: its left margin is the text's room beside it, and its right one the frame's",
  ".visually-hidden": "out of the flow, a pixel square",
  ".wiki-page > .toc-rail": "the rail's text is on the page's keyline, its entries' highlights in the gutter (--rail-inset)",
};

// The stylesheet's rules, with the media query each is in: { media, selectors, decls: [[property, value]] }.
function parseCss(source) {
  const rules = [];
  const parse = (src, media) => {
    let i = 0;
    for (;;) {
      const open = src.indexOf("{", i);
      if (open < 0) return;
      const head = src.slice(i, open).trim();
      let depth = 1;
      let j = open + 1;
      for (; depth > 0; j++) {
        if (j >= src.length) throw new Error(`unbalanced braces after ${head}`);
        if (src[j] === '"' || src[j] === "'") j = src.indexOf(src[j], j + 1);
        else if (src[j] === "{") depth++;
        else if (src[j] === "}") depth--;
      }
      const body = src.slice(open + 1, j - 1);
      if (head.startsWith("@media")) parse(body, head.slice("@media".length).trim());
      // Any other block's rules (@supports, @layer, @container) would be read as one rule: say so, rather than
      // leave them unchecked.
      else if (head.startsWith("@")) throw new Error(`wiki.css: ${head.split(/\s/)[0]}, which the keyline's check doesn't read`);
      else {
        const decls = splitTop(body, ";")
          .map((d) => d.trim())
          .filter(Boolean)
          .map((d) => [d.slice(0, d.indexOf(":")).trim(), d.slice(d.indexOf(":") + 1).trim()]);
        rules.push({ media, selectors: splitTop(head, ",").map((s) => s.trim()), decls });
      }
      i = j;
    }
  };
  parse(source.replace(/\/\*[\s\S]*?\*\//g, ""), null);
  return rules;
}

// SRC split on SEP outside parentheses (a selector's :is(a, b), a shadow's calc(2 * x)) and strings (a
// content: "·" / "").
function splitTop(src, sep) {
  const parts = [];
  let depth = 0;
  let quote = null;
  let cur = "";
  for (const ch of src) {
    if (quote) {
      if (ch === quote) quote = null;
    } else if (ch === '"' || ch === "'") quote = ch;
    else if (ch === "(") depth++;
    else if (ch === ")") depth--;
    if (ch === sep && depth === 0 && !quote) {
      parts.push(cur);
      cur = "";
    } else cur += ch;
  }
  parts.push(cur);
  return parts;
}

// A value's space-separated words, keeping var(...) and calc(...) whole.
const words = (value) => splitTop(value.trim().replace(/\s+/g, " "), " ").filter(Boolean);

// The left and right margins a rule's margin declarations give, or undefined where it gives none.
function sideMargins(decls) {
  let left;
  let right;
  for (const [p, v] of decls) {
    const w = words(v);
    if (p === "margin") {
      right = w[1] ?? w[0];
      left = w[3] ?? w[1] ?? w[0];
    } else if (p === "margin-inline") {
      left = w[0];
      right = w[1] ?? w[0];
    } else if (p === "margin-left" || p === "margin-inline-start") left = v;
    else if (p === "margin-right" || p === "margin-inline-end") right = v;
  }
  return { left, right };
}

// The width a box-shadow draws outside its box on the left: "ring", "frame", or null for none; or a string
// naming a shadow the check can't read.
function outsideEdge(value) {
  if (value === "var(--stone)") return "frame";
  if (value === "none" || value.startsWith("var(--bevel)")) return null;
  let edge = null;
  for (const shadow of splitTop(value, ",")) {
    const w = words(shadow);
    if (w.includes("inset")) continue;
    const [x, y, blur, spread] = w;
    if (x !== "0" || y !== "0" || blur !== "0") return `an offset or blurred shadow, ${shadow.trim()}`;
    if (spread === "var(--ring)") edge = edge ?? "ring";
    else if (spread === "var(--frame)") edge = "frame";
    else return `a ring of ${spread}, not var(--ring) or var(--frame)`;
  }
  return edge;
}

const ZERO = new Set(["0", "auto", undefined]);

// The keyline's problems in SOURCE, wiki.css's text: a message each.
function keylineProblems(source) {
  const rules = parseCss(source);
  const problems = [];
  const declsOf = (selector) => rules.filter((r) => r.selectors.includes(selector));
  const value = (selector, property) =>
    declsOf(selector)
      .flatMap((r) => r.decls)
      .filter(([p]) => p === property)
      .map(([, v]) => v);

  // The tokens.
  const root = Object.fromEntries(declsOf(":root").flatMap((r) => r.decls));
  for (const [token, want] of [
    ["--gutter", null],
    ["--ring", "var(--px)"],
    ["--frame", "calc(2 * var(--px))"],
  ]) {
    if (!(token in root)) problems.push(`:root doesn't define ${token}`);
    else if (want && root[token] !== want) problems.push(`:root's ${token} is ${root[token]}, not ${want}`);
  }
  if (!value(".wiki", "padding").includes("var(--gutter)")) problems.push(".wiki's padding isn't var(--gutter): the keyline is --gutter in");

  // Rings and frames outside a box: inset by them.
  for (const rule of rules) {
    for (const [p, v] of rule.decls) {
      if (p !== "box-shadow") continue;
      const edge = outsideEdge(v);
      if (edge === null) continue;
      for (const selector of rule.selectors) {
        if (edge !== "ring" && edge !== "frame") {
          problems.push(`${selector}: box-shadow draws ${edge}, which the keyline's check can't read`);
          continue;
        }
        // A state's ring (a:hover's, [data-pressed]'s) is the block's own, which its base selector insets.
        const base = selector.replace(/:(hover|focus|focus-visible|focus-within|active)\b|\[data-[\w-]+\]/g, "");
        if (base in OUT_OF_COLUMN) continue;
        const holder = INSET_BY[base] ?? base;
        const token = `var(--${edge})`;
        const margins = rules.filter((r) => r.selectors.includes(holder)).map((r) => sideMargins(r.decls));
        const lefts = margins.map((m) => m.left).filter((m) => m !== undefined);
        if (lefts.length === 0) problems.push(`${selector}: its ${edge} is outside its box, but ${holder} isn't inset by it (margin-inline: ${token})`);
        for (const left of lefts) {
          // .infobox's left is SIDE_MARGINS's: its right holds its frame.
          const side = holder in SIDE_MARGINS ? margins.map((m) => m.right).filter((m) => m !== undefined) : [left];
          for (const m of side) {
            if (m !== token) problems.push(`${selector}: its ${edge} is outside its box, but ${holder}'s margin is ${m}, not ${token}`);
          }
        }
      }
    }
  }

  // Left margins: tokens, or a list item's marker room.
  for (const rule of rules) {
    const { left } = sideMargins(rule.decls);
    if (ZERO.has(left) || left === "var(--ring)" || left === "var(--frame)") continue;
    for (const selector of rule.selectors) {
      if (selector in SIDE_MARGINS) continue;
      if (/\bli$/.test(selector) && /^\d+ch$/.test(left)) continue;
      problems.push(`${selector}: a left margin of ${left}, which moves it off the keyline (a ring's is var(--ring), a frame's var(--frame))`);
    }
  }

  // Lists: their markers on the keyline.
  for (const [selector, want] of [
    [":where(ul)", "2ch"],
    [":where(ol)", "3ch"],
    [":where(ol:has(> li:nth-child(10)))", "4ch"],
  ]) {
    if (!value(selector, "padding-left").includes(want)) problems.push(`${selector}'s padding-left isn't ${want}, its markers' width`);
  }
  return problems;
}

let failures = 0;
const fail = (message) => {
  console.error(`FAIL ${message}`);
  failures++;
};

const css = readFileSync(new URL("src/wiki/wiki.css", import.meta.url), "utf8");
const problems = keylineProblems(css);
for (const problem of problems) fail(`wiki.css: ${problem}`);
if (problems.length === 0) console.log("ok   wiki.css: every block's edge is on the keyline");

// And it catches each way off it: the stylesheet with one thing changed, and the problem that names it.
const cases = [
  ["margin: 16px 0; padding: 0; list-style: none;\n}\n.cards-wide", "margin: 16px 8px; padding: 0; list-style: none;\n}\n.cards-wide", ".cards: a left margin of 8px"],
  ["margin: 16px var(--ring) 8px; padding: 12px;", "margin: 16px 0 8px; padding: 12px;", ".hub-filters: its ring is outside its box, but .hub-filters's margin is 0, not var(--ring)"],
  [".gml pre { margin: 0 var(--ring);", ".gml pre { margin: 0;", ".gml pre: its ring is outside its box, but .gml pre's margin is 0, not var(--ring)"],
  [".navbox { margin: 0 var(--frame);", ".navbox { margin: 0 var(--ring);", ".navbox: its frame is outside its box, but .navbox's margin is var(--ring), not var(--frame)"],
  ["box-shadow: 0 0 0 var(--ring) var(--dirt); }\n.category-link", "box-shadow: 0 0 0 4px var(--dirt); }\n.category-link", ".category: box-shadow draws a ring of 4px"],
  ["margin: 16px var(--ring); padding: 0;\n  list-style: none;", "margin: 16px 0; padding: 0;\n  list-style: none;", ".category: its ring is outside its box, but .categories's margin is 0, not var(--ring)"],
  [":where(ul) { padding-left: 2ch; }", ":where(ul) { padding-left: 40px; }", ":where(ul)'s padding-left isn't 2ch"],
];
for (const [from, to, want] of cases) {
  if (!css.includes(from)) {
    fail(`self-test: wiki.css hasn't got ${JSON.stringify(from)} to change`);
    continue;
  }
  const got = keylineProblems(css.replace(from, to));
  if (!got.some((p) => p.startsWith(want))) fail(`self-test: ${JSON.stringify(to)} isn't caught as ${JSON.stringify(want)}: ${JSON.stringify(got)}`);
}
if (failures === 0) console.log(`ok   self-test: ${cases.length} ways off the keyline caught`);

if (failures > 0) {
  console.error(`site/test-keyline.mjs: ${failures} failed`);
  process.exit(1);
}
console.log("site/test-keyline.mjs: all passed");
