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
| `/releases/latest.json` | the native apps' signed update manifest (`docs/updates.md`), the release's own | `no-cache` |

`www.spelunky.ai` redirects (301) to the apex (its pages: the content-hashed files under `/play/v/` and
`/assets/` skip the Worker, but only a page on www would ask for them there), and a path that's none of these
is a 404 with `site/public/404.html`. Everything is in `site/`:

- `src/`: the app. `routes/` has its pages (TanStack Router's file routes; `routeTree.gen.ts` is generated
  from them by the build and `vite dev`, and committed). Pre-rendering is on (`vite.config.ts`): every
  static route is rendered to HTML at build time and served as a static asset like the rest, so the Worker
  renders nothing on a page view. `server.ts` is the Worker: the www redirect, the static assets with the
  `no-cache` header, and the router for server functions and for a page that isn't a file (neither exists
  yet, so on spelunky.ai the router only runs to confirm a 404).
- `public/`: copied as is: the landing page's media, `404.html`, and `_headers` (the immutable caching).
- `wrangler.jsonc`: the Worker and its domains. `vite.config.ts` builds the app with the Cloudflare Vite
  plugin, which writes the deployable Worker to `dist/server` (with its own `wrangler.json`), its static
  assets to `dist/client`, and `.wrangler/deploy/config.json`, which points `wrangler deploy` and
  `wrangler versions upload` at them.
- `package.json` and `package-lock.json` (every package pinned), and `.node-version` (the Node the build runs
  on, which Workers Builds reads).
- `build.sh` and `web-release.txt`: the build, below. `test.sh`: the smoke test (Testing locally).

The site never builds Rust. A GitHub Release carries the web build as `spelunky-web-TAG.tar.gz` (made by
`scripts/web --package`, attached by the release workflow), `site/web-release.txt` names the release and the
tarball's SHA-256, and `site/build.sh` downloads it, checks the checksum, builds the app (`npm ci`, `vite build`
and a type check), and unpacks the web build into its static assets at `/play/`. We
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
   update manifest (`docs/updates.md`).
2. `scripts/bump-web-release TAG` downloads them, checks the manifest's signature, and writes the tag and
   both SHA-256s into `site/web-release.txt`.
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

## Testing locally

```sh
site/test.sh                                        # the smoke test, below
scripts/web --package /tmp/spelunky-web.tar.gz      # a release build, plus /tmp/spelunky-web.tar.gz.sha256
site/build.sh --local /tmp/spelunky-web.tar.gz      # site/dist, checked against the .sha256 beside it
# (add --manifest latest.json to serve an update manifest at /releases/latest.json too)
(cd site && npx wrangler dev)                       # the built site, as Cloudflare runs it: http://localhost:8787
(cd site && npm run dev)                            # or Vite's dev server, which reloads as src/ changes
```

`wrangler dev` serves `site/dist` (no www redirect, unless it's given `--host www.spelunky.ai`); `npm run dev`
(once `build.sh` has installed the packages) serves the app and `public/`, but not what `build.sh` adds
(`/play/`, `/LICENSE.txt`, `/releases/`).

`site/test.sh` is the site's smoke test. It builds the site with a stand-in for the web build (`build.sh
--local`), serves it with `wrangler dev`, and checks every path above (status, caching and other headers,
contents, the landing page's), the 404 and the www redirect. It needs Node and the network (`npm ci`), not
Rust, and takes about ten seconds once npm has the packages cached. `scripts/check` doesn't run it, since a
change to the site gets its own build and preview (Previews, above); run it before opening a pull request
that changes `site/`. It leaves `site/dist` built with the stand-in.

## Limits

Workers static assets take files of up to 25 MiB (`build.sh` fails over that) and 20,000 files on the free
plan. The biggest file is the wasm, about 16.4 MiB (17,147,924 bytes at 851-3252); the bundle is about 1.2
MiB and the largest sound 1.5 MiB; a build is about 110 files. Requests for static assets are free; only
the Worker's (pages, the landing page's media) count against the 100,000 a day on the free plan, which is why
`/play/v/*` and `/assets/*` skip it (`run_worker_first` in `wrangler.jsonc`).
