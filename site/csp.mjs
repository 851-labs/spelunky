// Every page's Content-Security-Policy, as a <meta http-equiv> at the top of its <head> (docs/web-deploy.md,
// "Security headers"). The pages are pre-rendered, and each has inline scripts of its own (TanStack Start's
// hydration data, the source viewer's highlighting), so the policy names each by its hash: no other inline script
// runs, nor any script from another site but Cloudflare Web Analytics' beacon, which Cloudflare injects into the
// pages as it serves them. A header can't carry this, as the asset store's _headers are the same for every page and
// the Worker would have to read every page to hash its scripts. object-src, base-uri and frame-ancestors (which a
// <meta> can't carry) are the header's, which every response has (src/server.ts, public/_headers).
//
//   node csp.mjs write DIR    give every .html under DIR (dist/client) its policy
//   node csp.mjs check DIR    fail unless every one has it, naming exactly its inline scripts
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

// Cloudflare Web Analytics, injected into the pages for browsers (a <script src> from static.cloudflareinsights.com,
// with its own integrity hash), which reports to cloudflareinsights.com.
const BEACON_SCRIPT = "https://static.cloudflareinsights.com";
const BEACON_REPORT = "https://cloudflareinsights.com";
// The game's page (the web build's index.html at /play/) compiles its WebAssembly.
const GAME_PAGE = join("play", "index.html");

// The policy for a page whose inline scripts hash to `hashes`.
export function policy(hashes, { game = false } = {}) {
  const scripts = ["'self'", ...hashes.map((h) => `'${h}'`), ...(game ? ["'wasm-unsafe-eval'"] : []), BEACON_SCRIPT];
  return [
    "default-src 'self'",
    `script-src ${scripts.join(" ")}`,
    // React and the pages' markup set style attributes, and the landing page and the game's have a <style>.
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data:",
    `connect-src 'self' ${BEACON_REPORT}`,
    "form-action 'self'",
  ].join("; ");
}

// The page's inline scripts' hashes, in the order they come: the scripts a browser runs, so not JSON-LD, which is
// data, nor one with a src.
export function inlineScriptHashes(html) {
  const hashes = [];
  for (const [, attributes, body] of html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)) {
    if (/\ssrc=/i.test(attributes)) continue;
    const type = /\stype=["']?([^"'\s>]+)/i.exec(attributes)?.[1].toLowerCase();
    if (type !== undefined && type !== "module" && type !== "text/javascript") continue;
    hashes.push(`sha256-${createHash("sha256").update(body, "utf8").digest("base64")}`);
  }
  return [...new Set(hashes)];
}

const META = /<meta http-equiv="Content-Security-Policy" content="([^"]*)">/g;

function pages(dir, sub = "") {
  return readdirSync(join(dir, sub), { withFileTypes: true }).flatMap((entry) => {
    const path = join(sub, entry.name);
    if (entry.isDirectory()) return pages(dir, path);
    return entry.name.endsWith(".html") ? [path] : [];
  });
}

const escape = (text) => text.replaceAll("&", "&amp;").replaceAll('"', "&quot;");

// The page with its policy first in its <head>, before any script: after its <meta charset> (which must be in the
// first 1024 bytes), or else its <head>, or else its doctype (the game's page has no <head> tag).
function withPolicy(html, path) {
  const meta = `<meta http-equiv="Content-Security-Policy" content="${escape(policy(inlineScriptHashes(html), { game: path === GAME_PAGE }))}">`;
  for (const after of [/<meta charset=[^>]*>/i, /<head\b[^>]*>/i, /<!doctype html>/i]) {
    const match = after.exec(html);
    if (match) return html.slice(0, match.index + match[0].length) + meta + html.slice(match.index + match[0].length);
  }
  throw new Error(`${path}: no <meta charset>, <head> or doctype to put its Content-Security-Policy after`);
}

function main([mode, dir]) {
  if (!["write", "check"].includes(mode) || !dir) {
    console.error("usage: node csp.mjs write|check DIR");
    process.exit(2);
  }
  const problems = [];
  const all = pages(dir);
  for (const path of all) {
    const file = join(dir, path);
    const html = readFileSync(file, "utf8");
    const metas = [...html.matchAll(META)];
    if (mode === "write") {
      if (metas.length > 0) throw new Error(`${path} has a Content-Security-Policy already`);
      writeFileSync(file, withPolicy(html, path));
      continue;
    }
    if (metas.length !== 1) {
      problems.push(`${path}: ${metas.length} Content-Security-Policy metas, not 1`);
      continue;
    }
    const firstScript = html.search(/<script\b/i);
    if (firstScript !== -1 && firstScript < metas[0].index) problems.push(`${path}: a script before its policy`);
    // The policy without itself: what write would have made of the page.
    const without = html.slice(0, metas[0].index) + html.slice(metas[0].index + metas[0][0].length);
    const want = escape(policy(inlineScriptHashes(without), { game: path === GAME_PAGE }));
    if (metas[0][1] !== want) problems.push(`${path}: its policy isn't its scripts': ${metas[0][1]}, not ${want}`);
  }
  if (problems.length > 0) {
    console.error(problems.join("\n"));
    console.error(`csp.mjs: ${problems.length} of ${all.length} pages wrong`);
    process.exit(1);
  }
  console.log(`csp.mjs: ${mode === "write" ? "gave" : "checked"} ${all.length} pages' Content-Security-Policy`);
}

if (import.meta.url === `file://${process.argv[1]}`) main(process.argv.slice(2));
