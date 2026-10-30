// Keeps the content-hashed files of the builds before this one, so that a deploy doesn't 404 what the pages
// they served still name (version skew, 851-3659; docs/web-deploy.md, "Version skew"). A tab opened before a
// deploy loads its chunks lazily (the search index, the hover cards, the popups, the live clips' player), and a
// social site that fetched a page before it fetches its og:image after; each names the build it came from.
// A Workers deploy replaces the static assets, so without this the old names are gone the moment it's live.
//
//   node carry-assets.mjs DIR SITE DATE [MAX]
//
// DIR is the build's static assets (dist/client), SITE the live site to carry from (https://spelunky.ai, or
// - for none: build.sh's but on Workers Builds' builds of main), DATE the build's time (ISO 8601) and MAX how many files the build may
// have in all. It writes DIR/build-assets.json, which lists the build's hashed files (its own under assets/
// and wiki/og/, and the ones it carried) with the time each was last a build's own, and copies into DIR the
// live build's files that this one hasn't got, as its build-assets.json lists them: those a build had in the
// last RETAIN_DAYS, the newest first, up to MAX_CARRIED and MAX. Anything that goes wrong fetching them (no
// list, a network error, a file that's gone) carries less, never fails the build: a deploy without the old
// files is what the site did before.
import { createHash } from "node:crypto";
import { mkdirSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

// Long enough for a tab left open over a weekend, or a social site's crawl of a page and then its image;
// short enough that the old files leave in time. A file carried is one Workers takes and serves from the
// asset store, no Worker request.
export const RETAIN_DAYS = 7;
// What a busy day's deploys can't push past. A deploy renames about 225 of a build's ~285 scripts (every
// page's chunk has the commit in its links to GitHub), so 6,000 is the last 25 or so deploys': a day's, at
// main's busiest, and the week's when it's quieter. Downloading them takes a build about half a minute.
export const MAX_CARRIED = 6000;
// How long the carry may take in all, whatever the live site does (a stall, a rate limit): what's downloaded
// by then is kept, and the build goes on.
export const DEADLINE_MS = 180_000;
// The hashed files' directories: Vite's scripts and styles, and the wiki's Open Graph images (src/wiki/build/og.ts).
export const HASHED = ["assets", "wiki/og"];
export const LIST = "build-assets.json";
const FORMAT = 1;
// What the asset store serves each kind of hashed file as. A response that isn't it (an error page, a proxy's
// interstitial) isn't the file, and mustn't be served as it for a year under _headers' immutable caching.
const TYPES = { js: "javascript", css: "text/css", png: "image/png" };

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
// reads and doesn't upload (as build.sh's own count does).
const counted = (dir) => filesUnder(dir, "").filter((path) => path !== "_headers" && path !== ".assetsignore").length;

// What could be carried of the live build's list, in the order to carry it: the files this build hasn't got
// that a build had in the last RETAIN_DAYS, the newest first. A path the list names outside HASHED, with a
// `..`, or of a kind TYPES hasn't got, isn't one a build serves: it's left out.
export function candidates(previous, own, date) {
  const oldest = Date.parse(date) - RETAIN_DAYS * 86400 * 1000;
  const mine = new Set(own);
  return Object.entries(previous?.format === FORMAT ? previous.files ?? {} : {})
    .filter(([path, last]) => !mine.has(path) && hashedPath(path) && Date.parse(last) >= oldest)
    .sort(([a, x], [b, y]) => Date.parse(y) - Date.parse(x) || (a < b ? -1 : 1));
}

// Whether a build could carry a file of this name: of a kind TYPES has (test.sh fails a build with another, which
// the next build would list but never carry).
export const carriable = (path) => Object.hasOwn(TYPES, path.slice(path.lastIndexOf(".") + 1));

const hashedPath = (path) =>
  HASHED.some((top) => path.startsWith(`${top}/`)) &&
  !path.split("/").some((part) => part === "" || part === "." || part === "..") &&
  carriable(path);

// Whether a response is the file its path names: the kind of file its extension says, and for an Open Graph
// image, which is named for its bytes (og.ts: the first 16 hex digits of their SHA-256), those bytes. Vite's
// names hash its output in its own way, which a script can't be checked against.
export function intact(path, type, bytes) {
  if (bytes.length === 0 || !(type ?? "").includes(TYPES[path.slice(path.lastIndexOf(".") + 1)])) return false;
  const og = /^wiki\/og\/.+\.([0-9a-f]{16})\.png$/.exec(path);
  return !og || createHash("sha256").update(bytes).digest("hex").startsWith(og[1]);
}

async function fetchList(site) {
  const response = await fetch(`${site}/${LIST}`, { signal: AbortSignal.timeout(20_000) });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.json();
}

// The candidates in order, 16 at a time, until `limit` of them are carried or DEADLINE_MS is up: in rounds of
// as many as are still wanted, so that the next ones take the places of those that failed.
async function download(site, dir, paths, limit, deadlineMs) {
  const got = new Set();
  const failed = [];
  const deadline = AbortSignal.timeout(deadlineMs);
  const one = async (path) => {
    try {
      const url = `${site}/${path.split("/").map(encodeURIComponent).join("/")}`;
      const response = await fetch(url, { signal: AbortSignal.any([AbortSignal.timeout(30_000), deadline]) });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const bytes = Buffer.from(await response.arrayBuffer());
      if (!intact(path, response.headers.get("content-type"), bytes)) throw new Error("not the file its name says");
      mkdirSync(dirname(join(dir, path)), { recursive: true });
      writeFileSync(join(dir, path), bytes);
      got.add(path);
    } catch (error) {
      failed.push(`${path} (${error.message})`);
    }
  };
  let next = 0;
  while (next < paths.length && got.size < limit && !deadline.aborted) {
    const round = paths.slice(next, next + (limit - got.size));
    next += round.length;
    let i = 0;
    await Promise.all(
      Array.from({ length: 16 }, async () => {
        while (i < round.length && !deadline.aborted) await one(round[i++]);
      }),
    );
  }
  if (deadline.aborted) failed.push(`the rest (${deadlineMs / 1000} s were up)`);
  return { got, failed };
}

export async function carry(dir, site, date, max, log = console.log, deadlineMs = DEADLINE_MS) {
  const own = ownFiles(dir);
  const files = Object.fromEntries(own.map((path) => [path, date]));
  let previous = null;
  try {
    previous = site === null ? null : await fetchList(site);
    if (site !== null && previous === null) log(`carry-assets: ${site}/${LIST} isn't there: nothing to carry`);
  } catch (error) {
    log(`carry-assets: couldn't read ${site}/${LIST} (${error.message}): nothing carried`);
  }
  const listed = candidates(previous, own, date);
  // The room the rest of DIR (the pages, the web build, the wiki's media) leaves, less the list itself.
  const limit = Math.max(0, Math.min(MAX_CARRIED, max - counted(dir) - 1));
  const { got, failed } = await download(site, dir, listed.map(([path]) => path), limit, deadlineMs);
  for (const [path, last] of listed) if (got.has(path)) files[path] = last;
  for (const line of failed.slice(0, 5)) log(`carry-assets: couldn't carry ${line}`);
  if (failed.length > 5) log(`carry-assets: and ${failed.length - 5} more`);
  writeFileSync(join(dir, LIST), `${JSON.stringify({ format: FORMAT, date, files }, null, 1)}\n`);
  const carried = site === null ? "none carried" : `${got.size} carried from ${site} (of ${listed.length} it listed within the week; room for ${limit})`;
  log(`carry-assets: ${own.length} hashed files of its own, ${carried}`);
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
