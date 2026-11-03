// Gives the game's page, the web build's own index.html at /play/, the head every page of the site has
// (src/seo/play.ts), after its <title>, which it keeps: the release's copyright notice. A release needn't change.
//
//   node play-head.mjs PAGE    rewrite PAGE (build.sh: dist/client/play/index.html)
import { readFileSync, writeFileSync } from "node:fs";
import { playHead, playHeadHtml, unescapeHtml } from "./src/seo/play.ts";

const [page] = process.argv.slice(2);
if (!page) {
  console.error("usage: node play-head.mjs PAGE");
  process.exit(2);
}
const die = (message) => {
  console.error(`play-head.mjs: ${page} ${message}`);
  process.exit(1);
};
const html = readFileSync(page, "utf8");
const titles = [...html.matchAll(/<title>([^<]*)<\/title>/g)];
if (titles.length !== 1) die(`has ${titles.length} <title>s, not 1, for the head to follow`);
// A release that has some of the head already would have it twice: the page should then give it up to this, or
// this to the page.
for (const [what, tag] of [
  ["a viewport", /<meta name="viewport"/],
  ["a description", /<meta name="description"/],
  ["Open Graph's or Twitter's tags", /<meta (?:property="og:|name="twitter:)/],
  ["a <link rel> of the head's", /<link rel="(?:canonical|search|icon|apple-touch-icon)"/],
]) {
  if (tag.test(html)) die(`has ${what} already`);
}
const [title] = titles;
const at = title.index + title[0].length;
const head = playHeadHtml(playHead(unescapeHtml(title[1])));
writeFileSync(page, `${html.slice(0, at)}\n${head}${html.slice(at)}`);
console.log(`play-head.mjs: gave ${page} the site's head`);
