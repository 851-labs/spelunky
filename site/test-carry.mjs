// build.sh's carrying of the builds before it's hashed files (carry-assets.mjs; docs/web-deploy.md, "Version
// skew"), which site/test.sh runs: against a stand-in live site on localhost, never spelunky.ai.
//
//   node test-carry.mjs
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { carry, choose, intact, MAX_CARRIED, RETAIN_DAYS } from "./carry-assets.mjs";

let failures = 0;
const fail = (message) => {
  failures += 1;
  console.log(`FAIL ${message}`);
};

const DATE = "2026-10-29T12:00:00Z";
const daysAgo = (days) => new Date(Date.parse(DATE) - days * 86400 * 1000).toISOString().replace(/\.\d+Z$/, "Z");
const og = (name, bytes) => `wiki/og/${name}.${createHash("sha256").update(bytes).digest("hex").slice(0, 16)}.png`;

// The live build: its own files a day old, ones it carried, and what a list shouldn't have.
const served = {
  "assets/hub-OLD1.js": "old hub",
  "assets/wiki-OLD2.css": "old css",
  "assets/shared-SAME.js": "the same file both builds have",
  [og("monsters", "old card")]: "old card",
  "wiki/og/snake.0000000000000000.png": "bytes that aren't its name's",
  "assets/stale-OLD3.js": "carried 6 days ago",
};
const files = {
  "assets/hub-OLD1.js": daysAgo(1),
  "assets/wiki-OLD2.css": daysAgo(1),
  "assets/shared-SAME.js": daysAgo(1),
  [og("monsters", "old card")]: daysAgo(1),
  "wiki/og/snake.0000000000000000.png": daysAgo(1),
  "assets/stale-OLD3.js": daysAgo(RETAIN_DAYS - 1),
  // Too old to keep; gone from the live site (a 404); and paths no build serves.
  "assets/ancient-OLD4.js": daysAgo(RETAIN_DAYS + 1),
  "assets/gone-OLD5.js": daysAgo(1),
  "play/v/abc/game.wasm": daysAgo(1),
  "assets/../_headers": daysAgo(1),
};
const server = createServer((request, response) => {
  const path = decodeURIComponent(new URL(request.url, "http://x").pathname.slice(1));
  if (path === "build-assets.json") return response.end(JSON.stringify({ format: 1, date: daysAgo(1), files }));
  if (Object.hasOwn(served, path)) return response.end(served[path]);
  response.writeHead(404).end("not here");
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const site = `http://127.0.0.1:${server.address().port}`;

// This build: its own scripts (one the live one has too) and card, and a page.
const build = (dir, extra = {}) => {
  for (const [path, text] of Object.entries({ "index.html": "page", "assets/entry-NEW1.js": "new", "assets/shared-SAME.js": "the same file both builds have", [og("monsters", "new card")]: "new card", ...extra })) {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), text);
  }
};
const quiet = () => {};

try {
  // What it carries: the live build's files within the week that this one hasn't got and the live site still
  // serves, intact, and nothing outside assets/ and wiki/og/; this build's own dated today.
  {
    const dir = mkdtempSync(join(tmpdir(), "test-carry-"));
    try {
      build(dir);
      const result = await carry(dir, site, DATE, 20_000, quiet);
      const list = JSON.parse(readFileSync(join(dir, "build-assets.json"), "utf8"));
      const carried = ["assets/hub-OLD1.js", "assets/stale-OLD3.js", "assets/wiki-OLD2.css", og("monsters", "old card")];
      const want = {
        "assets/entry-NEW1.js": DATE,
        "assets/shared-SAME.js": DATE,
        [og("monsters", "new card")]: DATE,
        ...Object.fromEntries(carried.map((path) => [path, files[path]])),
      };
      const sorted = (o) => JSON.stringify(Object.fromEntries(Object.entries(o).sort(([a], [b]) => (a < b ? -1 : 1))));
      if (sorted(list.files) !== sorted(want)) fail(`carry: listed ${sorted(list.files)}, not ${sorted(want)}`);
      for (const path of carried) if (readFileSync(join(dir, path), "utf8") !== served[path]) fail(`carry: ${path} isn't the live site's`);
      for (const path of ["assets/ancient-OLD4.js", "assets/gone-OLD5.js", "play/v/abc/game.wasm", "wiki/og/snake.0000000000000000.png"]) {
        if (existsSync(join(dir, path))) fail(`carry: carried ${path}`);
      }
      if (readFileSync(join(dir, "assets/shared-SAME.js"), "utf8") !== "the same file both builds have") fail("carry: replaced the build's own file");
      if (result.own !== 3 || result.carried !== 4 || result.failed !== 2) fail(`carry: ${JSON.stringify(result)}`);
      if (failures === 0) console.log("ok   carry: the week's files the build hasn't got, intact, from assets/ and wiki/og/ only");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  }

  // Within the build's room: the newest first, never past the total, and none when there's none.
  {
    const before = failures;
    const previous = { format: 1, files: { "assets/a.js": daysAgo(3), "assets/b.js": daysAgo(1), "assets/c.js": daysAgo(2) } };
    const picked = (room) => choose(previous, [], DATE, room).map(([path]) => path).join();
    if (picked(2) !== "assets/b.js,assets/c.js") fail(`room: 2 picks ${picked(2)}`);
    if (picked(0) !== "" || picked(-5) !== "") fail("room: none picks some");
    const many = { format: 1, files: Object.fromEntries(Array.from({ length: MAX_CARRIED + 10 }, (_, i) => [`assets/x${i}.js`, daysAgo(1)])) };
    if (choose(many, [], DATE, 100_000).length !== MAX_CARRIED) fail(`room: more than ${MAX_CARRIED} carried`);
    // Another format, or no list, is nothing to carry.
    if (choose({ format: 2, files: previous.files }, [], DATE, 10).length !== 0 || choose(null, [], DATE, 10).length !== 0) fail("room: carried from an unknown list");
    const dir = mkdtempSync(join(tmpdir(), "test-carry-"));
    try {
      build(dir);
      // The build's 5 files, its list and 2 carried make 8.
      const result = await carry(dir, site, DATE, 8, quiet);
      const list = JSON.parse(readFileSync(join(dir, "build-assets.json"), "utf8"));
      if (result.carried !== 2 || Object.keys(list.files).length !== 5) fail(`room: a build with room for 2 carried ${result.carried}`);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
    if (failures === before) console.log(`ok   room: the newest first, at most ${MAX_CARRIED}, within the build's 20,000`);
  }

  // An Open Graph image's name is its bytes' hash; a script's isn't checked.
  {
    const before = failures;
    if (!intact(og("snake", "x"), Buffer.from("x")) || intact(og("snake", "x"), Buffer.from("y"))) fail("intact: an Open Graph image");
    if (!intact("assets/hub-ABC.js", Buffer.from("anything"))) fail("intact: a script");
    if (failures === before) console.log("ok   intact: an Open Graph image's bytes match its name");
  }

  // No live site, or no list on it (the first deploy with this): the build's own list, nothing carried, no
  // failure; and --no-carry (test.sh's) the same without asking.
  {
    const before = failures;
    for (const [what, from] of [["no list", `${site}/nothing`], ["no site", "http://127.0.0.1:9"], ["--no-carry", null]]) {
      const dir = mkdtempSync(join(tmpdir(), "test-carry-"));
      try {
        build(dir);
        const result = await carry(dir, from, DATE, 20_000, quiet);
        const list = JSON.parse(readFileSync(join(dir, "build-assets.json"), "utf8"));
        if (result.carried !== 0 || Object.keys(list.files).length !== 3) fail(`${what}: carried ${result.carried}, listed ${Object.keys(list.files)}`);
      } finally {
        rmSync(dir, { recursive: true, force: true });
      }
    }
    if (failures === before) console.log("ok   no live list: the build's own list, nothing carried");
  }
} finally {
  server.close();
}

console.log(failures === 0 ? "test-carry.mjs: all passed" : `test-carry.mjs: ${failures} failed`);
process.exit(failures === 0 ? 0 : 1);
