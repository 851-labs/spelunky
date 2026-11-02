// Gives the game's page, the web build's own index.html at /play/, the head every page of the site has
// (src/seo/play.ts): in place of the release's <title>, so a release needn't change.
//
//   node play-head.mjs PAGE    rewrite PAGE (build.sh: dist/client/play/index.html)
import { readFileSync, writeFileSync } from "node:fs";
import { playHeadHtml } from "./src/seo/play.ts";

const [page] = process.argv.slice(2);
if (!page) {
  console.error("usage: node play-head.mjs PAGE");
  process.exit(2);
}
const html = readFileSync(page, "utf8");
const titles = html.match(/<title>[^<]*<\/title>/g) ?? [];
if (titles.length !== 1) {
  console.error(`play-head.mjs: ${page} has ${titles.length} <title>s, not 1, for the head to replace`);
  process.exit(1);
}
writeFileSync(page, html.replace(titles[0], () => playHeadHtml()));
console.log(`play-head.mjs: gave ${page} the site's head`);
