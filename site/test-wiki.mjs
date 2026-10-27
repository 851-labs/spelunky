// The wiki pipeline's checks (docs/wiki-style.md, "The pipeline"), which site/test.sh runs: the committed
// pages compile with no errors, and a page that breaks a rule fails the build with an error naming it. Node
// runs the build's TypeScript as is (type stripping).
//
//   node test-wiki.mjs
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { loadWiki } from "./src/wiki/build/compile.ts";
import { wiki } from "./src/wiki/build/plugin.ts";
import { buildSearchIndex, matchingSections, searchWiki } from "./src/wiki/search.ts";

const here = (path) => new URL(path, import.meta.url).pathname;
const options = (contentDir, plannedFile = here("content/wiki-planned.yaml")) => ({
  contentDir,
  statsFile: here("content/wiki-data/stats.json"),
  relationsFile: here("content/wiki-data/relations.json"),
  plannedFile,
  clipsDir: here("../wiki/clips"),
  relationsSpec: here("../wiki/data/relations.toml"),
  media: null,
});

let failures = 0;
function fail(message) {
  console.error(`FAIL ${message}`);
  failures++;
}

// The committed pages.
const committed = await loadWiki(options(here("content/wiki")));
for (const error of committed.errors) fail(`content/wiki: ${error}`);
if (committed.pages.length === 0) fail("content/wiki: no pages");
const snake = committed.pages.find((page) => page.meta.slug === "snake");
if (!snake) fail("content/wiki: no snake page");
else {
  const { meta } = snake;
  const want = "The Snake is a monster in Spelunky Classic, found mostly in the Mines, that slithers back and forth along the ground and bites for 1 heart on contact.";
  if (meta.lede !== want) fail(`snake: lede ${JSON.stringify(meta.lede)}`);
  if (meta.infobox.find((row) => row.field === "health")?.text !== "1 hit point") fail("snake: infobox health isn't 1 hit point");
  if (meta.clips.map((clip) => clip.id).join() !== "snake/bite,snake/stomp,snake/thrown-rock") fail("snake: clips");
  // The infobox's citations come first, in its order (its data values cite their lines), then the body's.
  const cited = meta.infobox.map((row) => `${row.field} ${row.notes.join()}`).join("; ");
  if (cited !== "health 1; damage 2; speed 3; behaviour 4") fail(`snake: the infobox's notes are ${cited}`);
  if (meta.references[0]?.target !== "objects/oSnake/create.gml:10") fail("snake: the first citation");
  if (meta.references[3]?.target !== "objects/oSnake/step.gml:39-72") fail("snake: the behaviour's citation");
  if (meta.references[4]?.target !== "objects/oSnake/step.gml:41-54") fail("snake: the body's first citation");
  // Its relations: the places it's found in, named by the relations' spec until they have pages.
  const found = meta.relations.map((r) => `${r.label}: ${r.links.map((l) => l.text).join(", ")}`).join("; ");
  if (found !== "Found in: The Mines, Snake pit, Tutorial") fail(`snake: the infobox's relations are ${found}`);
  // The table of contents' sections, with their stable anchors.
  const sections = meta.sections.map((s) => s.id).join();
  if (sections !== "behaviour,damage,defeating,locations,tips,trivia") fail(`snake: sections ${sections}`);
  // The monsters' navbox, with the Snake under the Mines.
  const navbox = meta.navboxes.map((n) => `${n.category}: ${n.groups.map((g) => `${g.area} ${g.links.map((l) => l.slug)}`)}`).join();
  if (navbox !== "monsters: The Mines snake") fail(`snake: navboxes ${navbox}`);
  if (meta.gallery.length !== 0) fail(`snake: the body places every clip, but the gallery has ${meta.gallery}`);
  // Without the media, no VideoObject or ImageObject: there's nothing to point them at.
  if (meta.jsonLd.length !== 0) fail(`snake: media JSON-LD without the media: ${JSON.stringify(meta.jsonLd)}`);
  // Its relations: found_in in the infobox, damaged_by under See also, all red links for now, a cited
  // entry with its note number (the body's, where it cites the same lines).
  const foundIn = meta.relations.find((r) => r.relation === "found_in");
  if (foundIn?.links.map((l) => l.slug).join() !== "mines,snake-pit,tutorial") fail("snake: found_in");
  if (foundIn?.links.some((l) => l.href !== undefined)) fail("snake: found_in's pages aren't written, so they're red links");
  const damagedBy = meta.seeAlso.find((r) => r.relation === "damaged_by");
  const stomping = damagedBy?.links.find((l) => l.slug === "stomping");
  const stompCite = meta.references.find((r) => r.target === "objects/oEnemy/collision_oCharacter.gml:6-10");
  if (!stompCite || stomping?.notes.join() !== String(stompCite.n)) fail("snake: damaged_by's citations");
  if (meta.linksHere.length !== 0) fail("snake: no other page links to it");
}
if (committed.warnings.some((w) => !w.startsWith("no wiki media"))) fail(`content/wiki: warnings ${JSON.stringify(committed.warnings)}`);
// Its red links, listed: each planned page it links to.
const wanted = committed.wanted.map((w) => w.slug);
for (const slug of ["mines", "tutorial", "whip", "level-editor"]) if (!wanted.includes(slug)) fail(`wanted: no ${slug}`);
if (failures === 0) console.log(`ok   content/wiki: ${committed.pages.length} page(s) compile, linking ${wanted.length} planned page(s)`);

// Pages that link to each other: the Snake, and a Whip and a Jar (item pages, objects oWhip and oJar)
// whose relations point at it. The Whip goes from planned (a red link on the Snake's page) to written.
const itemPage = (slug, title, object, extra = "", body = "") => `---
slug: ${slug}
title: ${title}
type: item
categories: [items/weapons]
objects: [${object}]
lede: The ${title} is an item in Spelunky Classic.
infobox:
  image: { sprite: sWhipLeft, alt: "A ${title}" }
  kind: weapon
${extra}
faq:
  - { q: "One?", a: "One." }
  - { q: "Two?", a: "Two." }
  - { q: "Three?", a: "Three." }
sources: [objects/${object}/create.gml]
verified: { port: v0.1.7, date: 2026-10-27 }
---

${body || `The ${title}.`}
`;
const linked = mkdtempSync(join(tmpdir(), "test-wiki-"));
try {
  const snakeSource = readFileSync(here("content/wiki/snake.mdx"), "utf8");
  writeFileSync(join(linked, "snake.mdx"), snakeSource);
  writeFileSync(join(linked, "whip.mdx"), itemPage("whip", "Whip", "oWhip", "relations:\n  related: [snake]", "It hits a [[snake#behaviour|Snake]], as [[snake#clip-snake-stomp|a clip]] shows."));
  writeFileSync(join(linked, "jar.mdx"), itemPage("jar", "Jar", "oJar", "relations:\n  drops: [snake]"));
  // The Jar is planned in the committed list; here it's written, so it comes off.
  const planned = join(linked, "planned.yaml");
  writeFileSync(planned, readFileSync(here("content/wiki-planned.yaml"), "utf8").replace(/^jar: .*$/m, ""));
  const wiki = await loadWiki(options(linked, planned));
  for (const error of wiki.errors) fail(`linked pages: ${error}`);
  const page = (slug) => wiki.pages.find((p) => p.meta.slug === slug)?.meta;
  const infobox = (slug, relation) => page(slug)?.relations.find((r) => r.relation === relation);
  const seeAlso = (slug, relation) => page(slug)?.seeAlso.find((r) => r.relation === relation);
  const slugs = (r) => r?.links.map((l) => `${l.slug}${l.href ? "" : "?"}`).join();
  // The Snake's link to the Whip is a link now, and the Whip's and the Jar's pages link here.
  if (!seeAlso("snake", "damaged_by")?.links.find((l) => l.slug === "whip")?.href) fail("linked: the snake's damaged_by whip isn't a link");
  if (page("snake")?.linksHere.map((p) => p.slug).join() !== "jar,whip") fail(`linked: snake's What links here ${JSON.stringify(page("snake")?.linksHere)}`);
  if (page("whip")?.linksHere.map((p) => p.slug).join() !== "snake") fail("linked: whip's What links here");
  // Inverses: the Jar's drops are the Snake's Dropped by (in its infobox), the Snake's damaged_by is the
  // Whip's Damages (under See also), and related goes both ways.
  if (slugs(infobox("snake", "dropped_by")) !== "jar") fail("linked: snake dropped_by");
  if (slugs(seeAlso("whip", "damages")) !== "snake") fail("linked: whip damages");
  if (slugs(seeAlso("snake", "related")) !== "whip" || slugs(seeAlso("whip", "related")) !== "snake") fail("linked: related both ways");
  if (!wiki.pages.find((p) => p.meta.slug === "whip")?.code.includes('"/wiki/snake#behaviour"')) fail("linked: whip's [[snake#behaviour]]");
  if (wiki.wanted.some((w) => w.slug === "whip" || w.slug === "jar")) fail("linked: whip and jar aren't wanted any more");
  if (failures === 0) console.log("ok   linked pages: links, inverse relations and What links here");
} finally {
  rmSync(linked, { recursive: true, force: true });
}

// The hubs (src/wiki/build/hubs.ts): each lists its categories' pages and, not links, the pages to come
// (wiki-data's and the planned ones), by title ("The Mines" under M).
{
  const before = failures;
  const hub = (id) => committed.hubs.hubs.find((h) => h.id === id);
  const cards = (id) => hub(id)?.cards.map((c) => `${c.slug}${c.href ? "" : "?"}`).join(" ");
  const want = {
    monsters: "bat? caveman? giant-spider? skeleton? snake spider?",
    "items/weapons": "bow? machete? mattock? pistol? shotgun? web-cannon? whip?",
    shops: "ankh-shop? bomb-shop? clothing-shop? dice-house? general-store? kissing-parlor? rare-shop? weapon-shop?",
  };
  for (const [id, list] of Object.entries(want)) if (cards(id) !== list) fail(`hubs: ${id} lists ${cards(id)}, not ${list}`);
  const places = cards("places") ?? "";
  if (!places.startsWith("black-market? city-of-gold? dark-level? flooded-level? restless-dead? ice-caves? jungle? mines?")) fail(`hubs: places are ${places}`);
  // /wiki/items is its three categories' together.
  const items = hub("items")?.cards.length;
  const sum = ["items/equipment", "items/weapons", "items/carryables"].reduce((n, id) => n + hub(id).cards.length, 0);
  if (items !== sum) fail(`hubs: /wiki/items has ${items} cards, not ${sum}`);
  // The areas its pages are found in: the Snake's found_in, and where generation makes those to come.
  const areas = hub("monsters")?.areas.map((a) => a.slug).join();
  if (areas !== "mines,jungle,temple") fail(`hubs: the monsters' areas are ${areas}`);
  if (hub("monsters")?.cards.find((c) => c.slug === "bat")?.areas.join() !== "mines,jungle") fail("hubs: the Bat isn't in the Mines and the Jungle");
  if (hub("monsters")?.cards.find((c) => c.slug === "snake")?.areas.join() !== "mines") fail("hubs: the Snake isn't found in the Mines");
  const { home } = committed.hubs;
  if (home.featured.map((c) => c.slug).join() !== "snake") fail(`hubs: featured ${home.featured.map((c) => c.slug)}`);
  if (home.written !== committed.pages.length) fail(`hubs: ${home.written} written`);
  if (failures === before) console.log(`ok   hubs: ${home.total} cards, ${home.written} written`);
}

// Pages that break a rule, each with what its error must say. Each is the Snake's page changed.
const source = readFileSync(here("content/wiki/snake.mdx"), "utf8");
const edit = (from, to) => {
  if (!source.includes(from)) throw new Error(`snake.mdx has no ${JSON.stringify(from)}: update test-wiki.mjs`);
  return source.replace(from, to);
};
// A navbox groups its category's pages by area, a page in two being in both, and those in none after them.
{
  const dir = mkdtempSync(join(tmpdir(), "test-wiki-"));
  try {
    // Made-up monsters, whose relations the data can't back, so each cites a line.
    const cited = (slugs) => slugs.map((s) => `{ slug: ${s}, cite: objects/oSnake/step.gml:1 }`).join(", ");
    const page = (slug, title, foundIn) =>
      edit("slug: snake", `slug: ${slug}`)
        .replace("title: Snake", `title: ${title}`)
        .replace("objects: [oSnake]", `objects: [o${title}]`)
        .replace("found_in: [mines, snake-pit, tutorial]", `found_in: [${cited(foundIn.split(", "))}]`)
        .replace("    - whip\n    - machete\n    - spikes\n", `    - ${cited(["whip"])}\n`);
    writeFileSync(join(dir, "snake.mdx"), source);
    writeFileSync(join(dir, "cobra.mdx"), page("cobra", "Cobra", "jungle, mines"));
    writeFileSync(join(dir, "golem.mdx"), page("golem", "Golem", "tutorial"));
    const { pages, errors } = await loadWiki(options(dir));
    for (const error of errors) fail(`navboxes: ${error}`);
    const navbox = pages[0]?.meta.navboxes[0]?.groups.map((g) => `${g.area}: ${g.links.map((l) => l.slug).join(" ")}`).join("; ");
    const want = "The Mines: cobra snake; The Jungle: cobra; null: golem";
    if (navbox !== want) fail(`navboxes: ${navbox}, not ${want}`);
    else console.log(`ok   navboxes: ${navbox}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

// A page takes the place of its card: the Bat's, written, is a link, and a monster in its areas.
{
  const dir = mkdtempSync(join(tmpdir(), "test-wiki-"));
  try {
    writeFileSync(join(dir, "snake.mdx"), source);
    const bat = edit("slug: snake", "slug: bat")
      .replace("title: Snake", "title: Bat")
      .replace("objects: [oSnake]", "objects: [oBat]")
      .replace("found_in: [mines, snake-pit, tutorial]", "found_in: [mines, jungle]")
      // A flying Bat isn't hurt by spikes.
      .replace("    - spikes\n", "");
    writeFileSync(join(dir, "bat.mdx"), bat);
    const { hubs, errors } = await loadWiki(options(dir));
    for (const error of errors) fail(`hub cards: ${error}`);
    const monsters = hubs.hubs.find((h) => h.id === "monsters");
    const cards = monsters.cards.map((c) => `${c.slug}${c.href ? "" : "?"}`).join(" ");
    const want = "bat caveman? giant-spider? skeleton? snake spider?";
    if (cards !== want) fail(`hub cards: monsters ${cards}, not ${want}`);
    else if (monsters.cards[0].areas.join() !== "mines,jungle") fail(`hub cards: the Bat's areas are ${monsters.cards[0].areas}`);
    else console.log(`ok   hub cards: ${cards}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

// The search index the build makes of the committed pages (plugin.ts's virtual:wiki/search-index), and
// what searching it finds (src/wiki/search.ts).
{
  const warn = console.warn;
  console.warn = () => {};
  const code = await wiki(options(here("content/wiki"))).load.call({}, "\0virtual:wiki/search-index").finally(() => (console.warn = warn));
  const { index } = await import(`data:text/javascript,${encodeURIComponent(code)}`);
  const before = failures;
  const found = (query) => searchWiki(index, query).map((hit) => hit.doc.slug).join();
  // The title, as typed so far, a plural, any case and accents; a word only the body has, and one only the
  // FAQ has. Every word must match.
  // A table's cells ("Machete" and "Stomp" in the Defeating table), and a query's single letters ignored
  // ("Olmec's" is "olmec" and "s").
  for (const query of ["snake", "sna", "Snakes", "SNÄKE", "invincible", "poisonous", "snake bite", "mirrored", "machete", "stomp", "olmec's"]) {
    if (found(query) !== "snake") fail(`search: "${query}" finds ${JSON.stringify(found(query))}, not snake`);
  }
  for (const query of ["", "  ", "xyzzy", "snake xyzzy", "the"]) {
    if (found(query) !== "") fail(`search: "${query}" finds ${found(query)}`);
  }
  // A hit names the sections whose headings match, to link them.
  const sections = matchingSections(searchWiki(index, "snake trivia")[0]?.doc, "snake trivia").map(([id]) => id).join();
  if (sections !== "trivia") fail(`search: "snake trivia" names the sections ${sections}`);
  const hit = searchWiki(index, "snake")[0];
  if (hit?.doc.category !== "Monsters" || !hit.doc.lede.startsWith("The Snake is a monster")) {
    fail(`search: the Snake's hit is ${JSON.stringify(hit?.doc)}`);
  }
  // A title outranks a lede, a lede the text; a word itself outranks a longer one it starts.
  const ranked = buildSearchIndex([
    { slug: "in-text", title: "Other", aliases: [], lede: "", categories: [], sections: [], text: "rock rock rock rock rock" },
    { slug: "in-lede", title: "Thing", aliases: [], lede: "A rock.", categories: [], sections: [], text: "" },
    { slug: "rocket", title: "Rocket", aliases: [], lede: "", categories: [], sections: [], text: "" },
    { slug: "rock", title: "Rock", aliases: [], lede: "", categories: [], sections: [], text: "" },
  ]);
  const order = searchWiki(ranked, "rock").map((hit) => hit.doc.slug).join();
  if (order !== "rock,rocket,in-lede,in-text") fail(`search: "rock" ranks ${order}`);
  if (failures === before) console.log(`ok   search: ${index.terms.length} words in the index of ${index.docs.length} page(s)`);
}

const cases = [
  ["a category that isn't one", { "snake.mdx": edit("categories: [monsters]", "categories: [monster]") }, "frontmatter.categories[0]"],
  ["no categories", { "snake.mdx": edit("categories: [monsters]", "categories: []") }, "frontmatter.categories"],
  ["no lede", { "snake.mdx": edit("lede: >-", "summary: >-") }, "frontmatter.lede"],
  ["a type that isn't one", { "snake.mdx": edit("type: monster", "type: beast") }, "frontmatter.type"],
  ["a slug that isn't the file's name", { "snake.mdx": edit("slug: snake", "slug: snakes") }, "a page's file is named <slug>.mdx"],
  ["a slug that isn't kebab case", { "Snake.mdx": edit("slug: snake", "slug: Snake") }, "lowercase kebab case"],
  ["a hub's slug", { "monsters.mdx": edit("slug: snake", "slug: monsters") }, "slug monsters is reserved"],
  [
    "the same slug twice",
    { "snake.mdx": source, "cobra.mdx": edit("slug: snake", "slug: cobra\naliases: [snake]").replace("objects: [oSnake]", "objects: [oCobra]") },
    "snake is already snake.mdx's slug",
  ],
  ["an object on two pages", { "snake.mdx": source, "cobra.mdx": edit("slug: snake", "slug: cobra") }, "an object belongs to one page"],
  ["a stat that isn't in stats.json", { "snake.mdx": edit("health: { stat: snake.hp }", "health: { stat: snake.hitpoints }") }, "snake has no stat hitpoints"],
  ["an infobox field the type hasn't got", { "snake.mdx": edit("health: { stat: snake.hp }", "price: { stat: snake.hp }") }, "frontmatter.infobox"],
  ["a clip without a spec", { "snake.mdx": edit("clips: [snake/bite,", "clips: [snake/hiss, snake/bite,") }, "no clip snake/hiss"],
  ["an unknown component", { "snake.mdx": `${source}\n<Hiss />\n` }, "no component <Hiss>"],
  ["a generated section written by hand", { "snake.mdx": `${source}\n## FAQ\n` }, 'the layout generates "FAQ"'],
  ["no frontmatter", { "snake.mdx": "# Snake\n" }, "no frontmatter"],
  ["code between comments", { "snake.mdx": `${source}\n{/* a */ globalThis.fetch("x") /* b */}\n` }, "a page has no {expressions}"],
  ["an HTML attribute that's code", { "snake.mdx": `${source}\n<span title={Date.now()}>x</span>\n` }, "<span>'s attributes are strings"],
  ["an event handler", { "snake.mdx": `${source}\n<img src="x" alt="" onError="alert(1)" />\n` }, "<img> has an event handler"],
  ["a script", { "snake.mdx": `${source}\n<script>alert(1)</script>\n` }, "a page has no <script>"],
  ["two headings with one anchor", { "snake.mdx": `${source}\n## Behaviour\n` }, "two headings have the anchor #behaviour"],
  ["a heading with the contents' anchor", { "snake.mdx": `${source}\n## TOC\n` }, "#toc is one of the layout's anchors"],
  ["a heading for the contents", { "snake.mdx": `${source}\n## Contents\n` }, 'the layout generates "Contents"'],
  ["a heading with no anchor", { "snake.mdx": `${source}\n## ???\n` }, "has no letters or digits"],
  ["an element with the layout's anchor", { "snake.mdx": `${source}\n<div id="toc">x</div>\n` }, '<div id="toc">: #toc is one'],
  ["a date that doesn't exist", { "snake.mdx": edit("date: 2026-10-27", "date: 2026-13-01") }, "a date that exists"],
  ["a relation listing a page twice", { "snake.mdx": edit("- whip\n", "- whip\n    - whip\n") }, "damaged_by lists whip twice"],
  ["a heading with a navbox's anchor", { "snake.mdx": `${source}\n## Navbox monsters\n` }, "#navbox-monsters is one of the layout's anchors"],
  ["a stat that's text", { "snake.mdx": `${source}\n<Stat id="oSnake.type" />\n` }, 'is text ("Snake")'],
  ["a value the create event changes later", { "snake.mdx": `${source}\n<Stat id="oFrog.swimming" />\n` }, "changes swimming again"],
];
// A planned page list, changed.
const planned = readFileSync(here("content/wiki-planned.yaml"), "utf8");
const plannedEdit = (from, to) => {
  if (!planned.includes(from)) throw new Error(`wiki-planned.yaml has no ${JSON.stringify(from)}: update test-wiki.mjs`);
  return planned.replace(from, to);
};
cases.push(
  ["a link to no page", { "snake.mdx": `${source}\nSee [[jars]].\n` }, "[[jars]]: no page and no planned page"],
  ["a link to a page with a typo", { "snake.mdx": edit("[[mines|the Mines]] (levels 1–4)", "[[mine|the Mines]] (levels 1–4)") }, "[[mine]]: no page and no planned page"],
  ["a link to the page itself", { "snake.mdx": `${source}\nSee [[snake|Snakes]].\n` }, "[[snake]] is this page"],
  ["a link to a section a page hasn't got", { "snake.mdx": source, "whip.mdx": itemPage("whip", "Whip", "oWhip", "", "It hits a [[snake#fangs|Snake]].") }, "snake has no #fangs"],
  ["a relation to no page", { "snake.mdx": edit("found_in: [mines, snake-pit, tutorial]", "found_in: [mines, snake-pit, tutorial, caves]") }, "relations.found_in: caves: no page and no planned page"],
  ["a relation the data doesn't back", { "snake.mdx": edit("found_in: [mines, snake-pit, tutorial]", "found_in: [mines, snake-pit, tutorial, jungle]") }, "relations.found_in: jungle: relations.json doesn't say so for oSnake"],
  ["a relation to the wrong type of page", { "snake.mdx": edit("found_in: [mines, snake-pit, tutorial]", "found_in: [mines, snake-pit, tutorial, weapon-shop]") }, "found_in points at place pages, and weapon-shop is a shop page"],
  ["a relation listed twice", { "snake.mdx": edit("found_in: [mines, snake-pit, tutorial]", "found_in: [mines, snake-pit, tutorial, mines]") }, "found_in lists mines twice"],
  [
    "a plain relation on a page without objects",
    {
      "general-store.mdx": itemPage("general-store", "General Store", "oX", "relations:\n  found_in: [mines]")
        .replace("type: item", "type: shop")
        .replace("categories: [items/weapons]", "categories: [shops]")
        .replace("objects: [oX]\n", "")
        .replace("  kind: weapon\n", ""),
    },
    "general-store.mdx: relations.found_in: mines: a page without objects cites each relation",
  ],
  ["a planned page that's wiki-data's", { "snake.mdx": source, "planned.yaml": `${planned}whip: { title: Whip }\n` }, "whip is wiki-data's already (Whip)"],
  ["a planned page with a page", { "snake.mdx": source, "jar.mdx": itemPage("jar", "Jar", "oJar") }, "jar has a page now: remove it"],
  ["a planned page without a title", { "snake.mdx": source, "planned.yaml": plannedEdit("jar: { title: Jar, type: item }", "jar: { objects: [oJar] }") }, "planned.yaml: jar.title"],
  ["an alias that's wiki-data's slug", { "snake.mdx": edit("slug: snake", "slug: snake\naliases: [spider]") }, "alias spider is wiki-data's slug for Spider"],
  ["a relation to the page's own alias", { "snake.mdx": edit("slug: snake", "slug: snake\naliases: [serpent]").replace("  found_in: [mines,", "  related: [serpent]\n  found_in: [mines,") }, "related: serpent: a page doesn't relate to itself"],
  ["a link to a section the page doesn't show", { "snake.mdx": source, "whip.mdx": itemPage("whip", "Whip", "oWhip", "", "It hits a [[snake#gallery|Snake]].") }, "snake has no #gallery"],
  ["a relation to a planned page of the wrong type", { "snake.mdx": edit("found_in: [mines, snake-pit, tutorial]", "found_in: [mines, snake-pit, tutorial, { slug: throwing, cite: objects/oItem/step.gml:256-344 }]") }, "throwing is a mechanic page"],
  [
    "a room on two pages",
    { "snake.mdx": source, "planned.yaml": plannedEdit("tutorial: { title: Tutorial, type: place, rooms: [rTutorial] }", "tutorial: { title: Tutorial, rooms: [rTutorial] }\nintro: { title: Intro, rooms: [rTutorial] }") },
    "rTutorial is tutorial's and intro's",
  ],
  ["a planned page nothing links to", { "snake.mdx": source, "planned.yaml": `${planned}cobra: { title: Cobra }\n` }, "no page links to cobra: remove it"],
  ["a planned page that's a page's alias", { "snake.mdx": edit("slug: snake", "slug: snake\naliases: [serpent]"), "planned.yaml": `${planned}serpent: { title: Serpent }\n` }, "serpent is snake.mdx's alias"],
  ["a planned page with another's object", { "snake.mdx": source, "planned.yaml": plannedEdit("jar: { title: Jar, type: item }", "jar: { title: Jar, objects: [oSnake] }") }, "oSnake is snake's and jar's"],
);
for (const [what, files, want] of cases) {
  const dir = mkdtempSync(join(tmpdir(), "test-wiki-"));
  try {
    for (const [name, text] of Object.entries(files)) writeFileSync(join(dir, name), text);
    // A case's own planned pages, or the committed ones.
    const plannedFile = files["planned.yaml"] === undefined ? undefined : join(dir, "planned.yaml");
    const { errors } = await loadWiki(options(dir, plannedFile));
    if (!errors.some((error) => error.includes(want))) {
      fail(`${what}: no error with ${JSON.stringify(want)}; got ${JSON.stringify(errors)}`);
      continue;
    }
    // And the build fails on it: the plugin's start throws. (Quietly: it warns that there are no media.)
    const { warn, info } = console;
    console.warn = console.info = () => {};
    const thrown = await wiki(options(dir, plannedFile))
      .buildStart.call({})
      .then(() => null, (error) => error)
      .finally(() => Object.assign(console, { warn, info }));
    if (!thrown?.message.includes(want)) fail(`${what}: the build doesn't fail with it`);
    else console.log(`ok   ${what}: ${errors.find((error) => error.includes(want))}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

if (failures > 0) {
  console.error(`site/test-wiki.mjs: ${failures} failed`);
  process.exit(1);
}
console.log(`site/test-wiki.mjs: all ${cases.length + 5} passed`);
