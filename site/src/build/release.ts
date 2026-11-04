// "virtual:site/release": the native apps' downloads, for the /download page (src/routes/download.tsx), from the
// update manifest of the release the site puts live (docs/updates.md, "The manifest"): the latest.json build.sh
// downloads and checks against web-release.txt's SHA-256, and puts at /releases/latest.json. So the page lists what
// the apps update to, file for file, each with its size and SHA-256, and changes with web-release.txt: there's no
// list to keep by hand. build.sh names the file in SITE_RELEASE_MANIFEST; without one (`vite dev`, a release with
// no manifest) the page sends readers to GitHub's releases instead.
//
// Its signature isn't checked here: scripts/bump-web-release checked it before pinning the file's SHA-256, which
// build.sh checks (and site/test.sh's stand-in isn't signed).
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

export interface Release {
  // The tag without its v: 0.1.19.
  version: string;
  // The release's page on GitHub, its notes.
  notes: string;
  assets: ReleaseAsset[];
}

const ID = "virtual:site/release";

export function siteRelease(file = process.env.SITE_RELEASE_MANIFEST || null): Plugin {
  // Read once, when the build starts, so a bad manifest fails it before anything is rendered.
  let release: Release | null = null;
  return {
    name: "site-release",
    buildStart() {
      release = file ? parseRelease(readFileSync(file, "utf8"), file) : null;
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
  const outer = object(JSON.parse(text), "the file");
  const manifest = object(JSON.parse(string(outer.manifest, "its manifest")), "its manifest");
  const version = string(manifest.version, "the version");
  if (!/^\d+\.\d+\.\d+$/.test(version)) fail(`the version ${version} isn't x.y.z`);
  const notes = string(manifest.notes, "the notes");
  const assets = Object.entries(object(manifest.assets, "the assets")).map(([platform, value]): ReleaseAsset => {
    const asset = object(value, platform);
    const url = string(asset.url, `${platform}'s url`);
    // The apps take only HTTPS (docs/updates.md), and the page links nothing else.
    if (!url.startsWith("https://")) fail(`${platform}'s url ${url} isn't HTTPS`);
    const file = decodeURIComponent(new URL(url).pathname.split("/").pop() ?? "");
    if (!/^[A-Za-z0-9._+-]+$/.test(file)) fail(`${platform}'s url ${url} names no file`);
    const sha256 = string(asset.sha256, `${platform}'s sha256`);
    if (!/^[0-9a-f]{64}$/.test(sha256)) fail(`${platform}'s sha256 ${sha256} isn't 64 hex digits`);
    const size = asset.size;
    if (typeof size !== "number" || !Number.isSafeInteger(size) || size <= 0) fail(`${platform}'s size isn't a size`);
    return { platform, url, file, sha256, size: size as number, format: string(asset.format, `${platform}'s format`) };
  });
  if (assets.length === 0) fail("it has no assets");
  return { version, notes, assets };
}
