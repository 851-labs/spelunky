// The wiki's search: an inverted index the build makes of every page (src/wiki/build/plugin.ts's
// `virtual:wiki/search-index`), which the browser loads the first time a search box is used and queries
// itself, so search needs no server (docs/wiki-style.md, "The pipeline"). The same code builds it, in Node, and
// queries it, in the browser: no dependencies.
//
// A page's words score by where they are: its title most, then its aliases, its section headings, its lede
// and its categories, and its text least, by how often (up to a cap, so a long page doesn't win on length).
// Every word of a query must match: as a word of the index, the start of one (results as you type: "snak"),
// or one with up to two letters more ("snakes" finds "snake"); the closer the match, the higher it scores.

// What the build indexes of a page.
export interface SearchSource {
  slug: string;
  title: string;
  aliases: string[];
  lede: string;
  categories: string[];
  sections: { id: string; title: string }[];
  // Its text: the body and the FAQ's questions and answers, as plain text.
  text: string;
}

// What a result shows of its page: its title, its first category, its lede, and the sections a query's
// words can name (as `[anchor, title]`).
export interface SearchDoc {
  slug: string;
  title: string;
  category: string;
  lede: string;
  sections: [string, string][];
}

export interface SearchIndex {
  docs: SearchDoc[];
  // Every word, sorted, so the words a prefix starts are a run, found by binary search.
  terms: string[];
  // Each word's pages and scores, flat: [doc, score, doc, score, ...].
  postings: number[][];
}

export interface SearchHit {
  doc: SearchDoc;
  score: number;
  // The page's sections whose headings have one of the query's words, in the page's order.
  sections: [string, string][];
}

// Words too common to tell pages apart, left out of the index and (unless it's all there is) the query.
const STOP_WORDS = new Set(
  ("a an and are as at be but by can do does for from has have he her him his how if in into is it its of on " +
    "or she so than that the their them then there these they this to was what when where which who why " +
    "will with").split(" "),
);

const WEIGHTS = { title: 16, alias: 12, section: 6, lede: 5, category: 3 };
// A word in the text scores 1 for each time it's there, up to this: less than a lede's.
const TEXT_CAP = 4;
// How many letters a query's word may have beyond the index's ("snakes" for "snake").
const STEM = 2;

// The words of a text: lowercase letters and digits, accents dropped ("Olmec's" is "olmec" and "s").
export function words(text: string): string[] {
  return (
    text
      .normalize("NFKD")
      .replace(/\p{M}/gu, "")
      .toLowerCase()
      .match(/[\p{L}\p{N}]+/gu) ?? []
  );
}

const indexed = (word: string) => word.length > 1 && !STOP_WORDS.has(word);

export function buildSearchIndex(sources: SearchSource[]): SearchIndex {
  const scores = new Map<string, Map<number, number>>();
  const add = (doc: number, word: string, score: number) => {
    let docs = scores.get(word);
    if (!docs) scores.set(word, (docs = new Map()));
    docs.set(doc, (docs.get(doc) ?? 0) + score);
  };
  const docs = sources.map((source, doc): SearchDoc => {
    // Each field's words count once each...
    const field = (text: string, weight: number) => {
      for (const word of new Set(words(text).filter(indexed))) add(doc, word, weight);
    };
    field(source.title, WEIGHTS.title);
    for (const alias of source.aliases) field(alias, WEIGHTS.alias);
    for (const section of source.sections) field(section.title, WEIGHTS.section);
    field(source.lede, WEIGHTS.lede);
    for (const category of source.categories) field(category, WEIGHTS.category);
    // ...and the text's by how often they're there.
    const counts = new Map<string, number>();
    for (const word of words(source.text).filter(indexed)) counts.set(word, (counts.get(word) ?? 0) + 1);
    for (const [word, count] of counts) add(doc, word, Math.min(count, TEXT_CAP));
    return {
      slug: source.slug,
      title: source.title,
      category: source.categories[0] ?? "",
      lede: source.lede,
      sections: source.sections.map(({ id, title }) => [id, title]),
    };
  });
  const terms = [...scores.keys()].sort();
  return { docs, terms, postings: terms.map((term) => [...scores.get(term)!].flat()) };
}

// The query's words, as the index has them: its stop words dropped, unless they're all it has (a search
// for "the" as it's typed on the way to "theme").
export function queryWords(query: string): string[] {
  const all = words(query);
  const kept = all.filter((word) => !STOP_WORDS.has(word));
  return [...new Set(kept.length > 0 ? kept : all)];
}

// The pages that have every word of the query, best first.
export function searchWiki(index: SearchIndex, query: string, limit = Infinity): SearchHit[] {
  const wanted = queryWords(query);
  if (wanted.length === 0) return [];
  let found: Map<number, number> | null = null;
  for (const word of wanted) {
    // A page's best match for the word.
    const best = new Map<number, number>();
    for (const [term, closeness] of matches(index.terms, word)) {
      const postings = index.postings[term];
      for (let i = 0; i < postings.length; i += 2) {
        const score = postings[i + 1] * closeness;
        if (score > (best.get(postings[i]) ?? 0)) best.set(postings[i], score);
      }
    }
    if (found === null) found = best;
    else {
      const both = new Map<number, number>();
      for (const [doc, score] of found) {
        const more = best.get(doc);
        if (more !== undefined) both.set(doc, score + more);
      }
      found = both;
    }
    if (found.size === 0) return [];
  }
  return [...found!]
    .map(([doc, score]) => ({ doc: index.docs[doc], score }))
    .sort((a, b) => b.score - a.score || a.doc.title.localeCompare(b.doc.title, "en"))
    .slice(0, limit)
    .map(({ doc, score }) => ({ doc, score, sections: matchingSections(doc, wanted) }));
}

// The index's words a query's word matches, each with how closely: 1 for the word itself, less the more
// letters the two don't share.
function* matches(terms: string[], word: string): Generator<[number, number]> {
  for (let i = firstAtLeast(terms, word); i < terms.length && terms[i].startsWith(word); i++) {
    yield [i, closeness(word, terms[i])];
  }
  // A word of the index the query's word starts with, a letter or two shorter: a plural, say.
  for (let length = word.length - 1; length >= Math.max(3, word.length - STEM); length--) {
    const shorter = word.slice(0, length);
    const i = firstAtLeast(terms, shorter);
    if (terms[i] === shorter) yield [i, closeness(shorter, word)];
  }
}

const closeness = (shorter: string, longer: string) => 0.5 + (0.5 * shorter.length) / longer.length;

// The first index at which `terms` has `word` or a word after it.
function firstAtLeast(terms: string[], word: string): number {
  let lo = 0;
  let hi = terms.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (terms[mid] < word) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

// A match for a word as a whole word, the start of one, or with a letter or two more (as matches() has it).
export function wordMatches(word: string, wanted: string[]): boolean {
  return wanted.some(
    (want) => word.startsWith(want) || (word.length >= 3 && want.startsWith(word) && want.length - word.length <= STEM),
  );
}

function matchingSections(doc: SearchDoc, wanted: string[]): [string, string][] {
  return doc.sections.filter(([, title]) => words(title).some((word) => wordMatches(word, wanted))).slice(0, 3);
}
