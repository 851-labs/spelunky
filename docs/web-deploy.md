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
| `/wiki`, `/wiki/<slug>` | the wiki: its index and pages (`site/src/routes/wiki/`, from `site/content/wiki/`; docs/wiki-style.md), pre-rendered | `no-cache` |
| `/robots.txt`, `/sitemap.xml`, `/sitemap-video.xml`, `/sitemap-image.xml` | for crawlers (SEO, below): pre-rendered | `no-cache` |
| `/releases/latest.json` | the native apps' signed update manifest (`docs/updates.md`), the release's own | `no-cache` |
| `/wiki/media/HASH/` | the wiki's sprites and clips (`docs/wiki-media.md`), `HASH` being of their package | `immutable`, a year |

`www.spelunky.ai` redirects (301) to the apex (its pages: the content-hashed files under `/play/v/`,
`/assets/` and `/wiki/media/` skip the Worker, but only a page on www would ask for them there), and a path
that's none of these is a 404 with `site/public/404.html`. Everything is in `site/`:

- `src/`: the app. `routes/` has its pages (TanStack Router's file routes; `routeTree.gen.ts` is generated
  from them by the build and `vite dev`, and committed), and `seo/` their head tags, JSON-LD and the
  sitemap (SEO, below), and `wiki/` the wiki's components, layout and build (docs/wiki-style.md, "The
  pipeline"). Pre-rendering is on (`vite.config.ts`): every static route is rendered to HTML at
  build time and served as a static asset like the rest, so the Worker renders nothing on a page view.
  `server.ts` is the Worker: the www redirect, the static assets with the `no-cache` header, and the router
  for server functions and for a page that isn't a file (neither exists yet, so on spelunky.ai the router
  only runs to confirm a 404).
- `content/`: the wiki's pages (`content/wiki/<slug>.mdx`) and the snapshot of its stats
  (`content/wiki-data/stats.json`, docs/wiki-data.md).
- `public/`: copied as is: the landing page's media, `404.html`, and `_headers` (the immutable caching).
- `wrangler.jsonc`: the Worker and its domains. `vite.config.ts` builds the app with the Cloudflare Vite
  plugin, which writes the deployable Worker to `dist/server` (with its own `wrangler.json`), its static
  assets to `dist/client`, and `.wrangler/deploy/config.json`, which points `wrangler deploy` and
  `wrangler versions upload` at them.
- `package.json` and `package-lock.json` (every package pinned), and `.node-version` (the Node the build runs
  on, which Workers Builds reads).
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
   writes the tag and their SHA-256 into `site/wiki-media.txt` (a release without them leaves it alone).
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
`scripts/bump-web-release` moves both pins together, so a PR changing the pin is a deploy too, and while it's
`none` the site has no `/wiki/media/`. The media are unpacked before the app's build, which gets
`WIKI_MEDIA_DIR` (the unpacked package, for the pages to read `sprites/index.json` and `clips/index.json` as
they're pre-rendered), `WIKI_MEDIA_URL` (`/wiki/media/HASH`, to link the files) and `WIKI_MEDIA_DATE` (the
pin's `date`, when its release was published, or a `--wiki-media` package's modification time); none is set
while the pin is `none`, nor under `npm run dev`. The pages get them through `virtual:wiki-media` (SEO,
below).

**Caching.** A file's name doesn't say what's in it: `sSnakeWalkL/frames/0@4x.png` keeps its name when the
sprites are re-exported, and every clip's caption names the release that rendered it. So the directory names
the package, the files never change, and they're cached for good (`_headers`), like `/play/v/HASH/`. They
skip the Worker too (`run_worker_first`): a wiki page shows dozens of sprites and a few clips, each a request,
which through the Worker would come out of the free plan's 100,000 a day. A page names the current `HASH`,
and the pages are `no-cache`, so a new pin shows up on the next page view; a page loaded just before a deploy
can miss its media until a reload, as `/play/` can (Rolling back), and that miss is cached as immutable too.
The router never sees `/wiki/media/...`, and a wiki page mustn't be called `media` (`build.sh` fails if the
app pre-renders one).

Two things a page has to do:

- **Escape `@` as `%40`.** The asset store answers `0@4x.png` with a 307 to `0%404x.png`, its canonical form,
  so asking for that directly saves a round trip per image (`encodeURIComponent` on each path segment).
- **Expect no byte ranges.** The asset store ignores `Range` (a 200 with the whole file, on spelunky.ai as
  under `wrangler dev`), which Safari wants for an MP4 in a `<video>`; 851-3386 follows that up.

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
www redirect, the wiki's Snake page, and runs `test-seo.mjs` (SEO, below) and `test-wiki.mjs` (the wiki's
pages compile, and a page that breaks a rule fails the build: docs/wiki-style.md, "The pipeline"). It needs Node and the network (`npm ci`), not Rust, and
takes about ten seconds once npm has the packages cached. `scripts/check` doesn't run it, since a
change to the site gets its own build and preview (Previews, above); run it before opening a pull request
that changes `site/`. It leaves `site/dist` built with the stand-in.

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
there), Open Graph's and Twitter's tags (`summary_large_image`, with the landing page's gameplay still until
851-3340 makes an image per page), and a `<script type="application/ld+json">` per JSON-LD node.
`site/src/seo/jsonld.ts` builds those, typed: `website()` (with a SearchAction for `/wiki?q=`),
`organization()` and `videoGame()` for the landing page, and `breadcrumbs()`, `article()`, `itemList()`,
`videoObject()`, `imageObject()` and `faqPage()` for the wiki. The site and its Organization are the fan
project's; the VideoGame is Derek Yu's, published by Mossmouth; media carry the game's copyright notice and
license ([wiki license](wiki-license.md)).

`/sitemap.xml` comes from `site/src/seo/sitemap.ts`'s `SITEMAP_SOURCES`, lists of pages by canonical path (a
source of the wiki's pages joins them), through the route `src/routes/sitemap[.]xml.ts`, which the build
pre-renders to a static file.

**The wiki's media.** `site/src/seo/media-package.ts` reads the pinned media package's indexes
(`WIKI_MEDIA_DIR`, Wiki media above) at build time, and its Vite plugin hands what the pages need of them
(each clip's title, description, duration, size, MP4 and poster; each sprite's upscaled size, first frame and
animation) to the app as the module `virtual:wiki-media`, `null` without a pin.
The pages and sitemaps are rendered in workerd, which can't read the files, so the indexes are built into
the Worker. `site/src/seo/media.ts` turns them into JSON-LD:

```ts
import wikiMedia from "virtual:wiki-media";

jsonLd: [article({ ... }), clipVideo(wikiMedia, "snake/bite"), spriteImage(wikiMedia, "sSnakeLeft")],
```

`clipVideo()` is a clip's `VideoObject`: its spec's title and description, the poster as `thumbnailUrl`, the
MP4 as `contentUrl`, the duration in whole seconds (`PT4S`), the size, and the pin's `date` as `uploadDate`
(`site/wiki-media.txt`, written by `scripts/bump-web-release` from the release's `published_at`).
`spriteImage()` is a sprite's `ImageObject`, its first frame at 4x (or its animation, `{ anim: true }`).
Both carry the game's `creditText`, `copyrightNotice` and `license` ([wiki license](wiki-license.md)), and
a clip or sprite the package doesn't have fails the build. Media URLs escape each path segment, so a
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
exactly the pre-rendered pages (and `/play/`); the media sitemaps only pages it lists, each item with the
tags Google wants and its media under `/wiki/media/`, served as a 200; and `robots.txt` exactly the sitemaps
with pages. `site/test-media.mjs` runs `media.ts` and the media sitemaps on `test.sh`'s stand-in package,
with stand-in wiki pages (Node runs the TypeScript, stripping its types).

## Limits

Workers static assets take files of up to 25 MiB and 20,000 files a version on the free plan (100,000 on
Workers Paid, which nothing here needs), and `build.sh` fails over either. The biggest file is the wasm,
about 17.4 MiB (18,223,812 bytes at v0.1.7); the bundle is about 1.2 MiB and the largest sound 1.5 MiB; a web
build is about 110 files. The wiki's media are most of the files: v0.1.7's are 7,506 (7,467 for 804 sprites:
frames, strips and previews at native size and 4x, and a `sprite.json` each; and 8 clips of 4 or 5 files
each), 34 MB unpacked, the largest a 318 KiB clip. With the web build's 107 and the rest's 9, a build is
7,622 files, so 12,378 to spare: the sprites only change with the game's assets, and that's room for about
2,400 more clips. If it ever runs short, the native-size images (3,331 files) can go, since the pages only
show the `@4x` ones (`docs/wiki-style.md`, "Clips and images"); or Workers Paid.

Requests for static assets are free; only the Worker's (pages, the landing page's media) count against the
100,000 a day on the free plan, which is why `/play/v/*`, `/assets/*` and `/wiki/media/*` skip it
(`run_worker_first` in `wrangler.jsonc`).
