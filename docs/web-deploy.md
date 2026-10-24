# Deploying spelunky.ai

<https://spelunky.ai> is a Cloudflare Worker with static assets (account "851",
`630f294bcb2c1e9b751d9fe0655a453a`; zone spelunky.ai, `6661e9e5fb33bb71d0d23105fc615009`):

| Path | What | Caching |
| --- | --- | --- |
| `/` | the landing page (`site/public/`) | `no-cache` |
| `/play/` | the game: a release's web build (`scripts/web --package`) | `no-cache` (the page and `manifest.txt`) |
| `/play/v/HASH/` | the build's wasm, JS and assets, `HASH` being of their contents | `immutable`, a year |
| `/LICENSE.txt` | the Spelunky User License, copied from the repo | `no-cache` |

`www.spelunky.ai` redirects (301) to the apex (its pages: the build's files under `/play/v/` skip the
Worker, but only a page on www would ask for them there). Everything is in `site/`: `wrangler.jsonc` (the Worker and its
domains), `worker.js` (the redirect and the `no-cache` header), `public/_headers` (the immutable caching),
`build.sh` and `web-release.txt`.

The site never builds Rust. A GitHub Release carries the web build as `spelunky-web-TAG.tar.gz` (made by
`scripts/web --package`, attached by the release workflow), `site/web-release.txt` names the release and the
tarball's SHA-256, and `site/build.sh` downloads it, checks the checksum and unpacks it at `/play/`. There's
no Cloudflare API token anywhere: Cloudflare's Workers Builds (its Git integration) builds and deploys with
its own credentials.

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
| Builds for non-production branches | off |
| Build watch paths | include `site/*`, exclude nothing |
| Environment variables | none |

The watch path's `*` spans directories, so a change anywhere under `site/` builds and nothing else does
(the game's code changes don't touch the site until a release is bumped in). `wrangler deploy` attaches
both custom domains and creates their DNS records and certificates; if spelunky.ai or www already has a DNS
record (a registrar's parking page, say), delete it first or the deploy fails.

Until a release has a web build, `site/web-release.txt` says `none` and `build.sh` fails with a message
saying so, so the first build after connecting fails, deploying nothing, until the first bump below lands.

## Putting a version live

1. The release workflow (851-3250, not written yet) publishes a GitHub Release `TAG` with
   `spelunky-web-TAG.tar.gz`, made by `scripts/web --package`.
2. `scripts/bump-web-release TAG` downloads it and writes its tag and SHA-256 into `site/web-release.txt`.
3. Open a PR with that change; merging it to `main` builds and deploys the site (a minute or two).

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
of the free plan's 100,000 a day on every game start.

## Testing locally

```sh
scripts/web --package /tmp/spelunky-web.tar.gz      # a release build, plus /tmp/spelunky-web.tar.gz.sha256
site/build.sh --local /tmp/spelunky-web.tar.gz      # site/dist, checked against the .sha256 beside it
(cd site && npx wrangler dev)                       # http://localhost:8787 (no www redirect locally)
```

## Limits

Workers static assets take files of up to 25 MiB (`build.sh` fails over that) and 20,000 files on the free
plan. The biggest file is the wasm, about 16.4 MiB (17,147,924 bytes at 851-3252); the bundle is about 1.2
MiB and the largest sound 1.5 MiB; a build is about 110 files. Requests for static assets are free; only
the Worker's (pages, the landing page's media) count against the 100,000 a day on the free plan, which is why
`/play/v/*` skips it (`run_worker_first` in `wrangler.jsonc`).
