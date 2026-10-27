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

const here = (path) => new URL(path, import.meta.url).pathname;
const options = (contentDir) => ({
  contentDir,
  statsFile: here("content/wiki-data/stats.json"),
  clipsDir: here("../wiki/clips"),
  relationsSpec: here("../wiki/data/relations.toml"),
  mediaPin: here("wiki-media.txt"),
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
}
if (failures === 0) console.log(`ok   content/wiki: ${committed.pages.length} page(s) compile`);

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
    const page = (slug, title, foundIn) =>
      edit("slug: snake", `slug: ${slug}`)
        .replace("title: Snake", `title: ${title}`)
        .replace("objects: [oSnake]", `objects: [o${title}]`)
        .replace("found_in: [mines, snake-pit, tutorial]", `found_in: [${foundIn}]`);
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
  ["a heading with a navbox's anchor", { "snake.mdx": `${source}\n## Navbox monsters\n` }, "#navbox-monsters is one of the layout's anchors"],
  ["a stat that's text", { "snake.mdx": `${source}\n<Stat id="oSnake.type" />\n` }, 'is text ("Snake")'],
  ["a value the create event changes later", { "snake.mdx": `${source}\n<Stat id="oFrog.swimming" />\n` }, "changes swimming again"],
];
for (const [what, files, want] of cases) {
  const dir = mkdtempSync(join(tmpdir(), "test-wiki-"));
  try {
    for (const [name, text] of Object.entries(files)) writeFileSync(join(dir, name), text);
    const { errors } = await loadWiki(options(dir));
    if (!errors.some((error) => error.includes(want))) {
      fail(`${what}: no error with ${JSON.stringify(want)}; got ${JSON.stringify(errors)}`);
      continue;
    }
    // And the build fails on it: the plugin's start throws. (Quietly: it warns that there are no media.)
    const warn = console.warn;
    console.warn = () => {};
    const thrown = await wiki(options(dir))
      .buildStart.call({})
      .then(() => null, (error) => error)
      .finally(() => (console.warn = warn));
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
console.log(`site/test-wiki.mjs: all ${cases.length + 2} passed`);
