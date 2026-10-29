// The wiki pipeline's checks (docs/wiki-style.md, "The pipeline"), which site/test.sh runs: the committed
// pages compile with no errors, and a page that breaks a rule fails the build with an error naming it. Node
// runs the build's TypeScript as is (type stripping).
//
//   node test-wiki.mjs
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parse as parseYaml } from "yaml";
import { clipLive, loadWiki } from "./src/wiki/build/compile.ts";
import { hoverCards } from "./src/wiki/build/cards.ts";
import { cardScale, makeHubs } from "./src/wiki/build/hubs.ts";
import { markdownFiles } from "./src/wiki/build/markdown.ts";
import { readSnapshot, resolveStat } from "./src/wiki/build/stats.ts";
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
  quirksFile: here("../docs/quirks.md"),
  media: null,
  sourceDir: null,
  liveClips: null,
});

// A row of a Markdown table, whatever its cells' padding.
const tableRow = (...cells) => new RegExp(`^\\| ${cells.map((c) => c.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join(" +\\| ")} +\\|$`, "m");

let failures = 0;
function fail(message) {
  console.error(`FAIL ${message}`);
  failures++;
}

// The committed pages.
const committed = await loadWiki(options(here("content/wiki")));
for (const error of committed.errors) fail(`content/wiki: ${error}`);
if (committed.pages.length === 0) fail("content/wiki: no pages");
// A table stat's row: a shop's price on level 5, 10% of the base price more for each level past the second.
{
  const row = resolveStat(readSnapshot(here("content/wiki-data/stats.json")), "spectacles.price-by-level", { at: 5 });
  if (row.text !== "$10,400" || row.value !== "10400" || row.cite.join() !== "scripts/scrShopItemsGen.gml:186") {
    fail(`spectacles.price-by-level at 5 is ${JSON.stringify(row)}`);
  }
}
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
  // Its relations: the places it's found in, named by the relations' spec until they have pages, and
  // Olmec's page's drops (his yellow balls), uncited, so the notes above keep their numbers.
  const found = meta.relations.map((r) => `${r.label}: ${r.links.map((l) => l.text).join(", ")}`).join("; ");
  if (found !== "Found in: The Mines, Snake pit, Tutorial; Dropped by: Jar, Olmec") fail(`snake: the infobox's relations are ${found}`);
  // The table of contents' sections, with their stable anchors.
  const sections = meta.sections.map((s) => s.id).join();
  if (sections !== "behaviour,damage,defeating,locations,tips,trivia") fail(`snake: sections ${sections}`);
  // The monsters' navbox, its pages by area, the first the Mines, where the Snake is (and nowhere else).
  const navbox = meta.navboxes.find((n) => n.category === "monsters");
  const withSnake = navbox?.groups.filter((g) => g.links.some((l) => l.slug === "snake")).map((g) => g.area).join();
  if (meta.navboxes.length !== 1 || navbox.groups[0]?.area !== "The Mines" || withSnake !== "The Mines") {
    fail(`snake: navboxes ${JSON.stringify(meta.navboxes.map((n) => [n.category, n.groups.map((g) => g.area)]))}`);
  }
  if (meta.gallery.length !== 0) fail(`snake: the body places every clip, but the gallery has ${meta.gallery}`);
  // Its FAQ's FAQPage, the answers as plain text; and without the media, no VideoObject or ImageObject:
  // there's nothing to point them at.
  if (meta.jsonLd.map((node) => node["@type"]).join() !== "FAQPage") fail(`snake: JSON-LD ${JSON.stringify(meta.jsonLd)}`);
  const faq = meta.jsonLd[0]?.mainEntity ?? [];
  if (faq.map((q) => q.name).join("|") !== meta.questions.join("|")) fail("snake: the FAQPage's questions aren't the FAQ's");
  const answer = "1 heart per bite. Touching a Snake costs the player 1 heart, knocks him back and makes him invincible for 30 steps (1 second), so it can't bite again at once.";
  if (faq[0]?.acceptedAnswer.text !== answer) fail(`snake: the FAQPage's first answer is ${JSON.stringify(faq[0]?.acceptedAnswer)}`);
  // Its relations: found_in in the infobox, damaged_by under See also, all red links for now, a cited
  // entry with its note number (the body's, where it cites the same lines).
  const foundIn = meta.relations.find((r) => r.relation === "found_in");
  if (foundIn?.links.map((l) => l.slug).join() !== "mines,snake-pit,tutorial") fail("snake: found_in");
  // The Mines' page and the snake pit's are written, the Tutorial's isn't: a red link.
  if (foundIn?.links.map((l) => l.href !== undefined).join() !== "true,true,false") fail("snake: found_in's written pages are links, the rest red links");
  const damagedBy = meta.seeAlso.find((r) => r.relation === "damaged_by");
  const stomping = damagedBy?.links.find((l) => l.slug === "stomping");
  const stompCite = meta.references.find((r) => r.target === "objects/oEnemy/collision_oCharacter.gml:6-10");
  if (!stompCite || stomping?.notes.join() !== String(stompCite.n)) fail("snake: damaged_by's citations");
  // What links here: the Bat's page does (Olmec's yellow balls), with the others in title order.
  const linksHere = meta.linksHere.map((p) => p.title);
  if (!linksHere.includes("Bat") || linksHere.join() !== [...linksHere].sort().join()) fail(`snake: What links here is ${linksHere}`);
}
// Kali's altar, a place with objects: its part_of and drops are plain, backed by what relations.json says of
// its objects, in its infobox; and its gifts' pages show it as what drops them.
{
  const page = (slug) => committed.pages.find((p) => p.meta.slug === slug)?.meta;
  const altar = page("kalis-altar");
  const lists = altar?.relations.map((r) => `${r.relation}: ${r.links.map((l) => l.slug).join(",")}`).join("; ");
  const want = "part_of: mines,jungle,ice-caves,temple; drops: cape,climbing-gloves,spectacles,pitchers-mitt,spring-shoes,spike-shoes,paste,compass,jetpack,bomb-box,kapala,ball-and-chain,ghost,spider";
  if (lists !== want) fail(`kalis-altar: the infobox's relations are ${lists}`);
  const droppedBy = page("kapala")?.relations.find((r) => r.relation === "dropped_by");
  if (!droppedBy?.links.some((l) => l.slug === "kalis-altar")) fail("kapala: Kali's altar isn't what drops it");
  const contains = page("mines")?.relations.find((r) => r.relation === "contains");
  if (!contains?.links.some((l) => l.slug === "kalis-altar")) fail("mines: doesn't contain Kali's altar");
  // The places' navbox groups them by their part_of, as the hub filters them: an area's own page is in the rest,
  // all areas, so titled Areas.
  const navbox = altar?.navboxes.find((n) => n.category === "places");
  const groups = navbox?.groups.map((g) => g.area).join();
  if (groups !== "The Mines,The Jungle,The Ice Caves,The Temple,Areas") fail(`kalis-altar: the places' navbox groups are ${groups}`);
  const groupsOf = (slug) => navbox?.groups.filter((g) => g.links.some((l) => l.slug === slug)).map((g) => g.area).join();
  for (const [slug, want] of [["kalis-altar", "The Mines,The Jungle,The Ice Caves,The Temple"], ["snake-pit", "The Mines"], ["city-of-gold", "The Temple"], ["mines", "Areas"], ["olmecs-lair", "Areas"]]) {
    if (groupsOf(slug) !== want) fail(`kalis-altar: the places' navbox has ${slug} in ${groupsOf(slug)}, not ${want}`);
  }
}
if (committed.warnings.some((w) => !w.startsWith("no wiki media"))) fail(`content/wiki: warnings ${JSON.stringify(committed.warnings)}`);
// Without the media's gml/, no citation links to the source viewer.
if (committed.pages.some((p) => p.code.includes("/wiki/source/"))) fail("content/wiki: a citation links to the source viewer, without its gml/");

// The Markdown (src/wiki/build/markdown.ts): each page's twin, llms.txt and llms-full.txt, each starting with
// the license's notice (docs/wiki-license.md, "Per-page notice"), its link absolute.
{
  const before = failures;
  const notice =
    "Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC, and is used under the [Spelunky User License v1.1b](https://spelunky.ai/LICENSE.txt). spelunky.ai is an unofficial fan project, not affiliated with or endorsed by Mossmouth. The game and its assets may not be sold.\n\n";
  const audio = "Music by George Buzinkai and Jonathan Perry; sound effects made in SFXR. From Spelunky Classic 1.1, © 2008, 2009 Derek Yu and Mossmouth, LLC.";
  const files = markdownFiles(committed.pages);
  const want = [...committed.pages.map((page) => `wiki/${page.meta.slug}.md`), "llms.txt", "llms-full.txt"].sort().join();
  if ([...files.keys()].sort().join() !== want) fail(`markdown: the files are ${[...files.keys()]}`);
  for (const [name, text] of files) {
    if (!text.startsWith(notice)) fail(`markdown: ${name} doesn't start with the notice`);
    // Nothing of the MDX is left: components, [[links]], comments.
    if (/<[A-Z]|\[\[|\{\/\*/.test(text)) fail(`markdown: ${name} has MDX in it`);
  }
  const md = files.get("wiki/snake.md") ?? "";
  // The title, where it's from, the lede, the infobox's table with its notes, the body's sections, a red
  // link's text, a section's link made absolute, a note after its word, a GML excerpt with its caption, the
  // FAQ, and References' notes.
  for (const text of [
    "\n\n# Snake\n\nFrom the spelunky.ai wiki: <https://spelunky.ai/wiki/snake>. Verified against the spelunky.ai port [v0.1.7](https://github.com/851-labs/spelunky/releases/tag/v0.1.7) on 27 October 2026.\n\nThe Snake is a monster in Spelunky Classic, found mostly in the Mines, that slithers back and forth along the ground and bites for 1 heart on contact.\n\n| Snake ",
    "\n## Behaviour\n",
    "almost any block in [the Mines](https://spelunky.ai/wiki/mines), and a [Snake pit](https://spelunky.ai/wiki/snake-pit)",
    "**The Tutorial**, which has one",
    "as the [level editor](https://spelunky.ai/wiki/level-editor)'s tile `n`",
    "([Trivia](https://spelunky.ai/wiki/snake#trivia))",
    "walks off one[^5]. Where",
    "**Clip: Snake bite.** A snake slithers up to the player and bites him, taking a heart.",
    "  ```gml objects/oSnake/step.gml:73-76\n  else if (status == ATTACK)\n",
    "  `objects/oSnake/step.gml:73-76`, from `spelunky.gmk`, Spelunky Classic 1.1's source, © 2008, 2009 Derek Yu and Mossmouth, LLC ([Spelunky User License v1.1b](https://spelunky.ai/LICENSE.txt)).",
    "\n## FAQ\n\n### How much damage does a Snake do in Spelunky Classic?\n\n1 heart per bite. Touching a Snake",
    "\n## References\n\n[^1]: `objects/oSnake/create.gml:10`\n",
    "\n### Source code\n\n- `objects/oSnake/create.gml`\n",
  ]) {
    if (!md.includes(text)) fail(`markdown: snake.md has no ${JSON.stringify(text)}`);
  }
  for (const row of [["Health", "1 hit point[^1]"], ["Object", "`oSnake`"], ["Found in", "[The Mines](https://spelunky.ai/wiki/mines), [Snake pit](https://spelunky.ai/wiki/snake-pit), Tutorial"]]) {
    if (!tableRow(...row).test(md)) fail(`markdown: snake.md's infobox has no row ${row.join(" | ")}`);
  }
  // Without the media, no clip has sound, so no audio credit; with it, after the notice. llms.txt has no
  // clips, so never.
  if (md.includes(audio)) fail("markdown: snake.md has the audio credit, without the media");
  const snakePage = committed.pages.find((page) => page.meta.slug === "snake");
  const withAudio = markdownFiles([{ ...snakePage, meta: { ...snakePage.meta, audio: true } }]);
  if (!withAudio.get("wiki/snake.md")?.startsWith(`${notice}${audio}\n\n# Snake\n`)) fail("markdown: snake.md with sound hasn't the audio credit");
  if (!withAudio.get("llms-full.txt")?.startsWith(`${notice}${audio}\n\n# spelunky.ai`)) fail("markdown: llms-full.txt with sound hasn't the audio credit");
  if (withAudio.get("llms.txt")?.includes(audio)) fail("markdown: llms.txt has the audio credit");
  // llms.txt (https://llmstxt.org): the site's title and summary, then each category's pages' twins with
  // their ledes.
  const llms = files.get("llms.txt") ?? "";
  if (!llms.startsWith(`${notice}# spelunky.ai: the Spelunky Classic fan wiki\n\n> An unofficial fan wiki for Spelunky Classic`)) fail("markdown: llms.txt's title");
  const monsters = llms.slice(llms.indexOf("\n## Monsters\n\n"));
  if (!llms.includes("\n## Monsters\n\n- [") || !monsters.split("\n## ")[1]?.includes("\n- [Snake](https://spelunky.ai/wiki/snake.md): The Snake is a monster in Spelunky Classic, found mostly in the Mines, that slithers back and forth along the ground and bites for 1 heart on contact.\n")) fail("markdown: llms.txt's Snake");
  if (!llms.includes("\n## Optional\n")) fail("markdown: llms.txt has no Optional section");
  // llms-full.txt: every page, its notes named for it, as pages put together would otherwise share them.
  const full = files.get("llms-full.txt") ?? "";
  if (!full.includes("\n---\n\n# Snake\n") || !tableRow("Health", "1 hit point[^snake-1]").test(full) || !full.includes("\n[^snake-1]: `objects/oSnake/create.gml:10`\n")) fail("markdown: llms-full.txt's Snake");
  if (/\[\^\d/.test(full)) fail("markdown: llms-full.txt has a note not named for its page");
  if (failures === before) console.log(`ok   markdown: ${files.size} files`);
}
// Its red links, listed: each planned page it links to.
const wanted = committed.wanted.map((w) => w.slug);
for (const slug of ["tutorial", "bomb-box", "rope-pile"]) if (!wanted.includes(slug)) fail(`wanted: no ${slug}`);
if (failures === 0) console.log(`ok   content/wiki: ${committed.pages.length} page(s) compile, linking ${wanted.length} planned page(s)`);

// The planned pages as the Snake's page alone needs them: the fixtures below are built around it, and the
// committed list's pages only other pages link (the Skeleton's Skull) would fail them as linked by none. The
// written pages the Snake links or relates to that wiki-data hasn't got (the Level editor) aren't in the
// fixtures either: each is planned there instead, from its frontmatter, as it was before it was written.
const snakeLinks = new Set(committed.wanted.filter((w) => w.from.includes("snake")).map((w) => w.slug));
const readJson = (path) => JSON.parse(readFileSync(here(path), "utf8"));
const relationsJson = readJson("content/wiki-data/relations.json");
const dataSlugs = new Set([
  ...Object.keys(readJson("content/wiki-data/stats.json").entities),
  ...Object.keys(relationsJson.places),
  ...Object.keys(relationsJson.shops),
]);
const frontmatter = (slug) => parseYaml(readFileSync(here(`content/wiki/${slug}.mdx`), "utf8").split("\n---\n")[0].slice(4));
const snakePage = readFileSync(here("content/wiki/snake.mdx"), "utf8");
const snakeTargets = new Set([
  ...[...snakePage.matchAll(/\[\[([a-z0-9-]+)/g)].map((m) => m[1]),
  ...Object.values(frontmatter("snake").relations ?? {})
    .flat()
    .map((r) => (typeof r === "string" ? r : r.slug)),
]);
const writtenStandIns = [...snakeTargets]
  .filter((slug) => slug !== "snake" && !dataSlugs.has(slug) && committed.pages.some((p) => p.meta.slug === slug))
  .map((slug) => {
    const { title, type, categories, objects, rooms } = frontmatter(slug);
    const entry = { title, type };
    // A planned page's category is only for a hub other than its type's.
    if (categories[0] !== `${type}s`) entry.category = categories[0];
    if (objects?.length) entry.objects = objects;
    if (rooms?.length) entry.rooms = rooms;
    return `${slug}: ${JSON.stringify(entry)}\n`;
  });
const snakePlanned =
  readFileSync(here("content/wiki-planned.yaml"), "utf8")
    .split("\n")
    .filter((line) => !/^[a-z0-9-]+:/.test(line) || snakeLinks.has(line.slice(0, line.indexOf(":"))))
    .join("\n") + writtenStandIns.join("");
const snakePlannedDir = mkdtempSync(join(tmpdir(), "test-wiki-"));
const snakePlannedFile = join(snakePlannedDir, "planned.yaml");
writeFileSync(snakePlannedFile, snakePlanned);
process.on("exit", () => rmSync(snakePlannedDir, { recursive: true, force: true }));

// The monsters' hub's scope, as the hubs sort it (by title, "The Mines" under M): wiki-data's monsters, a
// planned list's, and the written character pages it takes in (the Shopkeeper), so a content PR that adds a
// monster's specs or plans one needn't change this file. Only characters: a page of another type in the
// wrong category would otherwise be in both lists, and pass.
const statsJson = JSON.parse(readFileSync(here("content/wiki-data/stats.json"), "utf8"));
const monsterScope = (planned, written = []) => {
  const titles = Object.entries(statsJson.entities)
    .filter(([, entity]) => entity.kind === "monster")
    .map(([slug, entity]) => [slug, entity.name]);
  for (const [slug, page] of Object.entries(parseYaml(planned) ?? {})) {
    if ((page.category ?? (page.type === "monster" ? "monsters" : null)) === "monsters") titles.push([slug, page.title]);
  }
  for (const { slug, title, type, categories } of written) {
    if (type === "character" && categories.includes("monsters") && !titles.some(([s]) => s === slug)) titles.push([slug, title]);
  }
  const key = (title) => title.replace(/^The /, "");
  return titles.sort((a, b) => key(a[1]).localeCompare(key(b[1]), "en")).map(([slug]) => slug);
};

// Pages that link to each other: the Snake, and a Whip and a Jar (item pages, objects oWhip and oJar)
// whose relations point at it. The Whip goes from planned (a red link on the Snake's page) to written.
// A mechanic page: no infobox, no objects, its hub card's sprite in `card`.
const mechanicPage = `---
slug: grip
title: Grip
type: mechanic
categories: [mechanics]
card: { sprite: sFallLeft }
lede: Grip in Spelunky Classic is how the player holds on.
faq:
  - q: What is grip?
    a: How the player holds on.
  - q: Does it slip?
    a: No.
  - q: Can it be lost?
    a: No.
sources:
  - objects/oSnake/step.gml
verified: { port: v0.1.7, date: 2026-10-27 }
---

Grip keeps the player on a ledge.
`;
// A quirk page, named by a docs/quirks.md row (src/wiki/build/quirks.ts).
const quirkPage = `---
slug: double-whip
title: Double whip
type: quirk
categories: [quirks]
card: { sprite: sFallLeft }
lede: Double whip is a quirk of Spelunky Classic in which a whip hits twice.
quirk:
  status: GML + Oracle
  where: objects/oJar/collision_oWhip.gml:1-21
  pinned_by: fidelity/replays/item-jar-whip-rolls-twice.txt
  entry: "1"
  rows:
    - A whipped Jar rolls its contents twice.
faq:
  - q: Does it?
    a: Yes.
  - q: Twice?
    a: Yes.
  - q: Always?
    a: Yes.
sources:
  - objects/oJar/collision_oWhip.gml
verified: { port: v0.1.7, date: 2026-10-27 }
---

It hits twice.
`;
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
  // A stand-in for the committed Jar's page, a weapon here so that llms.txt lists it beside the Whip.
  const planned = join(linked, "planned.yaml");
  writeFileSync(planned, snakePlanned);
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
  // Their Markdown links the pages, not their twins (src/wiki/build/markdown.ts), a section absolutely; and
  // llms.txt lists each page under its category, llms-full.txt has each once.
  const md = markdownFiles(wiki.pages);
  if (!md.get("wiki/whip.md")?.includes("It hits a [Snake](https://spelunky.ai/wiki/snake#behaviour)")) fail("linked: whip.md's [[snake#behaviour]]");
  if (!md.get("wiki/snake.md")?.includes("- **Damaged by:** [Whip](https://spelunky.ai/wiki/whip), Machete,")) fail("linked: snake.md's See also");
  if (!tableRow("Dropped by", "[Jar](https://spelunky.ai/wiki/jar)").test(md.get("wiki/snake.md") ?? "")) fail("linked: snake.md's infobox relations");
  const llms = md.get("llms.txt") ?? "";
  if (!llms.includes("## Monsters\n\n- [Snake](https://spelunky.ai/wiki/snake.md): The Snake is a monster")) fail("linked: llms.txt's Monsters");
  if (!llms.includes("## Weapons\n\n- [Jar](https://spelunky.ai/wiki/jar.md): The Jar is an item in Spelunky Classic.\n- [Whip](https://spelunky.ai/wiki/whip.md):")) fail("linked: llms.txt's Weapons");
  const titles = [...(md.get("llms-full.txt") ?? "").matchAll(/^# (.*)$/gm)].map((m) => m[1]).join();
  if (titles !== "spelunky.ai: the Spelunky Classic fan wiki,Snake,Jar,Whip") fail(`linked: llms-full.txt's pages are ${titles}`);
  if (failures === 0) console.log("ok   linked pages: links, inverse relations and What links here");
} finally {
  rmSync(linked, { recursive: true, force: true });
}

// Plain text (compile.ts's plainText: the FAQPage's answers, the search index's text) keeps the spaces the
// page has around punctuation: none added after inline code or a link, none dropped before code that is
// punctuation. Only the space before what says nothing there, a <Cite>, goes.
{
  const dir = mkdtempSync(join(tmpdir(), "test-wiki-"));
  try {
    const answers = [
      "Names starting with `!` aren't, nor `?`, `,`, `.` or `)` ones.",
      "A [[snake|Snake]]! A `whip`? A [[snake|Snake]], a `rock`. (A `rope`) and (a [[snake|Snake]]); a `bomb`: no.",
      'It holds on <Cite gml="objects/oSnake/step.gml:1" />. Not always (it slips <Cite gml="objects/oSnake/step.gml:2" />)!',
    ];
    const grip = mechanicPage.replace(/^faq:\n(?: {2}.*\n)*/m, `faq:\n${answers.map((a, i) => `  - q: Question ${i + 1}?\n    a: >-\n      ${a}\n`).join("")}`);
    writeFileSync(join(dir, "snake.mdx"), readFileSync(here("content/wiki/snake.mdx"), "utf8"));
    writeFileSync(join(dir, "grip.mdx"), grip);
    const wiki = await loadWiki(options(dir, snakePlannedFile));
    for (const error of wiki.errors) fail(`plain text: ${error}`);
    const page = wiki.pages.find((p) => p.meta.slug === "grip");
    const got = page?.meta.jsonLd.find((node) => node["@type"] === "FAQPage")?.mainEntity.map((q) => q.acceptedAnswer.text) ?? [];
    const want = [
      "Names starting with ! aren't, nor ?, ,, . or ) ones.",
      "A Snake! A whip? A Snake, a rock. (A rope) and (a Snake); a bomb: no.",
      "It holds on. Not always (it slips)!",
    ];
    for (const [i, text] of want.entries()) {
      if (got[i] !== text) fail(`plain text: FAQ answer ${i + 1} is ${JSON.stringify(got[i])}, not ${JSON.stringify(text)}`);
    }
    if (failures === 0) console.log("ok   plain text: punctuation after inline code, links and citations");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

// Citations link to their lines in the source viewer (compile.ts's sourceHref), each kind: a <Cite gml> and
// an infobox's cite in References, the Source code list, an excerpt's caption and a quirk's Where, on the
// page and in its twin. Only what the media's gml/ has: a file it hasn't got, or lines past a file's end,
// stay text, and the page's warning lists them.
{
  const dir = mkdtempSync(join(tmpdir(), "test-wiki-"));
  try {
    const gml = join(dir, "gml");
    const file = (path, lines) => {
      mkdirSync(join(gml, path, ".."), { recursive: true });
      writeFileSync(join(gml, path), "x = 1;\n".repeat(lines));
    };
    file("objects/oSnake/step.gml", 80);
    file("objects/oSnake/create.gml", 10);
    file("objects/oJar/collision_oWhip.gml", 21);
    const content = join(dir, "content");
    mkdirSync(content);
    writeFileSync(join(content, "snake.mdx"), readFileSync(here("content/wiki/snake.mdx"), "utf8"));
    writeFileSync(join(content, "double-whip.mdx"), quirkPage);
    const planned = join(dir, "planned.yaml");
    writeFileSync(planned, snakePlanned);
    const wiki = await loadWiki({ ...options(content, planned), sourceDir: gml });
    for (const error of wiki.errors) fail(`source links: ${error}`);
    const page = (slug) => wiki.pages.find((p) => p.meta.slug === slug);
    const snake = page("snake");
    const href = (target) => snake?.meta.references.find((ref) => ref.target === target)?.href ?? null;
    const want = {
      "objects/oSnake/create.gml:10": "/wiki/source/objects/oSnake/create#L10",
      "objects/oSnake/step.gml:39-72": "/wiki/source/objects/oSnake/step#L39-L72",
      "objects/oSnake/step.gml:81-87": null,
      "objects/oEnemy/step.gml:76-106": null,
    };
    for (const [target, url] of Object.entries(want)) if (href(target) !== url) fail(`source links: ${target} links to ${href(target)}`);
    const sources = JSON.stringify(snake?.meta.sources.slice(0, 3));
    const wantSources = '[{"path":"objects/oSnake/create.gml","href":"/wiki/source/objects/oSnake/create"},{"path":"objects/oSnake/step.gml","href":"/wiki/source/objects/oSnake/step"},{"path":"objects/oEnemy/create.gml"}]';
    if (sources !== wantSources) fail(`source links: the Snake's sources are ${sources}`);
    if (!snake?.code.includes('"/wiki/source/objects/oSnake/step#L73-L76"')) fail("source links: the Snake's excerpt's caption");
    const where = page("double-whip")?.meta.quirk?.whereHref;
    if (where !== "/wiki/source/objects/oJar/collision_oWhip#L1-L21") fail(`source links: the quirk's Where links to ${where}`);
    const warned = wiki.warnings.filter((w) => w.includes("gml/"));
    if (warned.length !== 1 || !warned[0].startsWith("snake.mdx: the wiki media's gml/ hasn't got ")) fail(`source links: warnings ${JSON.stringify(warned)}`);
    // The first three, in the order the page cites them, and how many more.
    const wantWarning = /^snake\.mdx: the wiki media's gml\/ hasn't got objects\/oEnemy\/collision_oCharacter\.gml:25 \(no such file\), objects\/oSnake\/create\.gml:18-26 \(it has 10 lines\), objects\/oSnake\/step\.gml:81-87 \(it has 80 lines\) and \d+ more: not linked to the source viewer$/;
    if (!wantWarning.test(warned[0] ?? "")) fail(`source links: the warning is ${warned[0]}`);
    const md = markdownFiles(wiki.pages);
    const snakeMd = md.get("wiki/snake.md") ?? "";
    for (const line of [
      "[^1]: [`objects/oSnake/create.gml:10`](https://spelunky.ai/wiki/source/objects/oSnake/create#L10)",
      "- [`objects/oSnake/step.gml`](https://spelunky.ai/wiki/source/objects/oSnake/step)\n- `objects/oEnemy/create.gml`",
      "[`objects/oSnake/step.gml:73-76`](https://spelunky.ai/wiki/source/objects/oSnake/step#L73-L76), from `spelunky.gmk`",
    ]) {
      if (!snakeMd.includes(line)) fail(`source links: snake.md hasn't got ${line}`);
    }
    const whereRow = tableRow("Where", "[`objects/oJar/collision_oWhip.gml:1-21`](https://spelunky.ai/wiki/source/objects/oJar/collision_oWhip#L1-L21)");
    if (!whereRow.test(md.get("wiki/double-whip.md") ?? "")) fail("source links: double-whip.md's Where");
    if (failures === 0) console.log("ok   source links: citations link to the lines the media's gml/ has");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

// The hubs (src/wiki/build/hubs.ts): each lists its categories' pages and, not links, the pages to come
// (wiki-data's and the planned ones), by title ("The Mines" under M).
{
  const before = failures;
  const hub = (id) => committed.hubs.hubs.find((h) => h.id === id);
  const cards = (id) => hub(id)?.cards.map((c) => `${c.slug}${c.href ? "" : "?"}`).join(" ");
  // Each hub's scope, written or not ("?"), by title; and every written page is a link in its categories'.
  const want = {
    monsters: monsterScope(readFileSync(here("content/wiki-planned.yaml"), "utf8"), committed.pages.map((p) => p.meta)).join(" "),
    "items/weapons": "bow machete mattock pistol sceptre shotgun teleporter web-cannon whip",
    shops: "ankh-shop bomb-shop clothing-shop dice-house general-store kissing-parlor rare-shop weapon-shop",
    // The Game's mechanic pages, by their planned `category` while they're to come.
    "the-game": "controls credits endings level-editor scoring settings versions",
  };
  for (const [id, list] of Object.entries(want)) if (cards(id)?.replaceAll("?", "") !== list) fail(`hubs: ${id} lists ${cards(id)}, not ${list}`);
  for (const { meta } of committed.pages) {
    for (const id of meta.categories) if (!hub(id)?.cards.some((c) => c.slug === meta.slug && c.href)) fail(`hubs: ${id} doesn't link ${meta.slug}`);
  }
  const places = cards("places") ?? "";
  // Every place is written but the Tutorial, to come (by title, "Graveyard level" for restless-dead).
  if (places !== "black-market city-of-gold dark-level flooded-level restless-dead ice-caves jungle kalis-altar mines moai mothership olmecs-lair sacrifice-pit snake-pit temple tutorial? yeti-lair") fail(`hubs: places are ${places}`);
  // /wiki/items is its three categories' together.
  const items = hub("items")?.cards.length;
  const sum = ["items/equipment", "items/weapons", "items/carryables"].reduce((n, id) => n + hub(id).cards.length, 0);
  if (items !== sum) fail(`hubs: /wiki/items has ${items} cards, not ${sum}`);
  // The areas its pages are found in: the Snake's found_in, and where generation makes those to come.
  const areas = hub("monsters")?.areas.map((a) => a.slug).join();
  if (!areas?.startsWith("mines,jungle")) fail(`hubs: the monsters' areas are ${areas}`);
  if (hub("monsters")?.cards.find((c) => c.slug === "bat")?.areas.join() !== "mines,jungle") fail("hubs: the Bat isn't in the Mines and the Jungle");
  if (hub("monsters")?.cards.find((c) => c.slug === "snake")?.areas.join() !== "mines") fail("hubs: the Snake isn't found in the Mines");
  // A place's are its part_of's (an area's own page is part of none), so the places hub filters by area too.
  const placeAreas = (slug) => hub("places")?.cards.find((c) => c.slug === slug)?.areas.join();
  if (hub("places")?.areas.map((a) => a.slug).join() !== "mines,jungle,ice-caves,temple") fail(`hubs: the places' areas are ${hub("places")?.areas.map((a) => a.slug)}`);
  for (const [slug, want] of [["kalis-altar", "mines,jungle,ice-caves,temple"], ["snake-pit", "mines"], ["city-of-gold", "temple"], ["mines", ""]]) {
    if (placeAreas(slug) !== want) fail(`hubs: ${slug}'s areas are ${placeAreas(slug)}, not ${want}`);
  }
  const { home } = committed.hubs;
  // The featured pages: the most recently verified first.
  const verified = (slug) => committed.pages.find((p) => p.meta.slug === slug)?.meta.verified.date ?? "";
  const featured = home.featured.map((c) => c.slug);
  if (featured.length === 0 || featured.some((slug, i) => i > 0 && verified(slug) > verified(featured[i - 1]))) fail(`hubs: featured ${featured}`);
  if (home.written !== committed.pages.length) fail(`hubs: ${home.written} written`);
  // /wiki/quirks filters by what its quirks involve, from their pages' `related`: the double Jar roll's Jar,
  // Whip, Machete and Mattock are all items. Its cards have their ledes; the other hubs' haven't.
  const quirks = hub("quirks");
  const involves = quirks?.involves.map((h) => h.id).join();
  if (!involves?.startsWith("monsters,characters,items")) fail(`hubs: the quirks involve ${involves}`);
  const jarRoll = quirks?.cards.find((c) => c.slug === "double-jar-roll");
  if (jarRoll?.involves?.join() !== "items" || !jarRoll.lede?.startsWith("The double Jar roll is a quirk")) fail(`hubs: the double Jar roll's card is ${JSON.stringify(jarRoll)}`);
  if (hub("monsters")?.cards.some((c) => c.involves || c.lede)) fail("hubs: a monster's card has a quirk's involves or lede");
  // A planned quirk's card has them from its wiki-planned.yaml entry, so a filter shows it as a written one's.
  // Every planned quirk is written now, so the shortcut win's 0:00 is planned again, from its frontmatter, in a
  // copy of the pages without its own: it involves the shortcuts (a mechanic), speedrunning (a guide) and
  // scoring (a mechanic page, but the game's, by its `category`).
  const dir = mkdtempSync(join(tmpdir(), "test-wiki-"));
  try {
    const content = join(dir, "wiki");
    cpSync(here("content/wiki"), content, { recursive: true });
    rmSync(join(content, "shortcut-win-no-time.mdx"));
    const { title, type, card, relations, lede } = frontmatter("shortcut-win-no-time");
    const entry = { title, type, card, related: relations.related, lede: lede.trim() };
    const planned = join(dir, "planned.yaml");
    writeFileSync(planned, `${readFileSync(here("content/wiki-planned.yaml"), "utf8")}shortcut-win-no-time: ${JSON.stringify(entry)}\n`);
    const replanned = await loadWiki(options(content, planned));
    for (const error of replanned.errors) fail(`hubs: the shortcut quirk planned again: ${error}`);
    const shortcut = replanned.hubs.hubs.find((h) => h.id === "quirks")?.cards.find((c) => c.slug === "shortcut-win-no-time");
    if (shortcut?.href !== null || shortcut.involves?.join() !== "mechanics,guides,the-game" || !shortcut.lede?.startsWith("The shortcut win's 0:00 is a quirk")) {
      fail(`hubs: the planned shortcut quirk's card is ${JSON.stringify(shortcut)}`);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
  const bare = quirks?.cards.filter((c) => !c.lede || !c.involves?.length).map((c) => c.slug);
  if (bare?.length) fail(`hubs: quirk cards without a lede or what they involve: ${bare.join(", ")}`);
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
        .replace("    - whip\n    - machete\n    - mattock\n    - spikes\n", `    - ${cited(["whip"])}\n`);
    writeFileSync(join(dir, "snake.mdx"), source);
    writeFileSync(join(dir, "cobra.mdx"), page("cobra", "Cobra", "jungle, mines"));
    writeFileSync(join(dir, "golem.mdx"), page("golem", "Golem", "tutorial"));
    const { pages, errors } = await loadWiki(options(dir, snakePlannedFile));
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
      // A flying Bat isn't hurt by spikes, and the Snake's page links the real Bat's.
      .replace("    - spikes\n", "")
      .replaceAll("[[bat|Bat]]", "Bat");
    writeFileSync(join(dir, "bat.mdx"), bat);
    const { hubs, errors } = await loadWiki(options(dir, snakePlannedFile));
    for (const error of errors) fail(`hub cards: ${error}`);
    const monsters = hubs.hubs.find((h) => h.id === "monsters");
    const cards = monsters.cards.map((c) => `${c.slug}${c.href ? "" : "?"}`).join(" ");
    const want = monsterScope(snakePlanned)
      .map((slug) => (slug === "bat" || slug === "snake" ? slug : `${slug}?`))
      .join(" ");
    if (cards !== want) fail(`hub cards: monsters ${cards}, not ${want}`);
    else if (monsters.cards.find((c) => c.slug === "bat")?.areas.join() !== "mines,jungle")
      fail(`hub cards: the Bat's areas are ${monsters.cards.find((c) => c.slug === "bat")?.areas}`);
    else console.log(`ok   hub cards: ${cards}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

// A card's sprite (hubs.ts): a page's infobox image, or its `card`'s sprite or frame; one to come's object's
// sprite, or CARD_SPRITES's or its planned `card`'s. A frame the sprite hasn't got fails, as do a
// CARD_SPRITES slug that has a page or that's no page to come and a card for a page no hub lists (a planned
// page with neither a type nor a category).
{
  const frames = { sGiantSpider: 4, sGiantSpiderJump: 1, sSnakeWalkL: 4, sBombArmed: 2, sSignGeneral: 1, sFallLeft: 1, sBatHang: 1 };
  // The Giant Spider's jump is a 32-pixel sprite, the rest 16-pixel ones.
  const sprite = (name, frame) => {
    if (!(name in frames)) return `no sprite ${name}`;
    if (frame >= frames[name]) return `${name} has no frame ${frame}`;
    const size = name === "sGiantSpiderJump" ? 32 : 16;
    return { src: `${name}/${frame}`, width: size, height: size };
  };
  const page = (slug, image, card) => ({
    meta: { slug, title: slug, image: image === null ? null : { sprite: image }, categories: ["monsters"], verified: { date: "2026-10-27" }, lede: "" },
    places: [],
    card,
  });
  const errors = [];
  const { hubs, home } = makeHubs(
    // A mechanic page has no infobox image: its card is its own `card`'s sprite.
    [page("giant-spider", "sGiantSpider", { sprite: "sGiantSpiderJump" }), page("snake", "sSnakeWalkL", { frame: 2 }), page("jar", "sBombArmed", { frame: 9 }), page("grip", null, { sprite: "sFallLeft" })],
    {
      unwritten: [
        { slug: "general-store", title: "General Store", type: "shop", objects: [] },
        { slug: "stomping", title: "Stomping", type: "mechanic", objects: [], card: { sprite: "sFallLeft" } },
        { slug: "bat", title: "Bat", type: "monster", objects: ["oBat"] },
        { slug: "level-editor", title: "Level editor", type: null, objects: [], card: { sprite: "sFallLeft" } },
        // A planned page's category puts it on a hub without a type, or in another than its type's.
        { slug: "credits", title: "Credits", type: null, category: "the-game", objects: [], card: { sprite: "sFallLeft" } },
        { slug: "ghost", title: "Ghost", type: "monster", objects: ["oGhost"] },
      ],
      kinds: { bat: "monster" },
      areas: [],
      foundIn: () => [],
    },
    sprite,
    // The Ghost's sprite isn't in the media: with no card naming it, its card has none, and no error.
    (object) => ({ oBat: "sBatHang", oGhost: "sGhostLeft" })[object] ?? null,
    errors,
    [],
    { "general-store": { sprite: "sSignGeneral" }, "giant-spider": { sprite: "sGiantSpider" }, cobra: { sprite: "sSnakeWalkL" } },
  );
  const srcs = hubs
    .flatMap((hub) => hub.cards)
    .filter((card, i, all) => all.findIndex((c) => c.slug === card.slug) === i)
    .map((card) => `${card.slug}=${card.image?.src ?? null}`)
    .sort()
    .join(" ");
  const want = "bat=sBatHang/0 credits=sFallLeft/0 general-store=sSignGeneral/0 ghost=null giant-spider=sGiantSpiderJump/0 grip=sFallLeft/0 jar=null snake=sSnakeWalkL/2 stomping=sFallLeft/0";
  const wantErrors = [
    "jar.mdx: infobox.image.card: sBombArmed has no frame 9",
    "hubs.ts: CARD_SPRITES has giant-spider, which has a page now",
    "hubs.ts: CARD_SPRITES has cobra, which is no page to come",
    "wiki-planned.yaml: level-editor.card: level-editor is on no hub",
  ];
  const missing = wantErrors.filter((want) => !errors.some((e) => e.startsWith(want)));
  // The Giant Spider's card draws its 32-pixel sprite bigger than the Snake's 16-pixel one, the hub's sprites'
  // box is the Giant Spider's, and the monsters' tile on /wiki (the first written card's, its) fits it in 64.
  const drawn = (image) => image && `${image.width}x${image.height}`;
  const monsters = hubs.find((hub) => hub.id === "monsters").cards;
  const sizes = ["giant-spider", "snake"].map((slug) => drawn(monsters.find((card) => card.slug === slug).image)).join();
  const tile = drawn(home.hubs.find((hub) => hub.id === "monsters").image);
  const box = hubs.find((hub) => hub.id === "monsters").box;
  if (srcs !== want) fail(`card sprites: ${srcs}, not ${want}`);
  else if (missing.length > 0 || errors.length !== wantErrors.length) fail(`card sprites: errors ${JSON.stringify(errors)}`);
  else if (sizes !== "96x96,64x64" || tile !== "64x64" || box !== 96) fail(`card sprites: drawn at ${sizes}, the tile at ${tile}, in a box of ${box}`);
  else console.log(`ok   card sprites: ${srcs}`);
}

// A card's scale (hubs.ts's cardScale): a 16-pixel sprite at 4x, half again as big for each doubling, the
// nearest whole number, within the box. Each size, as a sprite the hubs have: a gem, a rock, a monster, Lava's
// 17-pixel tile, the Ghost, a boss, the Moai's 48-pixel column, Olmec, and one too big for the box, at 1x all
// the same; and Olmec and a boss in /wiki's 64.
{
  const scales = [4, 8, 16, 17, 24, 32, 48, 64, 160].map((size) => cardScale(size, 128)).join();
  const tiles = [64, 32, 4].map((size) => cardScale(size, 64)).join();
  if (scales !== "7,5,4,4,3,3,2,2,1" || tiles !== "1,2,7") fail(`card scales: ${scales} in 128, ${tiles} in 64`);
  else console.log(`ok   card scales: ${scales}`);
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
  // The title, as typed so far, a plural, any case and accents, ranks its page first; so does each page's
  // title. Every word must match.
  const first = { snake: "snake", sna: "snake", Snakes: "snake", "SNÄKE": "snake", "snake bite": "snake", bat: "bat", caveman: "caveman", "giant spider": "giant-spider", skeleton: "skeleton" };
  for (const [query, slug] of Object.entries(first)) {
    if (found(query).split(",")[0] !== slug) fail(`search: "${query}" finds ${JSON.stringify(found(query))}, not ${slug} first`);
  }
  // A word only the Snake's FAQ has, and one only its Trivia has.
  for (const query of ["poisonous", "mirrored"]) {
    if (found(query) !== "snake") fail(`search: "${query}" finds ${JSON.stringify(found(query))}, not snake`);
  }
  // A word of the body ("invincible"), a table's cells ("Machete" and "Stomp" in the Defeating table), and a
  // query's single letters ignored ("Olmec's" is "olmec" and "s").
  for (const query of ["invincible", "machete", "stomp", "olmec's"]) {
    if (!found(query).split(",").includes("snake")) fail(`search: "${query}" finds ${JSON.stringify(found(query))}, not snake`);
  }
  for (const query of ["", "  ", "xyzzy", "snake xyzzy"]) {
    if (found(query) !== "") fail(`search: "${query}" finds ${found(query)}`);
  }
  // A stop word alone is kept, and matches the start of a word, as it's typed on the way to one: "the" finds
  // the Mines, whose Trivia quotes "(in theory)".
  if (!found("the").split(",").includes("mines")) fail(`search: "the" finds ${JSON.stringify(found("the"))}, not mines`);
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

// The hover cards the build makes of the committed pages (plugin.ts's virtual:wiki/hover-cards, cards.ts):
// each written page's title, type, lede, sprite and first numbers, small enough that every page's fits in a
// chunk the browser loads on the first hover.
{
  const warn = console.warn;
  console.warn = () => {};
  const code = await wiki(options(here("content/wiki"))).load.call({}, "\0virtual:wiki/hover-cards").finally(() => (console.warn = warn));
  const { cards } = await import(`data:text/javascript,${encodeURIComponent(code)}`);
  const before = failures;
  const slugs = committed.pages.map((page) => page.meta.slug).sort().join();
  if (Object.keys(cards).sort().join() !== slugs) fail(`hover cards: for ${Object.keys(cards)}, not every page (${slugs})`);
  // Without the media, no sprite.
  const want = {
    title: "Snake",
    kind: "Monster",
    lede: snake?.meta.lede,
    image: null,
    stats: [["Health", "1 hit point"], ["Damage", "1 heart"], ["Speed", "1 pixel per step"]],
  };
  if (JSON.stringify(cards.snake) !== JSON.stringify(want)) fail(`hover cards: the Snake's is ${JSON.stringify(cards.snake)}`);
  // With them: the sprite at the @4x file's size, or a half or a quarter of it to fit 96 pixels; its first
  // frame for reduced motion only when it's animated. The numbers are the infobox's data values (not its
  // texts), three at most.
  const url = "/wiki/media/sprites";
  const image = (name, size, anim) => ({
    sprite: name, anim, alt: "", src: `${url}/${name}/${anim ? "anim" : "frames/0"}%404x.png`, still: `${url}/${name}/frames/0%404x.png`, width: size, height: size / 2,
  });
  const row = (label, value) => ({ field: label.toLowerCase(), label, text: value === undefined ? undefined : `${value} things`, value, notes: [] });
  const meta = (slug, img, infobox = []) => ({ slug, title: slug, type: "monster", lede: "x".repeat(160), image: img, infobox });
  const made = hoverCards([
    meta("small", image("sSmall", 64, true), [row("Health", "1"), row("Behaviour"), row("Damage", "2"), row("Speed", "3"), row("Range", "4")]),
    meta("wide", image("sWide", 192, false)),
    meta("huge", image("sHuge", 384, true)),
    meta("blank", { ...image("sBlank", 0, false), height: 0 }),
  ]);
  const sizes = Object.values(made).map((card) => card.image && `${card.image.width}x${card.image.height}`).join();
  if (sizes !== "64x32,96x48,96x48,") fail(`hover cards: sprites drawn at ${sizes}`);
  if (made.small.image.still !== `${url}/sSmall/frames/0%404x.png` || made.wide.image.still !== null) fail("hover cards: still frames");
  if (made.small.stats.map(([label]) => label).join() !== "Health,Damage,Speed") fail(`hover cards: stats ${JSON.stringify(made.small.stats)}`);
  // A card with the longest lede, a sprite and three numbers is under 600 bytes, so the chunk stays under
  // site/test.sh's bound of 700 a page.
  const longest = JSON.stringify(made.small).length;
  if (longest > 600) fail(`hover cards: a full card is ${longest} bytes`);
  if (failures === before) console.log(`ok   hover cards: ${Object.keys(cards).length} page(s), a full card ${longest} bytes`);
}

// A clip plays live when the media have its replay and the web build at /play/ plays it: a scene's in any
// embeddable build, one cut from a committed replay only in a build that says it plays them (build.sh's
// WIKI_LIVE_REPLAYS, from 851-3572's marker), and the settings program's, which has no replay, in none.
{
  const before = failures;
  const url = (path) => `/wiki/media/clips/${path}`;
  const scene = { replay: "snake/bite/replay.txt", from: 281, scene: true };
  const cut = { replay: "snake/stomp/replay.txt", from: 1415, scene: false };
  const build = (replays) => ({ seek: true, phones: true, replays });
  const plays = (live, liveClips) => JSON.stringify(clipLive(live, liveClips, url));
  const want = (live) => JSON.stringify({ replay: url(live.replay), from: live.from, seek: true, phones: true });
  if (plays(scene, build(false)) !== want(scene)) fail(`live clips: a scene's is ${plays(scene, build(false))}`);
  if (plays(cut, build(false)) !== "null") fail(`live clips: a replay's plays in a build that doesn't say it can`);
  if (plays(cut, build(true)) !== want(cut)) fail(`live clips: a replay's is ${plays(cut, build(true))}`);
  if (plays(scene, null) !== "null" || plays(null, build(true)) !== "null") fail("live clips: without the build or the replay");
  if (failures === before) console.log("ok   live clips: a replay's clip plays only in a build that says it can");
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
  // A citation or a number alone on its source line is a block that ends its paragraph (851-3408): the
  // error names the line.
  [
    "a citation alone on its line",
    { "snake.mdx": edit('walks off one <Cite gml="objects/oSnake/step.gml:41-54" />. Where', 'walks off one\n<Cite gml="objects/oSnake/step.gml:41-54" />\nWhere') },
    `snake.mdx:${source.split("\n").findIndex((l) => l.includes("walks off one <Cite")) + 2}: <Cite> starts a line with nothing else on it`,
  ],
  ["a number alone on its line", { "snake.mdx": `${source}\n- A Snake has\n  <Stat id="snake.hp" />\n  in all.\n` }, "<Stat> starts a line with nothing else on it"],
  ["a generated section written by hand", { "snake.mdx": `${source}\n## FAQ\n` }, 'the layout generates "FAQ"'],
  ["no frontmatter", { "snake.mdx": "# Snake\n" }, "no frontmatter"],
  ["code between comments", { "snake.mdx": `${source}\n{/* a */ globalThis.fetch("x") /* b */}\n` }, "a page has no {expressions}"],
  ["an HTML attribute that's code", { "snake.mdx": `${source}\n<span title={Date.now()}>x</span>\n` }, "<span>'s attributes are strings"],
  ["an event handler", { "snake.mdx": `${source}\n<img src="x" alt="" onError="alert(1)" />\n` }, "<img> has an event handler"],
  ["a script", { "snake.mdx": `${source}\n<script>alert(1)</script>\n` }, "a page has no <script>"],
  ["two headings with one anchor", { "snake.mdx": `${source}\n## Behaviour\n` }, "two headings have the anchor #behaviour"],
  ["a heading with the contents' anchor", { "snake.mdx": `${source}\n## TOC\n` }, "#toc is one of the layout's anchors"],
  ["a heading with the contents rail's anchor", { "snake.mdx": `${source}\n## TOC rail\n` }, "#toc-rail is one of the layout's anchors"],
  ["a heading for the contents", { "snake.mdx": `${source}\n## Contents\n` }, 'the layout generates "Contents"'],
  ["a heading with no anchor", { "snake.mdx": `${source}\n## ???\n` }, "has no letters or digits"],
  ["an element with the layout's anchor", { "snake.mdx": `${source}\n<div id="toc">x</div>\n` }, '<div id="toc">: #toc is one'],
  ["a date that doesn't exist", { "snake.mdx": edit("date: 2026-10-27", "date: 2026-13-01") }, "a date that exists"],
  ["a relation listing a page twice", { "snake.mdx": edit("- whip\n", "- whip\n    - whip\n") }, "damaged_by lists whip twice"],
  ["a heading with a navbox's anchor", { "snake.mdx": `${source}\n## Navbox monsters\n` }, "#navbox-monsters is one of the layout's anchors"],
  ["a stat that's text", { "snake.mdx": `${source}\nIt is <Stat id="oSnake.type" />.\n` }, 'is text ("Snake")'],
  ["a value the create event changes later", { "snake.mdx": `${source}\nA Frog: <Stat id="oFrog.swimming" />.\n` }, "changes swimming again"],
  // A table stat (a price by level) is read a row at a time.
  ["a table without a row", { "snake.mdx": `${source}\nIt costs <Stat id="spectacles.price-by-level" />.\n` }, 'is a table: read a row with at="..."'],
  ["a table's row it hasn't got", { "snake.mdx": `${source}\nIt costs <Stat id="spectacles.price-by-level" at="17" />.\n` }, "has no row at=17 (its rows are 1 to 16)"],
  ["a row of a stat that isn't a table", { "snake.mdx": `${source}\nIt has <Stat id="snake.hp" at="2" />.\n` }, "isn't a table, so it has no row at=2"],
  // A page without an infobox names its hub card's sprite in `card`; one with an infobox, in its image.
  ["a card on a page with an infobox", { "snake.mdx": edit("slug: snake", "slug: snake\ncard: { sprite: sSnakeWalkL }") }, "a page with an infobox gives its card as infobox.image.card"],
  ["a card without a sprite", { "snake.mdx": source, "grip.mdx": mechanicPage.replace("card: { sprite: sFallLeft }", "card: { frame: 1 }") }, "a page without an infobox names its card's sprite"],
  // A quirk page names its rows as docs/quirks.md has them, in a section it has.
  ["a quirk row docs/quirks.md hasn't got", { "snake.mdx": source, "double-whip.mdx": quirkPage.replace("rolls its contents twice", "rolls three times") }, 'quirk.rows: no row of docs/quirks.md §1 starts "A whipped Jar rolls three times."'],
  ["a quirk section docs/quirks.md hasn't got", { "snake.mdx": source, "double-whip.mdx": quirkPage.replace('entry: "1"', 'entry: "42"') }, "quirk.entry: docs/quirks.md has no section 42"],
  ["a quirk row that isn't a sentence", { "snake.mdx": source, "double-whip.mdx": quirkPage.replace("rolls its contents twice.", "rolls") }, "a row's opening: a sentence or more"],
  ["a row that isn't a number", { "snake.mdx": `${source}\nIt costs <Stat id="spectacles.price-by-level" at="five" />.\n` }, 'at= is a whole number, not "five"'],
];
// A planned page list, changed.
const planned = snakePlanned;
const plannedEdit = (from, to) => {
  if (!planned.includes(from)) throw new Error(`the Snake's planned pages have no ${JSON.stringify(from)}: update test-wiki.mjs`);
  return planned.replace(from, to);
};
cases.push(
  ["a link to no page", { "snake.mdx": `${source}\nSee [[jars]].\n` }, "[[jars]]: no page and no planned page"],
  ["a link to a page with a typo", { "snake.mdx": edit("[[mines|the Mines]] (levels 1–4)", "[[mine|the Mines]] (levels 1–4)") }, "[[mine]]: no page and no planned page"],
  ["a link to the page itself", { "snake.mdx": `${source}\nSee [[snake|Snakes]].\n` }, "[[snake]] is this page"],
  ["a link to a section a page hasn't got", { "snake.mdx": source, "whip.mdx": itemPage("whip", "Whip", "oWhip", "", "It hits a [[snake#fangs|Snake]].") }, "snake has no #fangs"],
  ["a relation to no page", { "snake.mdx": edit("found_in: [mines, snake-pit, tutorial]", "found_in: [mines, snake-pit, tutorial, caves]") }, "relations.found_in: caves: no page and no planned page"],
  ["a relation the data doesn't back", { "snake.mdx": edit("found_in: [mines, snake-pit, tutorial]", "found_in: [mines, snake-pit, tutorial, jungle]") }, "relations.found_in: jungle: relations.json doesn't say so for oSnake"],
  // Kali's gifts for a body left on her altar (scrGetFavorMsg, from oEnemy's step, which the Snake runs) are
  // the altar's drops, not the Snake's.
  ["a monster dropping Kali's gift", { "snake.mdx": edit("  found_in: [mines,", "  drops: [jetpack]\n  found_in: [mines,") }, "relations.drops: jetpack: relations.json doesn't say so for oSnake"],
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
  // A place may name objects only when it's a thing in a level (Kali's altar), not an area or special level.
  [
    "a special level with objects",
    {
      "snake.mdx": source,
      "snake-pit.mdx": itemPage("snake-pit", "Snake pit", "oX")
        .replace("type: item", "type: place")
        .replace("categories: [items/weapons]", "categories: [places]")
        .replace("  kind: weapon\n", ""),
    },
    "snake-pit.mdx: an area or special level has no objects",
  ],
  ["a planned page that's wiki-data's", { "snake.mdx": source, "planned.yaml": `${planned}whip: { title: Whip }\n` }, "whip is wiki-data's already (Whip)"],
  ["a planned page with a page", { "snake.mdx": source, "level-editor.mdx": itemPage("level-editor", "Level editor", "oX") }, "level-editor has a page now: remove it"],
  ["a planned page without a title", { "snake.mdx": source, "planned.yaml": plannedEdit("tutorial: { title: Tutorial, type: place, rooms: [rTutorial], card: { sprite: sTutorialSign } }", "tutorial: { type: place, rooms: [rTutorial], card: { sprite: sTutorialSign } }") }, "planned.yaml: tutorial.title"],
  ["an alias that's wiki-data's slug", { "snake.mdx": edit("slug: snake", "slug: snake\naliases: [spider]") }, "alias spider is wiki-data's slug for Spider"],
  ["a relation to the page's own alias", { "snake.mdx": edit("slug: snake", "slug: snake\naliases: [serpent]").replace("  found_in: [mines,", "  related: [serpent]\n  found_in: [mines,") }, "related: serpent: a page doesn't relate to itself"],
  ["a link to a section the page doesn't show", { "snake.mdx": source, "whip.mdx": itemPage("whip", "Whip", "oWhip", "", "It hits a [[snake#gallery|Snake]].") }, "snake has no #gallery"],
  ["a relation to a planned page of the wrong type", { "snake.mdx": source, "planned.yaml": plannedEdit("tutorial: { title: Tutorial, type: place", "tutorial: { title: Tutorial, type: mechanic") }, "tutorial is a mechanic page"],
  [
    "a room on two pages",
    { "snake.mdx": source, "planned.yaml": plannedEdit("tutorial: { title: Tutorial, type: place, rooms: [rTutorial], card: { sprite: sTutorialSign } }", "tutorial: { title: Tutorial, rooms: [rTutorial], card: { sprite: sTutorialSign } }\nintro: { title: Intro, rooms: [rTutorial] }") },
    "rTutorial is tutorial's and intro's",
  ],
  ["a planned page nothing links to", { "snake.mdx": source, "planned.yaml": `${planned}cobra: { title: Cobra }\n` }, "no page links to cobra, and it has no type or category"],
  ["a planned page of its type's category", { "snake.mdx": source, "planned.yaml": `${planned}cobra: { title: Cobra, type: monster, category: monsters }\n` }, "cobra.category: monsters is its type's hub already"],
  ["a planned page with a page's title", { "snake.mdx": source, "planned.yaml": `${planned}serpent: { title: Snake, type: monster }\n` }, "serpent's title is snake.mdx's: remove it"],
  ["a planned page of no category", { "snake.mdx": source, "planned.yaml": `${planned}cobra: { title: Cobra, category: snakes }\n` }, "planned.yaml: cobra.category"],
  ["a planned page that's a page's alias", { "snake.mdx": edit("slug: snake", "slug: snake\naliases: [serpent]"), "planned.yaml": `${planned}serpent: { title: Serpent }\n` }, "serpent is snake.mdx's alias"],
  ["a planned page with another's object", { "snake.mdx": source, "planned.yaml": plannedEdit("rooms: [rTutorial]", "objects: [oSnake], rooms: [rTutorial]") }, "oSnake is snake's and tutorial's"],
);
// A planned quirk, with the `related` and `lede` its card on /wiki/quirks shows, changed.
const plannedQuirk = 'snake-double-bite: { title: Snakes biting twice, type: quirk, related: [snake], lede: "Snakes biting twice is a quirk of Spelunky Classic in which a Snake bites twice." }\n';
const quirkEdit = (from, to) => {
  if (!plannedQuirk.includes(from)) throw new Error(`the planned quirk has no ${JSON.stringify(from)}: update test-wiki.mjs`);
  return { "snake.mdx": source, "planned.yaml": `${planned}${plannedQuirk.replace(from, to)}` };
};
cases.push(
  ["a planned quirk without a lede", quirkEdit(', lede: "Snakes biting twice is a quirk of Spelunky Classic in which a Snake bites twice."', ""), "snake-double-bite is a planned quirk: its card on /wiki/quirks needs its lede"],
  ["a planned quirk without its related", quirkEdit(" related: [snake],", ""), "snake-double-bite is a planned quirk: its card on /wiki/quirks needs its related pages"],
  ["a planned quirk related to no page", quirkEdit("related: [snake]", "related: [snake, cobra]"), "snake-double-bite.related: cobra: no page and no planned page"],
  ["a planned quirk related to itself", quirkEdit("related: [snake]", "related: [snake-double-bite]"), "snake-double-bite.related: snake-double-bite: a page doesn't relate to itself"],
  ["a planned quirk relating a page twice", quirkEdit("related: [snake]", "related: [snake, snake]"), "snake-double-bite.related: snake is listed twice"],
  ["a planned quirk's lede with a link", quirkEdit("a Snake bites", "a [[snake]] bites"), "planned.yaml: snake-double-bite.lede: plain text"],
  ["a planned quirk's lede too long", quirkEdit("bites twice.", `bites${" and bites".repeat(10)}.`), "planned.yaml: snake-double-bite.lede"],
  ["a planned quirk's lede with emphasis", quirkEdit("bites twice", "bites _twice_"), "planned.yaml: snake-double-bite.lede: plain text"],
  ["a planned quirk on another hub", quirkEdit("type: quirk,", "type: quirk, category: the-game,"), "snake-double-bite.category: a quirk is on /wiki/quirks"],
  // Cobra is on no hub (no type or category), so the quirk involves nothing a filter shows; it's planned, as
  // the quirk's `related` names it.
  [
    "a planned quirk involving nothing on a hub",
    { "snake.mdx": source, "planned.yaml": `${planned}cobra: { title: Cobra }\n${plannedQuirk.replace("related: [snake]", "related: [cobra]")}` },
    "snake-double-bite.related: none of its pages is on a hub",
  ],
  ["a lede on a planned page that isn't a quirk", { "snake.mdx": source, "planned.yaml": `${planned}cobra: { title: Cobra, type: monster, lede: "The Cobra." }\n` }, "cobra.lede: only a planned quirk has one"],
);
for (const [what, files, want] of cases) {
  const dir = mkdtempSync(join(tmpdir(), "test-wiki-"));
  try {
    for (const [name, text] of Object.entries(files)) writeFileSync(join(dir, name), text);
    // A case's own planned pages, or the Snake's.
    const plannedFile = files["planned.yaml"] === undefined ? snakePlannedFile : join(dir, "planned.yaml");
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
console.log(`site/test-wiki.mjs: all ${cases.length + 7} passed`);
