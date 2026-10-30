// build.sh's carrying of the builds before it's hashed files (carry-assets.mjs; docs/web-deploy.md, "Version
// skew"), which site/test.sh runs: against a stand-in live site on localhost, never spelunky.ai.
//
//   node test-carry.mjs
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { candidates, carry, intact, MAX_CARRIED, RETAIN_DAYS } from "./carry-assets.mjs";

let failures = 0;
const fail = (message) => {
  failures += 1;
  console.log(`FAIL ${message}`);
};

const DATE = "2026-10-29T12:00:00Z";
const daysAgo = (days) => new Date(Date.parse(DATE) - days * 86400 * 1000).toISOString().replace(/\.\d+Z$/, "Z");
const hoursAgo = (hours) => daysAgo(hours / 24);
const og = (name, bytes) => `wiki/og/${name}.${createHash("sha256").update(bytes).digest("hex").slice(0, 16)}.png`;
const TYPES = { js: "text/javascript", css: "text/css; charset=utf-8", png: "image/png" };

// The live build: what it serves, and its list. Its own files an hour old, ones it carried, and what a list
// shouldn't have.
const served = {
  "assets/hub-OLD1.js": "old hub",
  "assets/wiki-OLD2.css": "old css",
  "assets/shared-SAME.js": "the same file both builds have",
  [og("monsters", "old card")]: "old card",
  "wiki/og/snake.0000000000000000.png": "bytes that aren't its name's",
  "assets/stale-OLD3.js": "carried 6 days ago",
};
// A file the live site answers as HTML, as an error page or a proxy's interstitial would.
const asHtml = new Set(["assets/html-OLD6.js"]);
const files = {
  "assets/hub-OLD1.js": hoursAgo(1),
  "assets/wiki-OLD2.css": hoursAgo(1),
  "assets/shared-SAME.js": hoursAgo(1),
  [og("monsters", "old card")]: hoursAgo(1),
  "wiki/og/snake.0000000000000000.png": hoursAgo(1),
  "assets/stale-OLD3.js": daysAgo(RETAIN_DAYS - 1),
  "assets/html-OLD6.js": hoursAgo(1),
  // Too old to keep; gone from the live site (a 404); and paths no build serves.
  "assets/ancient-OLD4.js": daysAgo(RETAIN_DAYS + 1),
  "assets/gone-OLD5.js": hoursAgo(1),
  "play/v/abc/game.wasm": hoursAgo(1),
  "assets/../_headers": hoursAgo(1),
  "assets/notes.txt": hoursAgo(1),
};
const server = createServer((request, response) => {
  const path = decodeURIComponent(new URL(request.url, "http://x").pathname.slice(1));
  if (path === "build-assets.json") return response.end(JSON.stringify({ format: 1, date: hoursAgo(1), files }));
  if (asHtml.has(path)) return response.writeHead(200, { "content-type": "text/html" }).end("<!doctype html>");
  if (Object.hasOwn(served, path)) return response.writeHead(200, { "content-type": TYPES[path.split(".").pop()] }).end(served[path]);
  response.writeHead(404, { "content-type": "text/html" }).end("not here");
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const site = `http://127.0.0.1:${server.address().port}`;

// This build: its own scripts (one the live one has too) and card, and a page: 4 files.
const build = (dir) => {
  for (const [path, text] of Object.entries({ "index.html": "page", "assets/entry-NEW1.js": "new", "assets/shared-SAME.js": "the same file both builds have", [og("monsters", "new card")]: "new card" })) {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), text);
  }
};
const quiet = () => {};
const inDir = async (body) => {
  const dir = mkdtempSync(join(tmpdir(), "test-carry-"));
  try {
    build(dir);
    return await body(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
};
const sorted = (o) => JSON.stringify(Object.fromEntries(Object.entries(o).sort(([a], [b]) => (a < b ? -1 : 1))));

try {
  // What it carries: the live build's files within the week that this one hasn't got and the live site still
  // serves, as what they say they are, and nothing outside assets/ and wiki/og/; this build's own dated now.
  await inDir(async (dir) => {
    const result = await carry(dir, site, DATE, 20_000, quiet);
    const list = JSON.parse(readFileSync(join(dir, "build-assets.json"), "utf8"));
    const carried = ["assets/hub-OLD1.js", "assets/stale-OLD3.js", "assets/wiki-OLD2.css", og("monsters", "old card")];
    const want = {
      "assets/entry-NEW1.js": DATE,
      "assets/shared-SAME.js": DATE,
      [og("monsters", "new card")]: DATE,
      ...Object.fromEntries(carried.map((path) => [path, files[path]])),
    };
    if (list.date !== DATE || sorted(list.files) !== sorted(want)) fail(`carry: listed ${sorted(list.files)} at ${list.date}, not ${sorted(want)}`);
    for (const path of carried) if (readFileSync(join(dir, path), "utf8") !== served[path]) fail(`carry: ${path} isn't the live site's`);
    for (const path of ["assets/ancient-OLD4.js", "assets/gone-OLD5.js", "assets/html-OLD6.js", "play/v/abc/game.wasm", "wiki/og/snake.0000000000000000.png", "assets/notes.txt"]) {
      if (existsSync(join(dir, path))) fail(`carry: carried ${path}`);
    }
    if (readFileSync(join(dir, "assets/shared-SAME.js"), "utf8") !== "the same file both builds have") fail("carry: replaced the build's own file");
    // gone-OLD5 (a 404), html-OLD6 (HTML) and the snake card (the wrong bytes) failed.
    if (result.own !== 3 || result.carried !== 4 || result.failed !== 3) fail(`carry: ${JSON.stringify(result)}`);
    if (failures === 0) console.log("ok   carry: the week's files the build hasn't got, as they say they are, from assets/ and wiki/og/ only");
  });

  // Within the build's room: the newest first, the next in line taking a failed one's place, never past the
  // total or MAX_CARRIED, and none when there's no room.
  {
    const before = failures;
    const previous = { format: 1, files: { "assets/a.js": daysAgo(3), "assets/b.js": daysAgo(1), "assets/c.js": daysAgo(2) } };
    if (candidates(previous, ["assets/b.js"], DATE).map(([path]) => path).join() !== "assets/c.js,assets/a.js") fail("room: the candidates' order");
    if (candidates({ format: 2, files: previous.files }, [], DATE).length !== 0 || candidates(null, [], DATE).length !== 0) fail("room: carried from an unknown list");
    // The build's 4 files and its list leave room for 3 in 8: the hour-old ones, but the three that fail,
    // and the 6-day-old one after them.
    await inDir(async (dir) => {
      const result = await carry(dir, site, DATE, 8, quiet);
      const list = JSON.parse(readFileSync(join(dir, "build-assets.json"), "utf8"));
      const got = Object.keys(list.files).filter((path) => list.files[path] !== DATE).sort().join();
      if (result.carried !== 3 || got !== `assets/hub-OLD1.js,assets/wiki-OLD2.css,${og("monsters", "old card")}`) fail(`room: room for 3 carried ${got}`);
    });
    await inDir(async (dir) => {
      const result = await carry(dir, site, DATE, 5, quiet);
      if (result.carried !== 0) fail(`room: no room carried ${result.carried}`);
    });
    // MAX_CARRIED, from a list that has more.
    const many = Object.fromEntries(Array.from({ length: MAX_CARRIED + 5 }, (_, i) => [`assets/x${i}.js`, hoursAgo(1)]));
    const big = createServer((request, response) => {
      const path = new URL(request.url, "http://x").pathname.slice(1);
      if (path === "build-assets.json") return response.end(JSON.stringify({ format: 1, files: many }));
      response.writeHead(200, { "content-type": "text/javascript" }).end("x");
    });
    await new Promise((resolve) => big.listen(0, "127.0.0.1", resolve));
    try {
      await inDir(async (dir) => {
        const result = await carry(dir, `http://127.0.0.1:${big.address().port}`, DATE, 100_000, quiet);
        if (result.carried !== MAX_CARRIED) fail(`room: carried ${result.carried}, not ${MAX_CARRIED}`);
      });
    } finally {
      big.close();
    }
    if (failures === before) console.log(`ok   room: the newest first, the next for one that fails, at most ${MAX_CARRIED} and the build's room`);
  }

  // A file is what it says: its kind, not empty, and an Open Graph image its name's bytes.
  {
    const before = failures;
    const x = Buffer.from("x");
    if (!intact(og("snake", "x"), "image/png", x) || intact(og("snake", "x"), "image/png", Buffer.from("y"))) fail("intact: an Open Graph image's bytes");
    if (!intact("assets/hub-ABC.js", "application/javascript", x) || !intact("assets/a.css", "text/css; charset=utf-8", x)) fail("intact: a script or a stylesheet");
    if (intact("assets/hub-ABC.js", "text/html", x) || intact("assets/hub-ABC.js", null, x) || intact("assets/hub-ABC.js", "text/javascript", Buffer.alloc(0))) fail("intact: an error page or an empty file");
    if (failures === before) console.log("ok   intact: a file of its kind, not empty, an Open Graph image with its name's bytes");
  }

  // A live site that stalls: what's downloaded when the deadline is up is kept, and the build goes on.
  {
    const before = failures;
    const slow = createServer((request, response) => {
      const path = new URL(request.url, "http://x").pathname.slice(1);
      if (path === "build-assets.json") return response.end(JSON.stringify({ format: 1, files: { "assets/fast.js": hoursAgo(1), "assets/slow.js": hoursAgo(2) } }));
      if (path === "assets/fast.js") return response.writeHead(200, { "content-type": "text/javascript" }).end("x");
      // Never answers.
    });
    await new Promise((resolve) => slow.listen(0, "127.0.0.1", resolve));
    try {
      await inDir(async (dir) => {
        const started = Date.now();
        const result = await carry(dir, `http://127.0.0.1:${slow.address().port}`, DATE, 20_000, quiet, 1000);
        const took = Date.now() - started;
        if (result.carried !== 1 || !existsSync(join(dir, "assets/fast.js")) || took > 5000) fail(`deadline: carried ${result.carried} in ${took} ms`);
      });
    } finally {
      slow.closeAllConnections();
      slow.close();
    }
    if (failures === before) console.log("ok   deadline: a stalled live site's files left behind when it's up, the rest kept");
  }

  // No live site, or no list on it (the first deploy with this): the build's own list, nothing carried, no
  // failure; and no live site asked (build.sh's but on Workers Builds' builds of main) the same.
  {
    const before = failures;
    for (const [what, from] of [["no list", `${site}/nothing`], ["no site", "http://127.0.0.1:9"], ["not asked", null]]) {
      await inDir(async (dir) => {
        const result = await carry(dir, from, DATE, 20_000, quiet);
        const list = JSON.parse(readFileSync(join(dir, "build-assets.json"), "utf8"));
        if (result.carried !== 0 || Object.keys(list.files).length !== 3) fail(`${what}: carried ${result.carried}, listed ${Object.keys(list.files)}`);
      });
    }
    if (failures === before) console.log("ok   no live list: the build's own list, nothing carried");
  }
} finally {
  server.close();
}

console.log(failures === 0 ? "test-carry.mjs: all passed" : `test-carry.mjs: ${failures} failed`);
process.exit(failures === 0 ? 0 : 1);
