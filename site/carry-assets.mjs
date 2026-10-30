// Keeps the content-hashed files of the builds before this one, so that a deploy doesn't 404 what the pages
// they served still name (version skew, 851-3659; docs/web-deploy.md, "Version skew"). A tab opened before a
// deploy loads its chunks lazily (the search index, the hover cards, the popups, the live clips' player), and a
// social site that fetched a page before it fetches its og:image after; each names the build it came from.
// A Workers deploy replaces the static assets, so without this the old names are gone the moment it's live.
//
//   node carry-assets.mjs DIR SITE DATE [MAX]
//
// DIR is the build's static assets (dist/client), SITE the live site to carry from (https://spelunky.ai, or
// - for none: build.sh --no-carry, which test.sh's hermetic build is),
// DATE the build's date (its commit's, ISO 8601) and MAX how many files the build may have in all (Workers'
// 20,000 a version, less build.sh's margin). It writes DIR/build-assets.json, which lists the build's hashed
// files (its own under assets/ and wiki/og/, and the ones it carried) with the date each was last a build's
// own, and copies into DIR the live build's files that this one hasn't got, as its build-assets.json lists
// them: those a build had in the last RETAIN_DAYS, the newest first, up to MAX_CARRIED and MAX. Anything that
// goes wrong fetching them (no list, a network error, a file that's gone) carries less, never fails the
// build: a deploy without the old files is what the site did before.
import { createHash } from "node:crypto";
import { mkdirSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

// Long enough for a tab left open over a weekend, or a social site's crawl of a page and then its image;
// short enough that the old files leave in time. A file carried is one Workers takes and serves from the
// asset store, no Worker request.
export const RETAIN_DAYS = 7;
// What a deploy's churn can't push past: a change to the shared code renames most of a build's ~285 scripts,
// and a page's change renames its own chunk and card. 3,000 is ten of the biggest deploys.
export const MAX_CARRIED = 3000;
// The hashed files' directories: Vite's scripts and styles, and the wiki's Open Graph images (src/wiki/build/og.ts).
export const HASHED = ["assets", "wiki/og"];
export const LIST = "build-assets.json";
const FORMAT = 1;

// The files under `rel` in DIR, by path from DIR.
function filesUnder(dir, rel) {
  const files = [];
  const walk = (rel) => {
    for (const name of readdirSync(join(dir, rel)).sort()) {
      const path = rel ? `${rel}/${name}` : name;
      if (statSync(join(dir, path)).isDirectory()) walk(path);
      else files.push(path);
    }
  };
  try {
    walk(rel);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  return files;
}

// The build's own hashed files, by path from DIR.
export const ownFiles = (dir) => HASHED.flatMap((top) => filesUnder(dir, top));

// The files Workers counts against a version's: all of DIR's but _headers and .assetsignore, which Wrangler
// reads and doesn't upload (build.sh counts them the same way).
const counted = (dir) => filesUnder(dir, "").filter((path) => path !== "_headers" && path !== ".assetsignore").length;

// What to carry of the live build's list: the files this build hasn't got that a build had in the last
// RETAIN_DAYS, the newest first, as many as `room` lets. A path the list names outside HASHED, or with a
// `..`, isn't one this build would serve: it's left out.
export function choose(previous, own, date, room) {
  const oldest = Date.parse(date) - RETAIN_DAYS * 86400 * 1000;
  const mine = new Set(own);
  return Object.entries(previous?.format === FORMAT ? previous.files ?? {} : {})
    .filter(([path, last]) => !mine.has(path) && hashedPath(path) && Date.parse(last) >= oldest)
    .sort(([a, x], [b, y]) => Date.parse(y) - Date.parse(x) || (a < b ? -1 : 1))
    .slice(0, Math.max(0, Math.min(MAX_CARRIED, room)));
}

const hashedPath = (path) =>
  HASHED.some((top) => path.startsWith(`${top}/`)) && !path.split("/").some((part) => part === "" || part === "." || part === "..");

// An Open Graph image is named for its bytes (og.ts: the first 16 hex digits of their SHA-256), so a carried
// one must have them; Vite's names hash its output in its own way, which a file can't be checked against.
export function intact(path, bytes) {
  const og = /^wiki\/og\/.+\.([0-9a-f]{16})\.png$/.exec(path);
  return !og || createHash("sha256").update(bytes).digest("hex").startsWith(og[1]);
}

async function fetchList(site) {
  const response = await fetch(`${site}/${LIST}`, { signal: AbortSignal.timeout(20_000) });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.json();
}

// Each file, a few at a time; what fails is left out.
async function download(site, dir, paths) {
  const got = [];
  const failed = [];
  let next = 0;
  const worker = async () => {
    while (next < paths.length) {
      const path = paths[next++];
      try {
        const response = await fetch(`${site}/${path}`, { signal: AbortSignal.timeout(30_000) });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const bytes = Buffer.from(await response.arrayBuffer());
        if (!intact(path, bytes)) throw new Error("its bytes aren't its name's");
        mkdirSync(dirname(join(dir, path)), { recursive: true });
        writeFileSync(join(dir, path), bytes);
        got.push(path);
      } catch (error) {
        failed.push(`${path} (${error.message})`);
      }
    }
  };
  await Promise.all(Array.from({ length: 16 }, worker));
  return { got: new Set(got), failed };
}

export async function carry(dir, site, date, max, log = console.log) {
  const own = ownFiles(dir);
  const files = Object.fromEntries(own.map((path) => [path, date]));
  let previous = null;
  try {
    previous = site === null ? null : await fetchList(site);
    if (site !== null && previous === null) log(`carry-assets: ${site}/${LIST} isn't there: nothing to carry`);
  } catch (error) {
    log(`carry-assets: couldn't read ${site}/${LIST} (${error.message}): nothing carried`);
  }
  // The room the rest of DIR (the pages, the web build, the wiki's media) leaves, less the list itself.
  const chosen = choose(previous, own, date, max - counted(dir) - 1);
  const { got, failed } = await download(site, dir, chosen.map(([path]) => path));
  for (const [path, last] of chosen) if (got.has(path)) files[path] = last;
  for (const line of failed.slice(0, 5)) log(`carry-assets: couldn't carry ${line}`);
  if (failed.length > 5) log(`carry-assets: and ${failed.length - 5} more`);
  writeFileSync(join(dir, LIST), `${JSON.stringify({ format: FORMAT, date, files }, null, 1)}\n`);
  log(`carry-assets: ${own.length} hashed files of its own, ${site === null ? "none carried" : `${got.size} carried from ${site} (of ${chosen.length} chosen)`}`);
  return { own: own.length, carried: got.size, failed: failed.length };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [dir, site, date, max] = process.argv.slice(2);
  if (!dir || !site || !date || Number.isNaN(Date.parse(date))) {
    console.error("usage: node carry-assets.mjs DIR SITE DATE [MAX]");
    process.exit(2);
  }
  await carry(dir, site === "-" ? null : site.replace(/\/$/, ""), date, max ? Number(max) : 20_000);
}
