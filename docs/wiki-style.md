# Wiki style guide

How to write a page for the spelunky.ai wiki (851-3329): what a page says, in what order, in what voice, and
how it proves it. It's for the content tickets (about 190 pages), so it's a set of rules more than a
discussion. Each page type has a skeleton in [`wiki/templates/`](../wiki/templates), and
[`wiki/templates/examples/snake.mdx`](../wiki/templates/examples/snake.mdx) is a finished page to copy from.

The wiki's other rules live beside this one: what a page may show and the notices it carries
([wiki license](wiki-license.md)), where sprites and clips come from ([wiki media](wiki-media.md)), and where
numbers and relations come from ([wiki data](wiki-data.md)).

**The page model below is a proposal.** The content schema and MDX pipeline (851-3333) aren't built yet;
that ticket turns the frontmatter here into a zod schema and the components into code, and may rename
things. Until then, write pages to this model, and when 851-3333 lands, it updates this guide and the
templates in the same PR.

## The page model

A page is `wiki/pages/<slug>.mdx`: YAML frontmatter, then an MDX body. The build validates the frontmatter,
resolves every link, stat and clip, and fails on anything it can't (851-3333, 851-3334). `wiki/templates/`
isn't built.

### Frontmatter

Every page:

| Field | Contents |
| --- | --- |
| `slug` | The page's URL, `/wiki/<slug>`; the file's name. See [Naming](#naming-and-slugs). |
| `title` | An entity's name as the game writes it, in title case: `Snake`, `Pitcher's Mitt`, `Olmec's Lair`. A mechanic, guide or quirk page's is a short description in sentence case: `Stomping`, `Reaching the City of Gold`, `Stomping while jumping`. |
| `type` | The page type, which picks the template, the infobox and the JSON-LD: `monster`, `item`, `treasure`, `trap`, `place`, `shop`, `character`, `mechanic`, `guide` or `quirk`. One per page. |
| `categories` | The hubs that list it (one or more): `monsters`, `characters`, `items/equipment`, `items/weapons`, `items/carryables`, `treasure`, `traps`, `places`, `shops`, `mechanics`, `guides`, `quirks`, `the-game`. The Shopkeeper is `[characters, monsters]`; the URL doesn't change with them. |
| `objects` | The game objects the page is about (`[oSpider, oSpiderHang]`): the keys into `stats.json`, `relations.json` and the sprite index. An object belongs to one page at most; that's how the build maps an object to a slug. Places and shops have none: their slug is the relations' area, level or shop slug, and a place lists the editor-built rooms it covers in `rooms` (`[rOlmec]`). |
| `lede` | The [one-sentence lede](#the-lede). It's also the meta description and the answer engines' snippet. |
| `aliases` | Other names people search for (`[mega-mouth, jaws]`): redirects and search terms. Optional. |
| `hatnote` | One line above the lede for a confusable page: `For the bigger spider, see [[giant-spider]].` Optional. |
| `infobox` | The type's [infobox fields](#infobox-fields). Not on mechanic, guide or quirk pages. |
| `quirk` | Quirk pages only, in place of `infobox`: `status`, `where`, `pinned_by` and `entry` ([fields](#infobox-fields)). |
| `relations` | Typed links to other pages ([Relations](#relations)). |
| `clips` | The page's clips, by id (`snake/bite`), the first being the page's main clip ([Clips](#clips-and-images)). |
| `faq` | Three to five [questions and answers](#faq-blocks). Not on guides, which are the answer. |
| `sources` | The code that defines the thing: object events and scripts, without lines (`objects/oSnake/step.gml`). Rendered as a "Source code" list under References; line-level citations go in the body ([Citing](#citing-the-code)). |
| `spoiler` | `true` on a page about a secret ([Spoilers](#spoilers)). Optional. |
| `verified` | `{ port: v0.1.7, date: 2026-10-27 }`: the port release the page was last checked against and when ([Verify](#verify-before-publishing)). Shown as "Verified against v0.1.7". |

Strings in `lede`, `hatnote`, infobox `text` and FAQ answers are inline MDX: they may use `[[links]]` and
`<Stat>`, and are rendered to plain text where plain text is needed (meta tags, JSON-LD).

### Infobox values

An infobox field is one of:

- `{ stat: snake.hp }`: a stat from `stats.json`, by entity slug and stat id. The build fills in the value,
  the unit and a citation of the line it came from.
- `{ init: oSnake.image_speed }`: a variable an object's create event sets (`stats.json`'s `objects`).
- `{ text: "...", cite: objects/oSnake/step.gml:39-72 }`: hand-written, with a citation. A `text` with a
  number in it must cite the line the number comes from; a `text` with none (a short description) may leave
  `cite` out if the body cites it.
- `{ sprite: sSnakeWalkL, anim: true, alt: "..." }`: the `image` field only.

Relations shown in the infobox (`found_in`, `sold_in`, `drops`) come from `relations`, not from `infobox`.

### Relations

`relations` maps a relation to a list of slugs. Each entry is a slug, which the build checks against
`relations.json` for one of the page's `objects` (counting inherited events, its `runs_as`), or
`{ slug, cite, note }` for one the data can't see, such as a mechanic or a thrown item. The build fails on a
plain slug the data doesn't back, and lists (as a warning) what the data has that the page leaves out, so a
page curates its relations but can't contradict the code. A page without `objects` (a place, a shop) has
nothing to check a plain slug against, so its entries take a `cite`.

| Relation | On | Points to | Inverse (generated) |
| --- | --- | --- | --- |
| `found_in` | monster, item, treasure, trap, character, shop | places | "Found here" on the place |
| `part_of` | place (a special level, a room) | places (its area) | "Contains" |
| `sold_in` | item | shops | "Sells" on the shop |
| `drops` | monster, item (containers), trap | items, treasure | `dropped_by` |
| `damaged_by` | monster, character | anything | `damages` |
| `related` | any | anything | "See also" on both |

Write a relation on one side only: the side the data keys it by (a monster's `found_in`, an item's
`sold_in`). The inverse appears by itself.

### Body components

| Component | Use |
| --- | --- |
| `[[slug]]`, `[[slug\|text]]`, `[[slug#anchor]]` | A wiki link ([Links](#links-and-navboxes)). |
| `<Stat id="snake.hp" />` | A number from `stats.json`, with its unit (`1 hit point`). `id="oSnake.hp"` reads an object's create-event value. `as="seconds"` turns steps into seconds and `as="per-second"` px/step into px/s (30 steps a second, `room_speed`; two decimals at most); `bare` leaves the unit off. |
| `<Cite gml="objects/oSnake/step.gml:66-71" />` | A citation: a numbered note linking to the lines ([Citing](#citing-the-code)). Also `replay=`, `clip=`, `quirk=` and `room=` (what the room editor placed in a room, `relations.json`'s `placed_in`). |
| ```` ```gml objects/oSnake/step.gml:73-76 ```` | A short GML excerpt: a fenced code block whose info string is `gml` and the lines' citation. The build adds the license's caption and checks the lines hold the text. A fence, not a component, because MDX would read the code's `{` and `<` as JavaScript and JSX. |
| `<Clip id="snake/bite" />` | A clip, with its title, the caption and an accessible description from the spec. |
| `<Sprite name="sSnakeWalkL" anim alt="..." />` | A sprite, still or animated, at an integer scale. |

The build generates the infobox, the hatnote, the lede, the FAQ, the Gallery (clips the body doesn't
place), See also, References, What links here and the navboxes. Don't write their headings.

## Tone

Write like an encyclopedia about a game: Wikipedia's register, a Fandom wiki's coverage.

- **Present tense, third person.** "The Snake bites for 1 heart", not "will bite you". "You" is for guides
  only. The controlled character is "the player".
- **Facts, not opinions.** No "annoying", "deadly", "the best weapon", "a must-have". Say what it does and
  let the numbers make the point: "kills a Snake in one hit" rather than "makes short work of Snakes".
- **No speculation.** Not what the developer meant, what was cut, or what fans think, unless the code shows
  it (a commented-out sprite, an unused state). Say what the code shows, and stop there.
- **Spelunky Classic only.** Spelunky HD and Spelunky 2 are other games with other rules: never import
  their names, numbers or behaviour, and don't compare to them except on a page about the game itself.
- **Names.** Capitalize a game entity's name when it means the entity: "a Snake", "two Bats", "the Whip",
  "the Mines", "the Black Market"; generic words stay lowercase ("monsters", "a trap", "an item"). The
  capital tells "the Rock" from "rock". Spell names as the game does ("Kissing Parlor", "Pitcher's Mitt");
  write the prose in British English, as the rest of the repo's docs are ("behaviour", "favour").
- **Plain words.** Explain the code's terms on first use ("hit points, its health"), and say "step" (a 30th
  of a second) rather than "frame" when it's a game tick. Don't write the code's names (variables, objects,
  sprites) in prose, except in Trivia, a quirk page or a mechanic's How it works, in code style: "the parent
  object most monsters share", not oEnemy.
- **Short.** A monster page is about 300 to 600 words of prose. Tables carry the numbers; prose carries
  what they mean.

## The lede

The `lede` is one sentence that answers "what is this?" directly, so an answer engine can quote it alone:

- It starts with the title and says what kind of thing it is in Spelunky Classic, then the one or two facts
  people most want: what it does, where it is, its price, how to beat it.
  > The Snake is a monster in Spelunky Classic, found mostly in the Mines, that slithers back and forth
  > along the ground and bites for 1 heart on contact.

  A mechanic, guide or quirk page's lede starts with its title too, and says what happens: "Stomping while
  jumping is a quirk of Spelunky Classic in which..."
- It names the game, "in Spelunky Classic" (not "Spelunky 1" or "the original"), since it's read out of
  context.
- 160 characters at most, rendered: it's the meta description too.
- No links, no clause that needs the rest of the page ("as described below"), no superlatives.
- Its numbers are `<Stat>`s, like the body's.

The layout renders the lede as the page's first sentence, so the body doesn't repeat it: the body opens
with the rest of the summary (two to four sentences, no heading), the facts that come next in importance.

## Naming and slugs

A slug is the thing's name in lowercase kebab case: letters and digits, words joined by `-`, apostrophes
and other punctuation dropped: `giant-spider`, `pitchers-mitt`, `olmecs-lair`, `kali`. URLs are flat
(`/wiki/<slug>`) whatever the type, so a slug never says what kind of page it is.

**Which name.** In this order:

1. What the game shows the player: a shop's sign or pickup message (`YOU GOT CLIMBING GLOVES!`, so
   `climbing-gloves` for `oGloves`), a level's message, the end-of-game stats (`MEGAMOUTH` for `oJaws`,
   whose `type` says "Mega Mouth"; `HAWKMAN` for `oHawkman`, whose `type` is a copied "Yeti").
2. The source's own name for it: its `type` string, or the comment that introduces it.
3. The object's name without the `o` prefix and without direction or state suffixes, as words:
   `oArrowTrapLeft` is `arrow-trap`, `oSmashTrap` is `smash-trap`.

The `wiki/data/*.toml` specs already use these slugs; a page uses its spec entity's slug, and if they
disagree, fix one in the same PR.

**One page per thing the player sees as one.** Objects that are one thing in different states share a
page: `oSpiderHang` (waiting on the ceiling) and `oSpider` (dropped) are both `spider`; `oSnake` is
`snake`. Things the game names differently get different pages: `spider` and `giant-spider`, `bat` and
`vampire`. A page that's both a character and a monster, or a character and a carryable, is one page with
two categories (`shopkeeper`, `damsel`).

**Collisions.** When two pages would get the same slug, the one that isn't the plain name takes a kind word
as a suffix, never a parenthesis: `arrow` (the item) and `arrow-trap`; `ankh` and `ankh-shop`; `olmec` and
`olmecs-lair`. Then give each a `hatnote` pointing at the other. A hub's slug (`monsters`, `items`,
`treasure`, `traps`, `places`, `shops`, `mechanics`, `guides`, `quirks`, `characters`, `the-game`) is never a
page's. Mechanic, guide and quirk slugs are short noun phrases: `stomping`, `throwing`,
`city-of-gold-route`, `jump-stomp`.

Slugs are permanent: renaming one needs an `aliases` entry for the old one, so links from outside keep
working.

## Page types

Each type's section headings are fixed, in this order, so anchors are the same on every page of a type
(`/wiki/snake#behaviour`, `/wiki/bat#behaviour`). Leave out a section with nothing to say; don't add one
outside the list without changing this guide. Generated parts are in italics.

| Type | Sections |
| --- | --- |
| monster | *Infobox, lede*, summary, Behaviour, Damage, Defeating, Locations, Tips, Trivia, *FAQ, Gallery, See also, References* |
| item: equipment | *Infobox, lede*, summary, Effect, Obtaining, Interactions, Tips, Trivia, *FAQ, …* |
| item: weapon | *Infobox, lede*, summary, Usage, Damage, Obtaining, Interactions, Tips, Trivia, *FAQ, …* |
| item: carryable | *Infobox, lede*, summary, Usage, Locations, Interactions, Tips, Trivia, *FAQ, …* |
| treasure | *Infobox, lede*, summary, Value, Locations, Collecting, Trivia, *FAQ, …* |
| trap | *Infobox, lede*, summary, How it works, Damage, Locations, Avoiding, Interactions, Trivia, *FAQ, …* |
| place | *Infobox, lede*, summary, Generation, Inhabitants, Hazards, Items and shops, Secrets, Trivia, *FAQ, …* |
| shop | *Infobox, lede*, summary, Stock, Prices, Shopkeeper, Stealing, Locations, Trivia, *FAQ, …* |
| character | *Infobox, lede*, summary, Behaviour, Interactions, Locations, Trivia, *FAQ, …* |
| mechanic | *Lede*, summary, How it works, Numbers, Examples, Quirks, *FAQ, …* |
| guide | *Lede*, summary, Requirements, Steps, Tips, Common mistakes, *See also, References* |
| quirk | *Quirk box, lede*, summary, What happens, Why, Reproducing it, In the port, *FAQ, …* |

The Game section's pages (controls, settings, the level editor, scoring, endings, credits, versions:
851-3361) are mechanic pages in the `the-game` category.

Notes on the sections:

- **Behaviour** (monster, character): what it does on its own, state by state, in the order the player
  meets them (asleep, then waking, then attacking).
- **Damage** (monster, weapon, trap): what it does to the player, or a weapon's to monsters, as a table
  when there's more than one case.
- **Defeating** (monster): every way it takes damage or dies, as a table: way, damage, citation. Then
  what the kill counts for (kill tallies, Kali's favour).
- **Locations**: where generation makes it and how likely, from `relations.json`; the level editor's tile
  code if it has one.
- **Tips**: practical consequences of the verified facts ("A Snake never walks off a ledge, so one on a
  platform stays on it"). Every tip must follow from something on the page. No second person.
- **Trivia**: oddities in the code (unused states and sprites, copied `type` strings, comments), each
  cited. Bugs the game keeps link to their quirk page.
- **Obtaining** / **Stock** / **Prices**: from `relations.json`'s shops and `stats.json`'s price tables;
  show the price-by-level table rather than describing it.
- **Generation** (place): which levels, how it's chosen and with what odds, how its rooms are laid out.
- **Why** (quirk): the code that causes it, with a short excerpt. **In the port**: that the port
  reproduces it (it always does, `docs/quirks.md`), and the off-by-default option that fixes it, if one
  exists.
- **Steps** (guide): a numbered list, one action each, in the second person ("Throw the Rock at the
  Snake").

### Infobox fields

"Data" means the build fills the value from `stats.json`/`relations.json` (`{ stat }`, `{ init }` or a
relation); "hand" means `{ text, cite }`. Every type has `image` (a sprite, hand-picked) and the generated
`objects`.

| Type | Field: from |
| --- | --- |
| monster | `health`: data (`<slug>.hp`); `damage`: data (`<slug>.damage`, contact damage); `speed`: data (a `*-speed` stat); `behaviour`: hand, under ten words; `found_in`, `drops`: relations |
| item | `kind`: `equipment`, `weapon` or `carryable`; `effect`: hand, under ten words; `damage`: data (weapons); `price`: data (`<slug>.price`); `sold_in`, `found_in`: relations |
| treasure | `value`: data (`<slug>.value`, or the area table `<slug>.value-by-area` in Value); `found_in`: relations |
| trap | `damage`: data or hand; `trigger`: hand; `found_in`, `drops`: relations |
| place | `levels`: hand ("1–4"); `chance`: hand (special levels); `part_of`: relation. Inhabitants, hazards and shops are generated from the other pages' `found_in`. |
| shop | `chance`: hand; `found_in`: relation; `shopkeeper`: a link. Its stock is generated from `relations.json`'s `shops`. |
| character | `role`: hand ("Rescuable", "Shop owner"); `health`: data, if it has one; `found_in`: relation |
| quirk (the `quirk` field) | `status`: the `docs/quirks.md` status (GML, OpenGMK, Oracle, GM8, DLL); `where`: the GML location; `pinned_by`: the test or replay; `entry`: the `docs/quirks.md` section |

When the infobox needs a number `stats.json` doesn't have, add a stat spec to `wiki/data/*.toml` in the same
PR ([wiki data](wiki-data.md#specs)) rather than writing it by hand.

## Numbers and units

Every number on a page comes from `stats.json` (written as `<Stat>`) or from a line of code the page
cites. Never from memory, another wiki or a video.

- **Units.** Health is "hit points" (a monster's) or "hearts" (the player's). Speeds are pixels per step
  with the per-second value after them: "1 pixel per step (30 per second)". Times are steps with seconds
  after them: "20–50 steps (0.67–1.67 seconds)". Money is dollars with thousands separators: "$15,000".
  Distances are pixels; a block is 16 pixels.
- **Ranges** use an en dash: "levels 1–4", "20–50 steps". `rand(a, b)` is every whole number from a to b;
  `random(n)` is a real number from 0 up to n, so write "less than n".
- **Odds** stay as the code gives them, with their scope: "a 1 in 60 chance for each block", "1 in 100 each
  step". Don't turn them into a percentage per level: that depends on how many blocks qualify, which the code
  doesn't fix. An average is fine if the arithmetic is shown and holds ("about once every 100 steps").
- **Decimals.** Write the value the code writes (`0.6`, not 0.6000000000000001; [wiki data](wiki-data.md)).
- **Tables** for anything with more than two values: a stat by level or by area, damage by source. Each row
  cites its line.

## Citing the code

Cite the original GML, not the port: the port is generated from it, and only the original is what
Spelunky Classic does.

- **A claim about behaviour** gets `<Cite gml="objects/oSnake/step.gml:47-54" />` after the sentence, with
  the lines that do it. Paths are relative to the `gml/` tree (as `docs/quirks.md` writes them); drop the
  `gml/` that `stats.json`'s `at.file` starts with. One citation per claim; a paragraph about one block of
  code cites it once.
- **Inherited code** is cited where it is: the Snake's bite is `objects/oEnemy/collision_oCharacter.gml`,
  since `oSnake` inherits it. Say so in the prose when it matters ("as for most monsters").
- **Numbers** written as `<Stat>` cite themselves. A number you type cites its line.
- **A citation renders** as a numbered note linking to the lines. There's no viewer for the GML on the site
  yet (851-3382); until there is, the note shows the path, the lines and the excerpt.
- **Excerpts** (a ```` ```gml ```` fence): only where the code itself is the explanation, as in a Why section or a Trivia
  item about a comment: a few lines (ten at most), quoted exactly, with the caption the
  [license](wiki-license.md#notice-text-for-the-templates) requires (the component adds it). Keep any credit
  comment in the lines you quote, and name the author when quoting Martin Piecyk's, ChevyRay's or tsg1zzn's
  scripts. Most pages need none.
- **Clips and replays** are cited as evidence of what happens in play: `<Cite clip="snake/stomp" />`, or
  `<Cite replay="fidelity/replays/action-throw-rock-snake.txt" />` for a committed replay whose story shows
  it.
- **Runner behaviour** (event order, collisions, RNG) isn't in the GML: cite the `docs/quirks.md` entry
  (`<Cite quirk="1" />` with its section) or a replay that shows it.

## Clips and images

All images and clips come from the [wiki media](wiki-media.md): sprites exported from the game's assets, and
clips recorded from the port playing a spec. Nothing else: no screenshots from elsewhere, no fan art, no
Spelunky HD or 2 art, no Spelunky logo.

**Clips.** List the page's clips in `clips`, the main one first (the infobox's, and the first VideoObject);
place them in the body with `<Clip id>` next to the text they show, and the rest go in the Gallery. Each
section that describes a move should place its clip.

- The caption under every clip is the license's, from the clip index (it names the port release); never
  write your own or leave it off.
- The clip's title and description come from its spec (`wiki/clips/<entity>/<move>.toml`), and are its
  accessible name and description: present tense, what's visible, one or two sentences ("The player drops
  onto a snake from the ledge above and kills it by landing on it"). If a page needs them to say something
  else, change the spec.
- The prose next to a clip says what it shows, so the page reads the same without video (clips are silent,
  so they need no captions or transcript, and the layout pauses them for `prefers-reduced-motion`).
- A clip whose spec's story doesn't assert what the page says it shows isn't evidence. Add the story fact to
  the spec (`scripts/wiki-clips --check`), or a new clip (851-3344's format).

**Sprites.** The infobox image is the thing's own sprite (`sprite` in the sprite index's `objects`), animated
when it moves. Body sprites illustrate a state or an item next to its name. Always integer scales,
nearest-neighbour (the `@4x` files), never smoothed or stretched.

**Alt text.** Describe what the image shows, briefly, for someone who can't see it: "A green snake with a
tan belly, coiled and facing left". Not "sprite of a snake" or the sprite's name. A sprite that's only an
icon next to its own name (a table row) gets `alt=""`. Information must never be in an image alone: a
table of sprites still names each one.

## Verify before publishing

A page makes claims the way a replay's story does ([formats: Stories](formats.md#stories)): each one is a
fact you can point at, and one you can't point at is cut. Before a page goes up:

1. **Read the code.** The object's events, then its parents' (an `action_inherited` or a missing event runs
   the parent's), then the scripts they call. Note the conditions around the lines you cite: `stats.json`'s
   `when` lists them, and they're often the interesting part (the Snake only moves near the screen).
   `scripts/wiki-data --find 'hp = ?' --in oSnake:create` finds a pattern, and `grep -rn oSnake assets/gml`
   finds everything that touches the object.
2. **Numbers** are `<Stat>`s or cited lines (above).
3. **"Always", "never" and "only"** need every source checked: `relations.json`'s `spawns_in`, `placed_in`
   and `drops`, the level editor (`scrCreateTileObj`), and code elsewhere that makes or changes the thing.
   The Snake isn't "only in the Mines": generation makes it nowhere else, but `drops` has Jars (in every
   area) and Olmec's yellow balls making Snakes, and `placed_in` the Tutorial. So its page says level
   generation makes Snakes only in the Mines, and lists the rest.
4. **What happens in play**, as opposed to what a line says, is shown in the engine: a clip whose story
   asserts it, or a replay whose story does (`replay-story` prints what a replay does, `--watch` adds the
   variables you name). This is for anything that depends on several pieces of code meeting, or on the
   runner (event order, collisions, timing). If it can't be shown, cut it or reduce it to what the code
   says.
5. **Things that look like bugs** are what Spelunky Classic does: describe them as behaviour, and link the
   quirk page when there is one. Don't call the port's behaviour different from the original's: it isn't
   (that's the project's whole rule), except behind off-by-default options, which only quirk pages mention.
6. **Clips** pass `scripts/wiki-clips --check` on the current main.
7. **Set `verified`** to the port's latest release (`git describe --tags --match 'v[0-9]*' --abbrev=0`) and
   the date.

Anything you can't verify and think is true becomes a Linear ticket (a clip to record, a stat spec to add),
not a hedged sentence.

## FAQ blocks

Three to five questions people actually search for about the thing, answered from the page. They become
the FAQPage JSON-LD and the Markdown twin's FAQ (851-3339).

- **Questions** in the searcher's words, naming the thing and, where it's natural, the game: "How much
  damage does a Snake do in Spelunky Classic?", "Where do you find the Jetpack?", "How do you get into the
  City of Gold?". Good sources: the infobox (damage, price, location), how to get or beat it, what it does
  to the player, and the misconceptions other games cause ("Is the Snake poisonous?" when Classic has no
  poison).
- **Answers** start with the answer ("1 heart.", "No.", "In the Black Market, for $50,000.") then add one or
  two sentences of context. Each stands alone: no "see above", no pronoun without its noun. Links and
  `<Stat>`s are fine.
- No question the page can't answer, and no answer that the body doesn't also say (the FAQ summarizes, it
  doesn't add).

## Links and navboxes

- **Link the first mention** of every thing that has a page, in the summary and then once per section at
  most; never the page itself, never twice in a sentence. Infoboxes and tables link every entry.
- `[[slug]]` shows the target's title; `[[slug|text]]` other text, for plurals and grammar: `[[bat|Bats]]`.
  `[[slug#anchor]]` links a section (`[[stomping#spike-shoes]]`). In a table cell, escape the pipe:
  `[[whip\|Whip]]`.
- **A link must resolve:** the build fails on a slug with no page (851-3334). Link only to pages that exist
  or that the same PR adds. When you add a page, search the existing pages for unlinked mentions of its
  name and link the first ones, in the same PR.
- **Relations, not prose lists.** "Damaged by" and "found in" go in `relations`, which render the infobox,
  the inverse lists and "What links here"; the prose explains them.
- **Navboxes** are generated: a page appears in the navbox of each of its categories, grouped by area where
  the type has one (Monsters: the Mines, the Jungle...). There's no navbox list to maintain; getting
  `categories` and `found_in` right is enough.
- **External links** only to the license and the project's own pages. Never to other wikis.

## Spoilers

Spoilers aren't hidden. An encyclopedia that hides its answers is useless to readers and to answer
engines, and collapsed text is harder to use with a screen reader. Instead:

- A page about a secret (the City of Gold, the Udjat Eye's room, the Hedjet, the Moai, Kali's rewards, the
  shortcuts, the ending) sets `spoiler: true`, and the layout shows a one-line note above the lede ("This
  page describes a secret"). Its lede still answers the question.
- Other pages don't give a secret's steps away: they name the secret and link it (`see [[city-of-gold]]`)
  rather than spelling out how to reach it, so a reader chooses to follow it.
- Secrets get their own sections where a type has them (a place's Secrets).

## Review checklist

Before approving a page (and before opening its PR):

- [ ] Frontmatter complete for the type, as its template has it: `slug` (= the file name and the data
      spec's slug), `title`, `type`, `categories`, `objects` (not on places, shops or guides), `lede`,
      `relations`, `clips`, `faq` (not on guides), `sources`, `verified`.
- [ ] The lede is one sentence of 160 characters or fewer, starts with the title, names Spelunky Classic,
      and answers "what is it".
- [ ] Sections are the type's, in its order, with its headings; no generated section written by hand.
- [ ] Every number is a `<Stat>` or cites its line; units and ranges as above; odds not turned into
      percentages.
- [ ] Every behaviour claim cites the GML (inherited code where it is); "always/never/only" checked against
      every source; play outcomes shown by a clip's or replay's story.
- [ ] Nothing from memory, other wikis, or Spelunky HD/2; no opinion or speculation; present tense; no
      "you" outside guides.
- [ ] Every clip's spec story asserts what the page says it shows, and `scripts/wiki-clips --check` passes;
      the prose says what each clip shows.
- [ ] Images are the game's sprites at integer scales, with alt text that describes them.
- [ ] Excerpts are short and captioned; no music; nothing the [license](wiki-license.md) rules out.
- [ ] Links: first mentions linked, every `[[slug]]` resolves, the new page's name linked from existing
      pages.
- [ ] FAQ: three to five real questions, answers that start with the answer and match the body.
- [ ] Spoilers: `spoiler: true` on secrets; other pages link rather than reveal.
- [ ] `verified` set to the current release and date.
