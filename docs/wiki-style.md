# Wiki style guide

How to write a page for the spelunky.ai wiki (851-3329): what a page says, in what order, in what voice, and
how it proves it. It's for the content tickets (about 190 pages), so it's a set of rules more than a
discussion. Each page type has a skeleton in [`wiki/templates/`](../wiki/templates), and
[`site/content/wiki/snake.mdx`](../site/content/wiki/snake.mdx) is a finished page to copy from, as are the
other Mines monsters the pilot wrote with it (851-3347: `bat`, `spider`, `giant-spider`, `caveman`,
`skeleton`). For an item, copy [`spectacles.mdx`](../site/content/wiki/spectacles.mdx) and the shop
equipment written with it (851-3351); for a weapon, [`whip.mdx`](../site/content/wiki/whip.mdx) and the
weapons written with it (851-3352); for a carryable, [`rock.mdx`](../site/content/wiki/rock.mdx),
[`jar.mdx`](../site/content/wiki/jar.mdx) (contents) and the carryables written with them (851-3353); for
treasure, [`gold-bar.mdx`](../site/content/wiki/gold-bar.mdx) and
[`golden-idol.mdx`](../site/content/wiki/golden-idol.mdx), with the gold written with them (851-3474); for a trap,
[`arrow-trap.mdx`](../site/content/wiki/arrow-trap.mdx) and the traps written with it (851-3354); for a place,
[`mines.mdx`](../site/content/wiki/mines.mdx) and the other areas written with it (851-3355).

The wiki's other rules live beside this one: what a page may show and the notices it carries
([wiki license](wiki-license.md)), where sprites and clips come from ([wiki media](wiki-media.md)), and where
numbers and relations come from ([wiki data](wiki-data.md)).

## The page model

A page is `site/content/wiki/<slug>.mdx`: YAML frontmatter, then an MDX body, served at `/wiki/<slug>`. The
build validates the frontmatter, resolves every stat, citation, clip, link and relation, and fails on
anything it can't ([The pipeline](#the-pipeline)). A link to a page that isn't written yet is a
[red link](#red-links), if the wiki plans that page.
`wiki/templates/` isn't built.

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

Relations shown in the infobox (`found_in`, `sold_in`, `drops`, `part_of`) come from `relations`, not from `infobox`.

### Relations

`relations` maps a relation to a list of slugs. Each entry is a slug, which the build checks against
`relations.json` for one of the page's `objects` (counting inherited events, its `runs_as`), or
`{ slug, cite, note }` for one the data can't see, such as a mechanic or a thrown item. The build fails on a
plain slug the data doesn't back, and lists (as a warning) what the data has that the page leaves out, so a
page curates its relations but can't contradict the code. A page without `objects` (a place, a shop) has
nothing to check a plain slug against, so its entries take a `cite`. `related` is the editors' own and
needs no backing.

| Relation | On | Points to | Inverse (generated) |
| --- | --- | --- | --- |
| `found_in` | monster, item, treasure, trap, character, shop | places | "Found here" on the place |
| `part_of` | place (a special level, a room) | places (its area) | "Contains" |
| `sold_in` | item | shops | "Sells" on the shop |
| `drops` | monster, item (containers), trap | items, treasure, monsters (a Jar's Snake) | "Dropped by" |
| `damaged_by` | monster, character | anything | "Damages" |
| `related` | any | anything | "See also" on both |

Write a relation on one side only: the side the data keys it by (a monster's `found_in`, an item's
`sold_in`). The inverse appears by itself, on the target's page once it's written. The infobox shows the
relations its type shows (`INFOBOX_RELATIONS`: a monster's Found in and Drops, an item's Sold in and Found
in...) and the inverses of what the thing has (Dropped by, Sells, Contains); See also shows the rest
(Damaged by, Damages, Found here, Related).

How the data backs a plain slug: the target's `objects` (or, for a place, its `rooms` or its area or level
slug; for a shop, its slug) must be what the page's objects are found in, sold in, drop, or are damaged by
in `relations.json` (docs/wiki-data.md, "The site's snapshot"). It counts inheritance as the game does:
code a parent object runs counts for the children that run it (the Snake runs oEnemy's step, so its spikes
and lava hurt the Snake; the Bat has its own step, so they don't), and a collision with an object is one
with its children (a Machete's slash is a kind of whip). What a page leaves out counts only what the data
says of its objects by name, or every monster page would be told it leaves out every item (any thrown item
hurts monsters) and what oEnemy's code makes (Kali's gifts, for a body on her altar).

### Body components

| Component | Use |
| --- | --- |
| `[[slug]]`, `[[slug\|text]]`, `[[slug#anchor]]` | A wiki link ([Links](#links-and-navboxes)). |
| `<Stat id="snake.hp" />` | A number from `stats.json`, with its unit (`1 hit point`). `id="oSnake.hp"` reads an object's create-event value. `as="seconds"` turns steps into seconds and `as="per-second"` px/step into px/s (30 steps a second, `room_speed`; two decimals at most); `bare` leaves the unit off. A table stat is read a row at a time: `<Stat id="spectacles.price-by-level" at="5" />` is its row for 5 (`$10,400`), and the build fails on a table without `at` or a row it hasn't got. |
| `<Cite gml="objects/oSnake/step.gml:66-71" />` | A citation: a numbered note linking to the lines ([Citing](#citing-the-code)). Also `replay=`, `clip=`, `quirk=` and `room=` (what the room editor placed in a room, `relations.json`'s `placed_in`). |
| ```` ```gml objects/oSnake/step.gml:73-76 ```` | A short GML excerpt: a fenced code block whose info string is `gml` and the lines' citation. The build adds the license's caption; `scripts/wiki-cites` checks the lines hold the text. A fence, not a component, because MDX would read the code's `{` and `<` as JavaScript and JSX. |
| `<Clip id="snake/bite" />` | A clip, with its title and an accessible description from the spec. |
| `<Sprite name="sSnakeWalkL" anim alt="..." />` | A sprite, still or animated, at an integer scale. |

The build generates the infobox, the hatnote, the lede, the table of contents, the FAQ, the Gallery (clips
the body doesn't place), See also, References, What links here and the navboxes. Don't write their
headings.

### The pipeline

The site (`site/`, docs/web-deploy.md) builds the pages; nothing else does.

- **Where pages live.** `site/content/wiki/<slug>.mdx`, under `site/` rather than the `wiki/pages/` this guide
  first proposed, so that a page's change is a change to the site: it gets a preview, and Workers Builds
  rebuilds the site for it. The numbers come from `site/content/wiki-data/stats.json`, a snapshot of
  `scripts/wiki-data`'s `stats.json` that `scripts/wiki-data` writes and `scripts/check` keeps current
  (docs/wiki-data.md, "The site's snapshot"), since the site's build has no Rust and no extracted assets. The
  clips' titles and descriptions come from their specs in `wiki/clips/`, and their files and the sprites from
  the pinned wiki media (docs/wiki-media.md), which `site/build.sh` unpacks first. The relations are checked
  against `site/content/wiki-data/relations.json`, the snapshot of `relations.json` beside the stats', which
  also names the places and shops that haven't a page yet; the areas the navboxes group pages by, in order,
  come from `wiki/data/relations.toml` (docs/wiki-data.md, "Relations").
- **The schema.** `site/src/wiki/build/schema.ts` is the frontmatter above as a zod schema, with each type's
  rules (an infobox's fields, which relations go on which type, FAQ or not). Every field above is in it as
  named.
- **The build.** `site/src/wiki/build/compile.ts` reads every page, validates it, and compiles its body and
  its inline MDX (the lede, hatnote, infobox texts and FAQ answers), resolving each `<Stat>` to its text,
  numbering the `<Cite>`s, and checking each `<Clip>` is in `clips` and placed once. `relations.ts` resolves
  the links and relations: what each slug names (a page, or a [red link](#red-links)'s planned page), each
  relation checked against `site/content/wiki-data/relations.json` (wiki-data's relations, snapshotted as the
  stats are), the inverses, and What links here. `plugin.ts`, a Vite
  plugin, serves the results as modules, one per page, and fails the build with every error at once:
  frontmatter that doesn't fit the schema, a slug that isn't the file's name, is a hub's or `media`, or is
  another page's slug or alias, an object on two pages, a stat `stats.json` hasn't got (or one that's text, or
  that the create event changes again under a condition), a clip without a spec, a component that isn't one
  of the above, code (an `{expression}` other than a comment, an attribute that's one, an event handler, a
  `<script>`), a heading the layout generates, one without an anchor (no letters or digits), two with the
  same anchor, or an anchor or HTML `id` of the layout's (`toc`, `content`, or starting `cite-`, `clip-`,
  `navbox-`, `toc-` or `infobox-`), a lede over 160 characters, a link or relation to a slug that's
  neither a page nor planned, a page linking itself, an anchor the target doesn't show, a relation to the
  wrong type of page, listed twice, or plain where the data doesn't back it, or a planned page that's
  written.
  A clip whose spec the pinned media don't have yet (a new one, until a release renders it) is only a
  warning, and shows as a placeholder with its title and description.
- **The routes.** `/wiki/$slug` (`site/src/routes/wiki/$slug.tsx`) pre-renders every page at its flat URL,
  `/wiki/<slug>` with no trailing slash, with the license's footer (docs/wiki-license.md), its
  head from `seo()` (the title "<Title> – Spelunky Classic fan wiki", the lede as the description, Article and
  BreadcrumbList JSON-LD, its FAQ as FAQPage JSON-LD, and a `<link rel="alternate" type="text/markdown">` to
  its Markdown twin), and an entry in `/sitemap.xml` dated by `verified`. The FAQ shows as a section of
  questions and answers after the body; the FAQPage has the same answers as plain text, which
  `site/test-seo.mjs` checks against what the page shows. A route's loader and head import no component:
  the route tree is in every page's entry chunk, the landing page's too, and only the routes' components are
  split into chunks of their own, so a component imported there would bring the wiki's code and Base UI to
  `/` (`site/test-bundle.mjs` checks it doesn't).
- **The hubs** (`site/src/wiki/hub.tsx`, their loaders' data and heads in `hub-route.ts`, their contents
  from `build/hubs.ts`; the list in `build/categories.ts`), Fandom's category pages, each pre-rendered with
  BreadcrumbList and ItemList JSON-LD.
  A hub is in the sitemap once one of its pages is written; until then it's `noindex`, as there's nothing on
  it to read:
  - `/wiki`: the search box (Search, below), the featured pages (the most recently verified),
    every category with how much of it is written, and how the wiki is made.
  - A hub per category, at `/wiki/<category>` (`/wiki/monsters`, `/wiki/items/weapons`), and `/wiki/items`,
    the three items' categories together, which is the breadcrumbs' step above each. A grid of cards, by
    title ("The Mines" under M), each the page's image sprite at a whole number of screen pixels a game
    pixel. Filters by name, by area (a page's `found_in`'s, or where generation makes a page to come's
    objects), by kind (`/wiki/items`), and by whether they're written (Base UI's input and toggle groups);
    pre-rendered, it shows everything, and the filters work once the page has hydrated.
  - **The whole scope.** A hub lists, dimmed and not as links, every page to come that a link may point at
    ([Red links](#red-links)): wiki-data's entities (in the hub of their `kind`, an `item` under Carryables
    until its page says otherwise), areas, special levels and shops, and `wiki-planned.yaml`'s pages (by
    their `type`; one without a type isn't listed), each with its object's sprite. So a hub shows the
    wiki's scope and what's left to write, and adding an entity to the specs or a planned page adds its
    card; writing the page turns the card into a link.
- **The layout** (`site/src/wiki/layout.tsx`, `wiki.css`), Fandom's shape in the landing page's pixel look:
  - **Breadcrumbs** above the title, from `src/wiki/crumbs.ts`, which the BreadcrumbList JSON-LD uses too,
    so the two can't differ: Wiki, the page's first category's hub (Items first for an item's), the page.
  - **The verified line** under the title: "Verified against the spelunky.ai port v0.1.7 on 27 October
    2026", the version linking to its release.
  - **The infobox**, floated beside the lede (after it, full width, on a phone): the title, the image
    sprite at twice its `@4x` file when it fits (a still frame for `prefers-reduced-motion`), the type's
    fields, each data value citing its lines as a numbered note like a `text`'s `cite`, the `objects`, and
    the Relations: those the type shows (`found_in`, `drops`, `sold_in` or `part_of`) and the inverses of
    what the thing has (Dropped by, Sells, Contains). The infobox's notes are numbered before the body's.
  - **The table of contents**, after the summary and before the first heading, when there are three
    sections or more: the body's `##` and `###` headings and the generated sections, each linking its
    stable anchor. On a phone it's a closed "Contents" disclosure (Base UI's Collapsible). On a wide screen
    (1200 pixels and up) it's a rail left of the article instead (`src/wiki/contents.tsx`), as Grokipedia's:
    it stays on screen as the page scrolls, and marks the section being read (`aria-current`), which an
    IntersectionObserver follows. Every heading shows a link to its anchor on hover and focus.
  - **The width**: the frame is up to 1400 pixels wide, but the prose and the tables stop at 75 characters,
    so on a wide screen the room goes to the rail, the infobox and the References, which are in two columns
    (read down the first, then the second) from 1000 pixels up. Below 1200 the frame stays 1000 wide.
  - **See also**, after the FAQ and the Gallery: the other relations and inverses (Damaged by, Damages,
    Found here, Related), and **What links here**, after the References ([Relations](#relations), [Links
    and navboxes](#links-and-navboxes)).
  - **Navboxes** at the foot, one per category, grouped by the `found_in` areas ([Links and
    navboxes](#links-and-navboxes)), each titled with a link to its hub.
  - **No layout shift:** every image and clip has its size in the HTML, the fonts are the system's, and the
    pre-rendered page hydrates without changing.
- **Search** (`site/src/wiki/search.ts`, `search-box.tsx`). The build makes an index of every page (its
  title, aliases, section headings, lede, categories, and its body's and FAQ's text) as its own chunk,
  `virtual:wiki/search-index`, which the browser loads the first time a search box is used and searches
  itself: no server. Every wiki page's header has a box that lists the best pages as you type (Base UI's
  autocomplete: the arrow keys, Enter and Escape work in it; its list's code, `search-popup.tsx`, is a chunk
  of its own, loaded with the index) and otherwise goes to `/wiki?q=<query>`, where `/wiki`'s own
  box lists every page that matches, with the sections whose headings do. That URL is the WebSite JSON-LD's
  SearchAction. A page's words score by where they are (title, then aliases, headings, lede, categories, then
  its text by how often), and every word of a query must match a word, the start of one, or one a letter or
  two shorter ("snakes" finds "snake"). It's a page of code rather than a library: Orama would add 22 KB
  gzipped to the page's code and MiniSearch 6 KB, where this is 1.2 KB; and Pagefind
  would mean a binary in Workers Builds, a pass over the built HTML that `vite dev` doesn't make, and
  hundreds of files toward the assets' limit (docs/web-deploy.md, "Limits") where this is one. The index has
  the pages that are written, not the planned ones (the Snake's alone is 1.5 KB gzipped).
- **Hover cards** (`site/src/wiki/hover-card.tsx`, their data from `build/cards.ts`): Base UI's preview
  card. Hovering a link to a written page (a `[[link]]`, a relation's, What links here's, a navbox's: a
  `CardLink`, the card's trigger, an `<a data-slug>`), focusing it
  from the keyboard, or long-pressing it on a touch screen shows a card of the page's infobox sprite
  (animated, its first frame for `prefers-reduced-motion`), title, type, lede and first three infobox
  numbers; a tap still follows the link, and a red link has none. The long press takes the place of the
  browser's own menu for a wiki link (open in a new tab, copy the link). The card is a tooltip (`role="tooltip"`,
  the link's `aria-describedby` while it shows) that stays while the pointer is on it and closes on Escape,
  placed under or over the link (by the line the pointer's on, when the link wraps) inside the viewport and
  out of the page's flow, so nothing moves. The long press and the tooltip's ARIA are the site's; the rest is
  Base UI's. The cards of every page are one chunk, `virtual:wiki/hover-cards`, which the browser loads the
  first time a link is hovered or focused (the Snake's card is 0.5 KB, and `site/test.sh` holds them to 0.7
  KB a page, so all 190 pages' stay well under 133 KB), rather than a chunk a page, whose loaders the pages'
  bundle would carry; the card's own code (Base UI's positioner and popup, `hover-card-popup.tsx`) is a
  chunk loaded with them.
- **Base UI** (`@base-ui/react`, CLAUDE.md): the search box, the hover cards and the hubs' filters are built
  on it, styled to the site's look, and it adds 41 KB gzipped to every page's code (126 KB to 167 KB): its
  combobox's state, which the header's box needs to hydrate, is 26 KB of that, and the hover cards'
  triggers, the toggle groups and the inputs most of the rest. The popups' code (positioning, the lists) is
  in chunks of their own, loaded with the data they show, which saves another 15 KB. The landing page
  carries it too, as it does all the wiki's code, until 851-3407.
- **Markdown and llms.txt** (`site/src/wiki/build/markdown.ts`), for answer engines and LLMs. Every written
  page has a Markdown twin at `/wiki/<slug>.md`: the title and where it's from, the hatnote, the lede, the
  infobox as a table, the body, the FAQ, the Gallery, See also and References (the citations as footnotes,
  the GML's paths and lines), but not the navigation (the table of contents, What links here, navboxes).
  It's made of the same trees as the page's HTML, after every `<Stat>`, `<Cite>`, `<Clip>` and `[[link]]` is
  resolved, so it can't say something else. Links go to the HTML pages, absolute: the page is the canonical
  URL, which an answer engine cites and a reader follows, and the twin's `Link: <...>; rel="canonical"`
  header (`site/src/server.ts`) says so, so search engines index the page rather than its twin; an agent
  finds the twins in `/llms.txt`. A red link is its text. `/llms.txt` (<https://llmstxt.org>) is the index: the
  site's title and summary, then each category's pages (a page in two is under both), each its twin and its
  lede; `/llms-full.txt` is every page once, in that order, their footnotes named for their pages. Read
  without the page's header and footer, each file starts with the footer's notice, its link absolute, and a
  twin of a page whose clips have sound (and `/llms-full.txt`, if any has) with the audio credit
  (docs/wiki-license.md; `site/src/wiki/notices.ts` has their text, which the footer uses too). The notice
  comes before llms.txt's title, which the convention puts first: the license comes first. The wiki's Vite
  plugin writes them into the static assets (no route: the Worker would carry every page's Markdown), and
  serves them under `vite dev`.
- **Checked outside the build.** The site's build checks a citation's form but can't check its lines or an
  excerpt's text, as it hasn't got the GML: `scripts/wiki-cites` does, in `scripts/check`
  ([Citing the code](#citing-the-code)).
- **Not built yet.** Clips play when clicked rather than on their own: a page has several.
- **Testing.** `site/test.sh` builds the site and checks `/wiki/snake`, the hubs, the search box, `/wiki?q=`
  and that the search index, the hover cards and the two popups' code are chunks of their own that no page
  loads up front, the
  Snake's Markdown twin, `/llms.txt` and `/llms-full.txt` (their types, caching and notices), and runs
  `site/test-wiki.mjs`, which compiles the committed pages (and checks the hubs' cards and their Markdown), a
  set of pages that link and relate to each other (inverses, What links here, their twins' links and
  `/llms.txt`), and a set of broken ones, each of which must fail the build with its error, searches the index
  of the committed pages, and checks their hover cards.

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
`snake`. That includes a form that turns into the monster, even if the code doesn't make it a monster: the
rising pile of bones `oFakeBones` is the Skeleton's, beside `oSkeleton`. Listing it also gives the page its
`found_in`, since generation makes the waiting form and the monster only comes out of it (`relations.json`
has `oSkeleton` in no place). Things the game names differently get different pages: `spider` and `giant-spider`, `bat` and
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
  when there's more than one case. A weapon's is by monster: the code most monsters share first, then each
  monster whose own code differs, a column for each part of a use (a swing's crack and its wind-up behind
  the player), agreeing with the monster pages' Defeating tables. A trap's is by victim (the player, most
  monsters, those it treats apart), with how often it hits (once, or every step, as nothing spaces out a
  trap's hits) and the end-of-game statistics row a death counts in.
- **Defeating** (monster): every way it takes damage or dies, as a table: way, damage, citation. Then
  what the kill counts for (kill tallies, Kali's favour). Check whether its step event runs the parent's
  (`action_inherited()` at the top): most monsters' Spikes, Lava, Spear Traps and water are in oEnemy's
  step, so a monster whose own step replaces it (the Bat, a hanging Spider, the Giant Spider) isn't hurt by
  them, and the page says so, as readers ask. Say what limits the damage in the Damage column: a cooldown
  ("1, at most once every 10 steps"), a stun, a hit that does nothing while it's stunned.
- **Locations**: where generation makes it and how likely, from `relations.json`; the level editor's tile
  code if it has one.
- **Tips**: practical consequences of the verified facts ("A Snake never walks off a ledge, so one on a
  platform stays on it"). Every tip must follow from something on the page. No second person.
- **Trivia**: oddities in the code (unused states and sprites, copied `type` strings, comments), each
  cited. Bugs the game keeps link to their quirk page.
- **Effect** (equipment): first how it goes on (picked up free, paid for or carried out of a shop, which
  `scrStealItem` turns into its global, its message and its icon) and that it stays on until a new game;
  then everything the code that checks its global changes, a `###` an effect when there are several (the
  Pitcher's Mitt's Throwing and Catching). What only shows in play, a jump's height or a throw's distance,
  is measured in a trace of a scene and cites the scene's clip.
- **Obtaining** / **Stock** / **Prices**: from `relations.json`'s shops and `stats.json`'s price tables;
  show the price-by-level table rather than describing it, an area a row and a level a column, each cell
  a `<Stat at>`, with a dash where there are no shops (level 1, and level 16, Olmec's Lair). An item's
  Obtaining starts with a table of its sources (source, chance, citation): its shops, a Dice House prize,
  Crates, blocks that hide it, Kali, the monsters that leave one. It ends with stealing, which links
  `docs/quirks.md` §1 and cites the item's theft replay where there is one (`fidelity/replays/item-seed-*`).
  `spectacles.mdx` is the model.
- **Generation** (place): which levels, how the player gets there (with a clip), how it's chosen and with
  what odds, how its rooms are laid out. **Inhabitants** (place): a table of what generation makes there
  (monster, where, chance, citation), checked against `relations.json`'s `spawns_in` for the place and the
  monsters' pages' `found_in`, so the page, its generated "Found here" and the hubs' area filters agree.
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
PR ([wiki data](wiki-data.md#specs)) rather than writing it by hand (the Giant Spider's `jump-speed`). A
stat's `note` is written before its page, so check it against the code too: the Spider's jump range
note said it only hops within the range, and it hops toward the player from anywhere, only less often.

A monster with several speeds shows the one that matters to the player in `speed`: the Caveman's charge,
the Spider's jump across. The `image` is also the page's card on the hubs, so pick the sprite of the thing
in action, not at rest (the flying `sBatLeft`, not the object's own `sBatHang`; 851-3398).

## Numbers and units

Every number on a page comes from `stats.json` (written as `<Stat>`) or from a line of code the page
cites. Never from memory, another wiki or a video.

- **Units.** Health is "hit points" (a monster's) or "hearts" (the player's). Speeds are pixels per step
  with the per-second value after them: "1 pixel per step (30 per second)". A speed is the one the code sets, which
  isn't always what the monster covers: `moveTo` moves whole pixels (a fraction becomes 1 more pixel every
  so many steps, on each axis), and friction applied after the speed is set changes what it moves by. The
  Caveman's walk is 1.5 in its code and 4 pixels every 3 steps in play; a Bat's 1 is about 1.4 on a
  diagonal. Check a fractional or diagonal speed in a trace (`replay-run REPLAY OUT.jsonl` has every
  instance's `x` and `y` each frame) and say what it comes to. Times are steps with seconds
  after them: "20–50 steps (0.67–1.67 seconds)". Money is dollars with thousands separators: "$15,000".
  Distances are pixels; a block is 16 pixels.
- **Ranges** use an en dash: "levels 1–4", "20–50 steps". `rand(a, b)` is every whole number from a to b;
  `random(n)` is a real number from 0 up to n, so write "less than n".
- **Odds** stay as the code gives them, with their scope: "a 1 in 60 chance for each block", "1 in 100 each
  step". A roll in an `else if` chain only happens when the ones before it fail, so say so: "1 in 90, once
  the rolls before it, from a Jetpack's 1 in 500 to a Teleporter's 1 in 100, fail" (a Crate's). Don't turn
  them into a percentage per level: that depends on how many blocks qualify, which the code doesn't fix. An
  average is fine if the arithmetic is shown and holds ("about once every 100 steps").
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
- **Never wrap a line so that it holds only `<Cite>`s or `<Stat>`s.** MDX makes a line of nothing but
  components a block of its own, so it ends the paragraph: the notes stand alone and the rest of the
  sentence starts a new paragraph. Move them up to the end of the line before (a tag may break inside, `<Cite`
  then `gml="..." />` on the next line), or start the line with a word. The build fails on it (851-3408).
- **A citation renders** as a numbered note linking to the lines. There's no viewer for the GML on the site
  yet (851-3382); until there is, the note shows the path, the lines and the excerpt.
- **Excerpts** (a ```` ```gml ```` fence): only where the code itself is the explanation, as in a Why section or a Trivia
  item about a comment: a few lines (ten at most), quoted exactly, with the caption the
  [license](wiki-license.md#notice-text-for-the-templates) requires (the component adds it). Keep any credit
  comment in the lines you quote, and name the author when quoting Martin Piecyk's, ChevyRay's or tsg1zzn's
  scripts. Most pages need none.
- **Checked.** `scripts/wiki-cites` (in `scripts/check`, so on every pull request) reads every page and
  fails on a path that isn't in the `gml/` tree (in its exact case), lines past a file's end, or an excerpt
  that isn't exactly the lines it cites, its whitespace aside (an excerpt may indent with spaces what the
  GML indents with tabs). It finds paths by their shape (`objects/...gml`, `scripts/...gml`, then any
  `:N` or `:N-M`) wherever they are: `<Cite gml>`, the infobox's and relations' `cite`, `sources`, a
  quirk's `where`, an excerpt's info string, comments and prose. An excerpt's fence opens on a line of
  its own, indented as its list item is if it's in one; one in a quote or on a list item's first line
  fails, as the check can't read it, and so does a code block whose fence never closes. Its errors name the page and line, the citation, and what's there
  instead: the file's lines, the files beside a missing one, or the lines an excerpt should quote.
- **Clips and replays** are cited as evidence of what happens in play: `<Cite clip="snake/stomp" />`, or
  `<Cite replay="fidelity/replays/action-throw-rock-snake.txt" />` for a committed replay whose story shows
  it. A page can only cite its own clips (those in its `clips`), so another page's clip (a quirk's) is
  cited by listing it, which also puts it in the Gallery, or not at all.
- **Runner behaviour** (event order, collisions, RNG) isn't in the GML: cite the `docs/quirks.md` entry
  (`<Cite quirk="1" />` with its section) or a replay that shows it.
- **A bug in the GML** the page describes (a test that's always true, a `type` string never set) is
  behaviour: describe it, cite the lines, and quote them in Trivia if the code is the point. If
  `docs/quirks.md` doesn't have it, file a ticket to add it (and its quirk page) rather than leaving it
  out; link the quirk page once it exists.

## Clips and images

All images and clips come from the [wiki media](wiki-media.md): sprites exported from the game's assets, and
clips recorded from the port playing a spec. Nothing else: no screenshots from elsewhere, no fan art, no
Spelunky HD or 2 art. (The Spelunky logo belongs to the site's masthead, not to pages.)

**Clips.** List the page's clips in `clips`, the main one first (the infobox's, and the first VideoObject);
place them in the body with `<Clip id>` next to the text they show, and the rest go in the Gallery. Each
section that describes a move should place its clip.

- The clip's title and description come from its spec (`wiki/clips/<entity>/<move>.toml`), and are its
  accessible name and description: present tense, what's visible, one or two sentences ("The player drops
  onto a snake from the ledge above and kills it by landing on it"). If a page needs them to say something
  else, change the spec.
- The prose next to a clip says what it shows, so the page reads the same without video or sound (clips
  have no speech, only the game's sound effects and music, so they need no captions or transcript, and the
  layout pauses them for `prefers-reduced-motion`).
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
   says. Lines that read one way can play another: the Caveman's walk sets it standing at a ledge's edge,
   then sets its walking speed again in the same step, so it slides off anyway; only a scene showed it
   ([wiki media](wiki-media.md#staging-a-scene) has how to stage one).
5. **Things that look like bugs** are what Spelunky Classic does: describe them as behaviour, and link the
   quirk page when there is one. Don't call the port's behaviour different from the original's: it isn't
   (that's the project's whole rule), except behind off-by-default options, which only quirk pages mention.
6. **Clips** pass `scripts/wiki-clips --check` on the current main.
7. **Citations** pass `scripts/wiki-cites` (it reads `assets/gml`, so after `scripts/extract`).
8. **Set `verified`** to the port's latest release (`git describe --tags --match 'v[0-9]*' --abbrev=0`) and
   the date.

Anything you can't verify and think is true becomes a Linear ticket (a clip to record, a stat spec to add),
not a hedged sentence.

## FAQ blocks

Three to five questions people actually search for about the thing, answered from the page. They become
the page's FAQ section, its FAQPage JSON-LD and its Markdown twin's FAQ.

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
- **A link must resolve:** to a page, or to one the wiki plans, which is a [red link](#red-links) until
  it's written. The build fails on any other slug. A thing neither wiki-data nor `wiki-planned.yaml` has
  (the Scarab, Kali, a Frog) stays plain text, unless the page needs it linked or related, when it goes on
  the planned list with the `objects` a relation to it is checked by (the Skeleton's `drops: [skull]` added
  `skull: { title: Skull, type: item, objects: [oSkull] }`). When you add a page, search the existing pages for
  unlinked mentions of its name and link the first ones, in the same PR.
- **`[[slug#anchor]]`** must name a heading (or a generated section: `faq`, `see-also`,
  `what-links-here`...) the target has, once it's written; and a page never links itself: a section of
  its own is `[text](#anchor)`.
- **Relations, not prose lists.** "Damaged by" and "found in" go in `relations`, which render the infobox,
  the inverse lists and "What links here"; the prose explains them.
- **Navboxes** are generated: a page appears in the navbox of each of its categories, grouped by area where
  the type has one (Monsters: the Mines, the Jungle...): a page `found_in` two areas is in both, and a
  category's pages in none come last, as Other. There's no navbox list to maintain; getting `categories`
  and `found_in` right is enough.
- **External links** only to the license and the project's own pages. Never to other wikis.
- **What links here**, at the foot of every page, lists the pages whose text (body, lede, hatnote, infobox
  texts, FAQ) or relations link to it.

### Red links

Most pages aren't written yet, and a page should still link the things it mentions, so that the links
are there when their pages arrive. Like Fandom and Wikipedia, the wiki shows a link to a page that doesn't
exist as a red link: its text in red, dotted, titled "This page hasn't been written yet", and not a link,
so readers and crawlers aren't sent to a 404. Unlike them, a red link must be to a page the wiki plans:

- `site/content/wiki-planned.yaml` lists the planned pages not written yet (`tutorial: { title: Tutorial,
  type: place, rooms: [rTutorial] }`), with the `type` a relation to them is checked by, and the `objects`
  and `rooms` a plain one is.
  wiki-data's slugs count as planned without being listed: its entities', areas', special levels' and
  shops' slugs are their pages' (docs/wiki-data.md).
- A link or relation to any other slug fails the build. Without that, a typo (`[[mine]]`) would be a red
  link for good, which is why Wikipedia has bots hunting them; here it's caught where it's made.
- The build fails on a planned slug that has a page, or that no page links to: the PR that writes a page
  (or drops the last link to one) takes it off the list, so the list is always the wiki's backlog of
  linked-but-unwritten pages.
- The build prints the red links, each unwritten page with the pages linking to it, most linked first: the
  wiki's "wanted pages".

Once every planned page is written, the list is empty, and a link to a missing page fails the build, as
the wiki's first plan had it.

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
- [ ] `scripts/wiki-cites` passes: every cited file and line exists and every excerpt quotes its lines.
- [ ] Excerpts are short and captioned; audio carries the audio credit; nothing the [license](wiki-license.md) rules out.
- [ ] Links: first mentions linked, every `[[slug]]` resolves, the new page's name linked from existing
      pages.
- [ ] FAQ: three to five real questions, answers that start with the answer and match the body.
- [ ] Spoilers: `spoiler: true` on secrets; other pages link rather than reveal.
- [ ] `verified` set to the current release and date.
