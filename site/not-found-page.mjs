// Leaves the 404 page (src/routes/404.tsx, pre-rendered to 404.html) without scripts. The Worker serves it at any
// path that isn't a page (src/server.ts), where the router would hydrate as another page, so the root leaves out
// its scripts (src/routes/__root.tsx); but the pre-render still names the route's chunks to preload, and writes
// the router's state in an inline script for them. Neither would run anything: this takes them out, so a miss
// downloads no JS. Before csp.mjs, so the page's policy names no script.
//
//   node not-found-page.mjs PAGE    rewrite PAGE (build.sh: dist/client/404.html)
import { readFileSync, writeFileSync } from "node:fs";

const [page] = process.argv.slice(2);
if (!page) {
  console.error("usage: node not-found-page.mjs PAGE");
  process.exit(2);
}
const html = readFileSync(page, "utf8");
const preloads = /<link rel="modulepreload"[^>]*\/>/g;
const scripts = /<script\b[^>]*>[\s\S]*?<\/script>/g;
const removed = (html.match(preloads) ?? []).length + (html.match(scripts) ?? []).length;
const stripped = html.replace(preloads, "").replace(scripts, "");
if (/<script\b|modulepreload/i.test(stripped)) {
  console.error(`not-found-page.mjs: ${page} still has a script or a preload`);
  process.exit(1);
}
if (!stripped.includes('<header class="site-header">')) {
  console.error(`not-found-page.mjs: ${page} hasn't the site's header: is it the 404 route's?`);
  process.exit(1);
}
writeFileSync(page, stripped);
console.log(`not-found-page.mjs: took ${removed} scripts and preloads out of ${page}`);
