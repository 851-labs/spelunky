// "virtual:site/release": the native apps' downloads, for the /download page (src/routes/download.tsx), from the
// update manifest of the release the site puts live (docs/updates.md, "The manifest"): the latest.json build.sh
// downloads and checks against web-release.txt's SHA-256, and puts at /releases/latest.json. So the page lists what
// the apps update to, file for file, each with its size and SHA-256, and changes with web-release.txt: there's no
// list to keep by hand. build.sh names the file in SITE_RELEASE_MANIFEST; without one (`vite dev`, a release with
// no manifest) the page sends readers to GitHub's releases instead.
//
// Its signature isn't checked here: scripts/bump-web-release checked it before pinning the file's SHA-256, which
// build.sh checks (and site/test.sh's stand-in isn't signed).
//
// The manifest has only what the apps update to. The release's other downloads, the macOS .dmg and the Linux
// tarballs (851-4016), come from its SHA256SUMS, which web-release.txt pins too (sums-sha256, checked by build.sh,
// in SITE_RELEASE_SUMS), with the sizes the bump measured as it checked each file against it (web-release.txt's
// size lines, in SITE_RELEASE_SIZES): SHA256SUMS has none. The page lists the files the bump sized that aren't the
// manifest's. Not the manifest's own new key, which only a release from after it would have, and every installed
// app would download each day for files it never takes: SHA256SUMS is every release's, and its files' names.
import { readFileSync } from "node:fs";
import type { Plugin } from "vite";

export interface ReleaseAsset {
  // The manifest's platform: macos, windows-x86_64, linux-aarch64...
  platform: string;
  url: string;
  // The file's name, the URL's last part: spelunky-0.1.19-macos.zip.
  file: string;
  sha256: string;
  size: number;
  format: string;
}

// One of the release's other downloads, from its SHA256SUMS: spelunky-0.1.19-macos.dmg.
export interface ReleaseFile {
  url: string;
  file: string;
  sha256: string;
  size: number;
}

export interface Release {
  // The tag without its v: 0.1.19.
  version: string;
  // The release's page on GitHub, its notes.
  notes: string;
  assets: ReleaseAsset[];
  // The others, in SHA256SUMS's order (by name); none without one.
  files: ReleaseFile[];
}

const ID = "virtual:site/release";

export function siteRelease(
  file = process.env.SITE_RELEASE_MANIFEST || null,
  sums = process.env.SITE_RELEASE_SUMS || null,
  sizes = process.env.SITE_RELEASE_SIZES || null,
): Plugin {
  // Read once, when the build starts, so a bad manifest fails it before anything is rendered.
  let release: Release | null = null;
  return {
    name: "site-release",
    buildStart() {
      if ((sums === null) !== (sizes === null)) {
        throw new Error("SITE_RELEASE_SUMS and SITE_RELEASE_SIZES go together (src/build/release.ts)");
      }
      release = file ? parseRelease(readFileSync(file, "utf8"), file) : null;
      if (release && sums && sizes) {
        release.files = parseFiles(release, readFileSync(sums, "utf8"), readFileSync(sizes, "utf8"), sums, sizes);
      }
    },
    resolveId: (id) => (id === ID ? `\0${ID}` : undefined),
    load: (id) => (id === `\0${ID}` ? `export const release = ${JSON.stringify(release)};` : undefined),
  };
}

// latest.json: {"manifest": "<the manifest, as a string>", "signature": "..."}, the manifest
// {"version", "notes", "assets": {PLATFORM: {url, sha256, size, format}}}. Anything else fails the build, naming
// the file, rather than put a broken link or a wrong checksum on the page.
export function parseRelease(text: string, name = "latest.json"): Release {
  const fail = (what: string): never => {
    throw new Error(`${name}: ${what} (docs/updates.md, "The manifest")`);
  };
  const object = (value: unknown, what: string): Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : fail(`${what} isn't an object`);
  const string = (value: unknown, what: string): string =>
    typeof value === "string" && value !== "" ? value : fail(`${what} isn't a string`);
  const json = (source: string, what: string): unknown => {
    try {
      return JSON.parse(source);
    } catch (error) {
      return fail(`${what} isn't JSON: ${(error as Error).message}`);
    }
  };
  // The apps take only HTTPS (docs/updates.md), and the page links nothing else.
  const https = (value: unknown, what: string): URL => {
    const text = string(value, what);
    let url: URL | null = null;
    try {
      url = new URL(text);
    } catch {
      // Not a URL: refused below.
    }
    return url?.protocol === "https:" ? url : fail(`${what} ${text} isn't an HTTPS URL`);
  };
  const outer = object(json(text, "the file"), "the file");
  const manifest = object(json(string(outer.manifest, "its manifest"), "its manifest"), "its manifest");
  const version = string(manifest.version, "the version");
  // x.y.z, or a prerelease's x.y.z-pre, as the apps' versions are (crates/update).
  if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) fail(`the version ${version} isn't x.y.z`);
  const notes = https(manifest.notes, "the notes").href;
  const assets = Object.entries(object(manifest.assets, "the assets")).map(([platform, value]): ReleaseAsset => {
    const asset = object(value, platform);
    const parsed = https(asset.url, `${platform}'s url`);
    const url = parsed.href;
    // Release files are named with letters, digits and ._-+ only (docs/updates.md), which a URL's path has as is.
    const file = parsed.pathname.split("/").pop() ?? "";
    if (!/^[A-Za-z0-9._+-]+$/.test(file)) fail(`${platform}'s url ${url} names no file`);
    const sha256 = string(asset.sha256, `${platform}'s sha256`);
    if (!/^[0-9a-f]{64}$/.test(sha256)) fail(`${platform}'s sha256 ${sha256} isn't 64 hex digits`);
    const size = asset.size;
    if (typeof size !== "number" || !Number.isSafeInteger(size) || size <= 0) fail(`${platform}'s size isn't a size`);
    return { platform, url, file, sha256, size: size as number, format: string(asset.format, `${platform}'s format`) };
  });
  if (assets.length === 0) fail("it has no assets");
  return { version, notes, assets, files: [] };
}

// SHA256SUMS: a "HEX  NAME" line a file, as sha256sum writes it (" *NAME" in its binary mode), and the sizes'
// "size NAME BYTES" lines (web-release.txt's; its other lines aren't read). A size for a file SHA256SUMS hasn't, or
// a manifest file whose SHA-256 isn't SHA256SUMS's (pins of two releases), fails the build, as a bad manifest does.
export function parseFiles(
  release: Release,
  sums: string,
  sizes: string,
  sumsName = "SHA256SUMS",
  sizesName = "web-release.txt",
): ReleaseFile[] {
  const listed = new Map<string, string>();
  for (const line of sums.split("\n")) {
    if (line === "") continue;
    const match = /^([0-9a-f]{64}) [ *]([A-Za-z0-9._+-]+)$/.exec(line);
    if (!match) throw new Error(`${sumsName}: "${line}" isn't a SHA-256 and a release file's name`);
    if (listed.has(match[2])) throw new Error(`${sumsName}: ${match[2]} is there twice`);
    listed.set(match[2], match[1]);
  }
  for (const asset of release.assets) {
    if (listed.get(asset.file) !== asset.sha256) {
      throw new Error(`${sumsName}: the manifest's ${asset.file} isn't there with the manifest's SHA-256: another release's?`);
    }
  }
  const sized = new Map<string, number>();
  for (const line of sizes.split("\n")) {
    const words = line.trim().split(/\s+/);
    if (words[0] !== "size") continue;
    const [, file, bytes] = words;
    if (words.length !== 3 || !/^[1-9][0-9]*$/.test(bytes) || !Number.isSafeInteger(Number(bytes))) {
      throw new Error(`${sizesName}: "${line}" isn't "size NAME BYTES"`);
    }
    if (!listed.has(file)) throw new Error(`${sizesName}: ${file} has a size, but ${sumsName} hasn't it`);
    if (sized.has(file)) throw new Error(`${sizesName}: ${file} has two sizes`);
    sized.set(file, Number(bytes));
  }
  // In the manifest's release: its files' directory on GitHub (docs/updates.md, "each FILE ... under its own name").
  const base = release.assets[0].url.replace(/[^/]+$/, "");
  const manifest = new Set(release.assets.map((asset) => asset.file));
  return [...listed]
    .filter(([file]) => sized.has(file) && !manifest.has(file))
    .map(([file, sha256]) => ({ url: base + file, file, sha256, size: sized.get(file) as number }));
}
