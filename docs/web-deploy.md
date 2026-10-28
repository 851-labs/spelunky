# Deploying spelunky.ai

<https://spelunky.ai> is a [TanStack Start](https://tanstack.com/start) app, deployed as a Cloudflare Worker
with static assets (account "851", `630f294bcb2c1e9b751d9fe0655a453a`; zone spelunky.ai,
`6661e9e5fb33bb71d0d23105fc615009`):

| Path | What | Caching |
| --- | --- | --- |
| `/` | the landing page (`site/src/routes/index.tsx`, pre-rendered) | `no-cache` |
| `/assets/` | the pages' scripts, named by a hash of their contents | `immutable`, a year |
| `/play/` | the game: a release's web build (`scripts/web --package`) | `no-cache` (the page and `manifest.txt`) |
| `/play/v/HASH/` | the build's wasm, JS and assets, `HASH` being of their contents | `immutable`, a year |
| `/LICENSE.txt` | the Spelunky User License, copied from the repo | `no-cache` |
| `/wiki`, `/wiki/<slug>` | the wiki: its hubs (`/wiki`, `/wiki/monsters`, `/wiki/items/weapons`...) and pages (`site/src/routes/wiki/`, from `site/content/wiki/`; docs/wiki-style.md), pre-rendered | `no-cache` |
| `/robots.txt`, `/sitemap.xml`, `/sitemap-video.xml`, `/sitemap-image.xml` | for crawlers (SEO, below): pre-rendered | `no-cache` |
| `/opensearch.xml` | the wiki's search for browsers, as an OpenSearch description (SEO, below): pre-rendered | `no-cache` |
| `/favicon.ico`, `/icon.svg`, `/apple-touch-icon.png` | the site's icons, the game's Spelunker (SEO, below) | `no-cache` |
| `/wiki/<slug>.md`, `/llms.txt`, `/llms-full.txt` | the wiki for answer engines and LLMs: each page's Markdown twin (`text/markdown`, its `Link` header naming the page as canonical), their index and all of them (`text/plain`), written by the build (docs/wiki-style.md, "Markdown and llms.txt") | `no-cache` |
| `/releases/latest.json` | the native apps' signed update manifest (`docs/updates.md`), the release's own | `no-cache` |
| `/wiki/media/HASH/` | the wiki's sprites and clips (`docs/wiki-media.md`), `HASH` being of their package; the clips' videos with byte ranges | `immutable`, a year |
| `/wiki/og/<slug>.HASH.png` | the wiki's Open Graph images, one a page and hub, `HASH` being of the file (SEO, "Open Graph images") | `immutable`, a year |

`www.spelunky.ai` redirects (301) to the apex (its pages: the content-hashed files under `/play/v/`,
`/assets/` and `/wiki/media/` skip the Worker, all but the clips, but only a page on www would ask for them
there), and a path that's none of these is a 404 with `site/public/404.html`. Everything is in `site/`:

- `src/`: the app. `routes/` has its pages (TanStack Router's file routes; `routeTree.gen.ts` is generated
  from them by the build and `vite dev`, and committed), and `seo/` their head tags, JSON-LD and the
  sitemap (SEO, below), and `wiki/` the wiki's components, layout and build (docs/wiki-style.md, "The
  pipeline"). Pre-rendering is on (`vite.config.ts`): every static route is rendered to HTML at
  build time and served as a static asset like the rest, so the Worker renders nothing on a page view.
  `server.ts` is the Worker: the www redirect, the static assets with the `no-cache` header, the wiki's clips
  with byte ranges ([Wiki media](#wiki-media)), and the router
  for server functions and for a page that isn't a file (neither exists yet, so on spelunky.ai the router
  only runs to confirm a 404).
- `content/`: the wiki's pages (`content/wiki/<slug>.mdx`), the pages it plans but hasn't written
  (`content/wiki-planned.yaml`, docs/wiki-style.md, "Red links"), and the snapshots of its stats and
  relations (`content/wiki-data/`, docs/wiki-data.md).
- `public/`: copied as is: the landing page's media, the site's icons (`scripts/site-icons`), `404.html`, and
  `_headers` (the immutable caching).
- `wrangler.jsonc`: the Worker and its domains. `vite.config.ts` builds the app with the Cloudflare Vite
  plugin, which writes the deployable Worker to `dist/server` (with its own `wrangler.json`), its static
  assets to `dist/client`, and `.wrangler/deploy/config.json`, which points `wrangler deploy` and
  `wrangler versions upload` at them.
- `package.json` and `package-lock.json` (every package pinned), and `.node-version` (the Node the build runs
  on, which Workers Builds reads). `package.json`'s `allowScripts` approves the install scripts of exactly the
  versions npm asked about (esbuild's and workerd's binaries, fsevents), so `npm ci` runs them without a
  warning. A bump that changes one of those versions brings the warning back (and, once npm blocks unapproved
  scripts by default, skips the script): re-approve it with `npm install-scripts approve <package>`
  (which moves its pin to the new version), and delete by hand any entry left for a version the lockfile no
  longer has: `npm install-scripts prune` off macOS would drop fsevents too, which only installs there.
- `build.sh` and `web-release.txt`: the build, below; `wiki-media.sh` and `wiki-media.txt`: the wiki's media
  ([Wiki media](#wiki-media)). `test.sh`: the smoke test (Testing locally).

The site never builds Rust. A GitHub Release carries the web build as `spelunky-web-TAG.tar.gz` (made by
`scripts/web --package`, attached by the release workflow), `site/web-release.txt` names the release and the
tarball's SHA-256, and `site/build.sh` downloads it, checks the checksum, builds the app (`npm ci`, `vite build`
and a type check), and unpacks the web build into its static assets at `/play/`, and the wiki's media at
`/wiki/media/HASH/`. We
create and store no Cloudflare API token: Cloudflare's Workers Builds (its Git integration) builds and
deploys with a build token of its own, which lives only in Cloudflare (the Worker's Settings → Build → "API
token", named like "Workers Builds - DATE"; the account's existing one is reused) and never in GitHub.

## Connecting the repo (once)

In the Cloudflare dashboard, account 851: **Workers & Pages → Create → Import a repository**, pick GitHub
`851-labs/spelunky`, and set:

| Setting | Value |
| --- | --- |
| Project (Worker) name | `spelunky` (must match `name` in `site/wrangler.jsonc`) |
| Production branch | `main` |
| Root directory (path) | `site` |
| Build command | `./build.sh` |
| Deploy command | `npx wrangler deploy` |
| Builds for non-production branches | on, with the non-production deploy command `npx wrangler versions upload` |
| Build watch paths | include `site/*`, exclude nothing |
| Environment variables | none |

The watch path's `*` spans directories, so a change anywhere under `site/` builds and nothing else does
(the game's code changes don't touch the site until a release is bumped in). `wrangler deploy` attaches
both custom domains and creates their DNS records and certificates; if spelunky.ai or www already has a DNS
record (a registrar's parking page, say), delete it first or the deploy fails.

**Previews.** A pull request whose branch changes something under `site/` builds too, but its deploy command
is `wrangler versions upload`: that uploads a version with its own preview URL (`preview_urls: true` in
`wrangler.jsonc`, which each deploy applies, so the Worker's Settings → Domains & Routes setting follows it) and never touches spelunky.ai, whose custom domains only `wrangler deploy`
on `main` attaches. Workers Builds posts the preview's link and build status on the pull request. A preview
serves the release `site/web-release.txt` names, so it shows changes to the site (the pages, the Worker,
headers, `build.sh`), not to the game: a pull request's browser build is checked by `pr.yml`'s
"Mac mini / Web build check".

Until a release has a web build, `site/web-release.txt` says `none` and `build.sh` fails with a message
saying so, so the first build after connecting fails, deploying nothing, until the first bump below lands.

## Putting a version live

1. The release workflow ([docs/ci.md](ci.md)) publishes a GitHub Release `TAG` with
   `spelunky-web-TAG.tar.gz`, made by `scripts/web --package`, and `latest.json`, the native apps' signed
   update manifest (`docs/updates.md`), and `wiki-media-TAG.tar.gz`, the wiki's media (`docs/wiki-media.md`).
2. `scripts/bump-web-release TAG` downloads them, checks the manifest's signature, and writes the tag and
   both SHA-256s into `site/web-release.txt`; and checks the wiki's media as the site will unpack them and
   writes the tag and their SHA-256 into `site/wiki-media.txt` (a release without them leaves it alone, and
   so do newer media from `wiki-media.yml`: `docs/wiki-media.md`, "Publishing").
3. Open a PR with that change; merging it to `main` builds and deploys the site (a minute or two). The web
   build at `/play/` and the native apps' update at `/releases/latest.json` go live together.

`https://spelunky.ai/play/manifest.txt` lists the SHA-256 of every file live at `/play/`, and the build log
shows which release it unpacked.

## Rolling back

- **Fast:** dashboard → Workers & Pages → `spelunky` → **Deployments** → an earlier version → **Rollback**.
  It's instant, and a version includes its assets, so this restores that version's game too. The next
  build from `main` deploys over it, so follow up with the PR below.
- **Through Git:** revert the bump of `site/web-release.txt` (or bump to the older tag) and merge.

A deploy serves only its own build, so a `/play/` page loaded just before it can fail to start (an HTTP 404
for its old `v/HASH/` files) and works on a reload, which names the new `v/HASH/`. A game already running has
fetched everything it needs. Those 404s come with `_headers`' immutable caching too (`/play/v/*` skips the
Worker, and `_headers` can't tell a miss from a hit), which only matters to that browser if it's rolled back
to exactly that old build; routing `/play/v/*` through the Worker to fix it would spend ~110 Worker requests
of the free plan's 100,000 a day on every game start. The same goes for the pages' scripts under `/assets/`.

## Wiki media

`site/wiki-media.txt` pins a package of the wiki's sprites and clips, a release asset like the web build
(`docs/wiki-media.md`, "Publishing"), and `site/build.sh` has `site/wiki-media.sh` download it, check its
SHA-256 and unpack it at `/wiki/media/HASH/`, `HASH` being the first 16 hex digits of the package's SHA-256.
`scripts/bump-web-release` moves both pins together, and between releases `wiki-media.yml` pushes a branch
moving the media's pin alone (`scripts/bump-wiki-media`), so a PR changing the pin is a deploy too, and
while it's `none` the site has no `/wiki/media/`. The media are unpacked before the app's build, which gets
`WIKI_MEDIA_DIR` (the unpacked package, for the pages to read `sprites/index.json` and `clips/index.json` as
they're pre-rendered), `WIKI_MEDIA_URL` (`/wiki/media/HASH`, to link the files) and `WIKI_MEDIA_DATE` (the
pin's `date`, when its release was published, or a `--wiki-media` package's modification time); none is set
while the pin is `none`, nor under `npm run dev`. The wiki's pipeline and `virtual:wiki-media` read them (SEO,
below).

**Caching.** A file's name doesn't say what's in it: `sSnakeWalkL/frames/0@4x.png` keeps its name when the
sprites are re-exported, and every clip's caption names the release that rendered it. So the directory names
the package, the files never change, and they're cached for good (`_headers`), like `/play/v/HASH/`. They
skip the Worker too (`run_worker_first`), all but the clips' videos (Byte ranges, below): a wiki page shows
dozens of sprites and a few clips, each a request, which through the Worker would come out of the free plan's
100,000 a day. A page names the current `HASH`, and the pages are `no-cache`, so a new pin shows up on the next
page view; a page loaded just before a deploy can miss its media until a reload, as `/play/` can (Rolling
back), and that miss is cached as immutable too.
The router never sees `/wiki/media/...`, and a wiki page mustn't be called `media` (`build.sh` fails if the
app pre-renders one).

**Escape `@` as `%40`.** The asset store answers `0@4x.png` with a 307 to `0%404x.png`, its canonical form, so
a page asks for that directly, saving a round trip per image (`encodeURIComponent` on each path segment).

**Byte ranges.** The asset store ignores `Range`: a request for `bytes=0-1` gets a 200 with the whole file,
on spelunky.ai as under `wrangler dev`. Current WebKit plays the clips anyway: in 851-3386's tests, WebKit 26
on macOS (Playwright's) and Safari on iOS 27's Simulator played every clip on `/wiki/snake`, MP4 and WebM,
straight from spelunky.ai, asking for `bytes=0-1` and taking the whole file. They're served with ranges all
the same because Apple's guide says a server must answer them for video on iOS ("Configuring Your Server",
Safari HTML5 Audio and Video Guide), and iOS 18 and earlier couldn't be tested (an iPhone without WebM plays
the MP4). Clips with audio (851-3395) make ranges matter more: they're bigger, and a player seeks in them by
range.

So the clips' videos, `clips/**.mp4` and `.webm`, and only they, come through the Worker (`src/server.ts`).
A satisfiable single range gets a 206 sliced from the asset (`Content-Range`, `Content-Length`,
`Accept-Ranges`, and the asset's own headers: the immutable caching, type and ETag); anything else (no
`Range`, one past the end, several, a malformed one, an If-Range that isn't the ETag) the plain 200 with the
whole file that a server may always send; and a HEAD the `Content-Length` the asset store leaves out. The
sprites, posters and indexes never reach it: negative `run_worker_first` rules beat positive ones and `*`
spans directories, so the rules keep the rest of the package off the Worker by extension (`.png`, `.json`,
`.txt`), and `wiki-media.sh` fails on a package with a file of another kind, or a video outside `clips/`,
which would come through it.

**The cost.** A clip costs Worker requests only when it's played: the pages' `<video>`s have
`preload="none"`. Measured on a preview, a play is one request in Chrome (`bytes=0-`) and in a Safari that
plays the WebM (no `Range`), and two in a Safari that plays the MP4 (`bytes=0-1`, then the rest); a loop
replays from memory, and the immutable caching can spare a replay's requests. So the free plan's 100,000 a
day is about 50,000 clip plays, less the page views. Seeking in a clip that isn't all in yet (with audio, a
bigger one) is a request more each time. Each of these reads the whole clip from the asset store, which
costs no request: the binding's answer has no `Content-Length`, so that's how the Worker learns the size.

Serving the clips from an R2 bucket with a custom domain, which answers ranges itself, would keep them off
the Worker, but needs a bucket and its domain set up in the dashboard and the clips uploaded with a token on
each bump, outside the deploy: a version's preview and rollback wouldn't bring its clips, as they bring the
rest of its media. The Worker needs nothing outside `wrangler.jsonc`.

## Testing locally

```sh
site/test.sh                                        # the smoke test, below
scripts/web --package /tmp/spelunky-web.tar.gz      # a release build, plus /tmp/spelunky-web.tar.gz.sha256
site/build.sh --local /tmp/spelunky-web.tar.gz      # site/dist, checked against the .sha256 beside it
# (add --manifest latest.json to serve an update manifest at /releases/latest.json too, and
# --wiki-media target/wiki-media-v1.tar.gz for a scripts/wiki-media-package build instead of the pinned one)
(cd site && npx wrangler dev)                       # the built site, as Cloudflare runs it: http://localhost:8787
(cd site && npm run dev)                            # or Vite's dev server, which reloads as src/ changes
```

`wrangler dev` serves `site/dist` (no www redirect, unless it's given `--host www.spelunky.ai`); `npm run dev`
(once `build.sh` has installed the packages) serves the app and `public/`, but not what `build.sh` adds
(`/play/`, `/LICENSE.txt`, `/releases/`, `/wiki/media/`).

`site/test.sh` is the site's smoke test. It builds the site with stand-ins for the web build and the wiki's
media (`build.sh --local ... --wiki-media ...`), so it downloads neither, serves it with `wrangler dev`, and
checks every path above (status, caching and other headers, contents, the landing page's), the 404 and the
www redirect, the clips' byte ranges, the wiki's Snake page and hubs, and runs `test-seo.mjs` (SEO, below) and
`test-wiki.mjs` (the wiki's pages compile, and a page that breaks a rule fails the build: docs/wiki-style.md,
"The pipeline"). `wrangler dev`'s asset store isn't quite Cloudflare's: it serves `.txt` as `text/plain;
charset=utf-8` where spelunky.ai's serves a bare `text/plain`, and pages as `text/html; charset=utf-8` where
spelunky.ai's serves a bare `text/html` (the pages' `<meta charset>` covers that). So a content type the Worker
has to set itself is checked by `test-server.mjs` too, which runs `src/server.ts` against a stand-in asset store
that answers as Cloudflare's does. It needs Node and the network (`npm ci`), not Rust, and takes about ten
seconds once npm has the packages cached. `scripts/check` doesn't run it; CI does, for a pull request that
changes `site/`, and `pr-ok` needs it to pass ([docs/ci.md](ci.md#pull-requests)). It leaves `site/dist` built
with the stand-in.

## SEO

Every page's route sets its head with `seo()` (`site/src/seo/seo.ts`), so no page can leave a tag out or
let two disagree:

```ts
head: () => seo({
  title: "Snake – Spelunky Classic fan wiki",
  description: lede,                 // a wiki page's lede (docs/wiki-style.md)
  path: "/wiki/snake",              // where it's served: its canonical URL, and og:url
  type: "article",                  // Open Graph's type; "website" by default
  jsonLd: [article({ ... }), breadcrumbs([...])],
}),
```

It sets the `<title>`, the meta description, `<link rel="canonical">` (`https://spelunky.ai` plus the path,
exactly as served: a page pre-rendered to `dir/index.html` is at `/dir/`, the asset store redirecting `/dir`
there), Open Graph's and Twitter's tags (`summary_large_image`: the wiki's pages and hubs have their own
image, [below](#open-graph-images), and the landing page its gameplay still), and a
`<script type="application/ld+json">` per JSON-LD node.
`site/src/seo/jsonld.ts` builds those, typed: `website()` (with a SearchAction for `/wiki?q=`),
`organization()` and `videoGame()` for the landing page, and `breadcrumbs()`, `article()`, `itemList()`,
`videoObject()`, `imageObject()` and `faqPage()` for the wiki (a page's FAQ, made at build time with its
media's), and `seo({ alternates })` adds a `<link rel="alternate">` per other format, a wiki page's Markdown
twin. Every page's head has a `<link rel="search">` to `/opensearch.xml` (`site/src/seo/opensearch.ts`,
route `src/routes/opensearch[.]xml.ts`, pre-rendered), an OpenSearch description of `/wiki?q=`, so a browser
can add the wiki's search as a search engine: the landing page's too, as Chrome only adds one a site's root
page names. The asset store serves any `.xml` as `application/xml`, so the Worker gives it
`application/opensearchdescription+xml`. Its `<Image>` is the favicon. Every page's head names the site's
icons too, as does `404.html`: `/favicon.ico` (16 to 256 pixels), `/icon.svg` (crisp at any size; browsers
that take an SVG prefer it) and `/apple-touch-icon.png` (180 pixels, opaque, on the site's gold), all in
`site/public/` and made from the game's Spelunker (`sStandLeft`, 16x16, scaled by whole factors) by
`scripts/site-icons`, which `scripts/check` runs with `--check` so they stay what the reference `.gmk`
gives. The site and its Organization are the fan project's; the VideoGame is Derek Yu's, published by
Mossmouth; media carry the game's copyright notice and license ([wiki license](wiki-license.md)).

`/sitemap.xml` comes from `site/src/seo/sitemap.ts`'s `SITEMAP_SOURCES`, lists of pages by canonical path (a
source of the wiki's pages joins them), through the route `src/routes/sitemap[.]xml.ts`, which the build
pre-renders to a static file.

**The wiki's media.** `site/src/seo/media-package.ts` reads the pinned media package's indexes
(`WIKI_MEDIA_DIR`, Wiki media above) at build time, the one reader of them: the wiki's build
(`src/wiki/build/compile.ts`: the pages, the hubs' cards, the Open Graph images) uses what it reads, and its
Vite plugin hands what the JSON-LD and the sitemaps need of them (`wikiMedia`: each clip's title, description,
duration, size, MP4 and poster; each sprite with pixels, its upscaled size, first frame and animation) to the
app as the module `virtual:wiki-media`, `null` without a pin.
The sitemaps are rendered in workerd, which can't read the files, so the indexes are built into the Worker.
`site/src/seo/media.ts` turns them into JSON-LD. A wiki page's head has a `VideoObject` per clip it shows
(its frontmatter's `clips`, in order) and an `ImageObject` of its infobox's sprite, made when the wiki is
compiled (`src/wiki/build/compile.ts`'s `mediaJsonLd`) and carried in the page's meta. A route's code is
the browser's too, so a page's route importing `virtual:wiki-media` would send every visitor the whole index
(about 90 KB); only the sitemaps' routes, the Worker's alone, import it, and `site/test.sh` fails if a
browser script has it.

`clipVideo()` is a clip's `VideoObject`: its spec's title and description, the poster as `thumbnailUrl`, the
MP4 as `contentUrl`, the duration in whole seconds (`PT4S`), the size, and the pin's `date` as `uploadDate`
(`site/wiki-media.txt`, written by `scripts/bump-wiki-media` from the release's `published_at`).
`spriteImage()` is a sprite's `ImageObject`, its first frame at 4x (or its animation, `{ anim: true }`).
Both carry the game's `creditText`, `copyrightNotice` and `license` ([wiki license](wiki-license.md)), and
throw for a clip or sprite the package doesn't have: a page's clip the pinned media haven't rendered yet is
left out of its JSON-LD, as it is of the video sitemap. Media URLs escape each path segment, so a
sprite's `0@4x.png` is `0%404x.png`, the URL the asset store serves rather than redirects.

`/sitemap-video.xml` and `/sitemap-image.xml` are Google's video and image sitemaps, from the same module
(`sitemap.ts`, routes `sitemap-video[.]xml.ts` and `sitemap-image[.]xml.ts`): each page with a clip (its
`<video:video>`s: thumbnail, title, description, MP4, duration and publication date) or a sprite (its
`<image:image>`s). `sitemap.ts`'s `PAGE_MEDIA_SOURCES` say which page shows what: a wiki page shows the clips
of its frontmatter's `clips` and its infobox's sprite, animated if the page shows it so
(`src/wiki/sitemap.ts`'s `wikiPageMedia`, from `virtual:wiki/pages`). A clip the pinned media haven't got
yet (a spec newer than the release) is left out, and a page is listed only if `/sitemap.xml` lists it.
`/robots.txt` (`src/routes/robots[.]txt.ts`) allows everything and names `/sitemap.xml`, and each media
sitemap with a page in it: a `<urlset>` without a `<url>` isn't valid against the sitemaps schema, so an
empty one is served but not named. All four are pre-rendered like the pages.

`site/test-seo.mjs`, run by `site/test.sh`, checks the built site: every pre-rendered page has one title,
description and canonical URL, none shared with another page, the canonical is where the page is served
(a 200, not a redirect), Open Graph's and Twitter's tags are there, and each JSON-LD node parses with
`@context` and a `@type` of ours with the properties search engines require of it. The sitemap must list
exactly the pre-rendered pages (and `/play/`) but those that are `noindex` (a wiki hub with nothing written); the media sitemaps only pages it lists, each item with the
tags Google wants and its media under `/wiki/media/`, served as a 200; and `robots.txt` exactly the sitemaps
with pages. A page's `VideoObject`s (with Google's required `name`, `thumbnailUrl` and `uploadDate`, the
date with its time zone) and `ImageObject`s must be the media the media sitemaps list under it, in order,
and `/wiki/snake` must have both. A page's `FAQPage` must be the FAQ it shows (its questions and, as plain
text, its answers), and `/wiki/snake` must have one. Every wiki page must name its Markdown twin, which
must be built and served, every twin be named by its page, and `/llms.txt` list them all. Every page must
name `/opensearch.xml`. `site/test-media.mjs` runs `media.ts` and the media sitemaps on `test.sh`'s stand-in package,
with stand-in wiki pages (Node runs the TypeScript, stripping its types).

### Open Graph images

Every wiki page and hub, and `/wiki`, has a 1200x630 card as its `og:image` and `twitter:image` (with their
size and alt text): its sprite in a stone frame, its categories, its name and its infobox's first three
numbers (a hub's, how many of its pages are written and to come), under the SPELUNKY title sprite and a
"fan wiki" badge, over the line "Unofficial fan wiki · spelunky.ai" (docs/wiki-license.md). Everything on
it is the game's own pixel art at whole scales, nearest neighbour: the text is in its sprite fonts, `sFont`
and `sFontSmall` (`font_add_sprite`'s, so upper case, `' '` to `'Z'`), the colours the wiki's.
`site/src/wiki/build/og.ts` draws them when the wiki is compiled, from the pinned media's sprites, and the
wiki's Vite plugin writes them to `dist/client/wiki/og/<slug>.<hash>.png` (a hub's at its id,
`items/weapons.<hash>.png`; `/wiki`'s at `wiki.<hash>.png`), the hash being the first 16 hex digits of the
file's SHA-256. So they skip the Worker and are cached for good like `/wiki/media/`, and a card that
changes gets a new URL, which a social site that cached the old one fetches afresh. Without the media
(`npm run dev` without `WIKI_MEDIA_DIR`, or no pin) there are none, and the pages have the landing page's
still; with them, `npm run dev` serves them too. A title or number
with a character the fonts haven't got and `og.ts` has no substitute for (`–` is `-`) fails the build,
naming it.

**Why at the site's build.** Pages are written between releases, so a card has to follow its page, not
the media package; and Workers Builds has Node, not Rust. The cards are drawn by a few hundred lines of
TypeScript on `node:zlib` (`png.ts` reads and writes the PNGs), not satori and resvg or a canvas package:
those bring native or wasm binaries and a font to the build, for drawing that is only blitting sprites at
whole scales. The same pages and media always give the same pixels (integer arithmetic only), and the same
file on the same Node (zlib's level 9: another zlib may compress them differently, which only renames them).
Each card is indexed colour, 3 to 7 KB, and takes about 25 ms.

`site/test-og.mjs`, run by `site/test.sh`, checks that every pre-rendered wiki page and hub names its own
card in `og:image` and `twitter:image` with its size and alt text, that each is served as a 1200x630
`image/png` with immutable caching and the bytes the build wrote, that `/wiki/og/` has nothing else, and
that compiling the wiki again makes the same files. `test.sh`'s stand-in media package has made-up fonts
for it, which `test-og.mjs stand-in` writes.

## Limits

Workers static assets take files of up to 25 MiB and 20,000 files a version on the free plan (100,000 on
Workers Paid, which nothing here needs), and `build.sh` fails over either. The biggest file is the wasm,
about 17.4 MiB (18,223,812 bytes at v0.1.7); the bundle is about 1.2 MiB and the largest sound 1.5 MiB; a web
build is about 110 files. The wiki's media are most of the files: v0.1.7's are 7,506 (7,467 for 804 sprites:
frames, strips and previews at native size and 4x, and a `sprite.json` each; and 8 clips of 4 or 5 files
each), 34 MB unpacked, the largest a 318 KiB clip. With the web build's 107, the wiki's Open Graph images
(one a page and hub: 16 with the first page, about 210 once every page is written, a few KB each) and the
rest's 9, a build is about 7,690 files, so over 12,000 to spare: the sprites only change with the game's assets, and that's room for about
2,400 more clips. If it ever runs short, the native-size images (3,331 files) can go, since the pages only
show the `@4x` ones (`docs/wiki-style.md`, "Clips and images"); or Workers Paid.

Requests for static assets are free; only the Worker's (pages, the landing page's media, the wiki's clips)
count against the 100,000 a day on the free plan, which is why `/play/v/*`, `/assets/*`, `/wiki/media/*` and
`/wiki/og/*` skip it (`run_worker_first` in `wrangler.jsonc`), all but the clips ([Wiki media](#wiki-media), "Byte
ranges").
