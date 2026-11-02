// The third-party software in the pages' scripts, with its licenses: /third-party-licenses.txt (docs/web-deploy.md,
// "Third-party licenses"). The client's bundle is React, Base UI, TanStack Router and the rest, MIT-licensed, whose
// notices the minifier strips, and the MIT license asks that they go with every copy. So the build writes every
// package whose code is in the bundle, by what the bundle holds (not what package.json lists, which has the
// build's tools and the Worker's too), with its license file as it ships it; and fails on a package without one,
// rather than ship it without its notice. Only the client's: the Worker's runs on Cloudflare, and isn't
// distributed.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, sep } from "node:path";
import type { Plugin } from "vite";

export const LICENSES_FILE = "third-party-licenses.txt";

// The bundlers' own runtime code that they put in the bundle, by the virtual modules' ids, and the package it's from.
const RUNTIMES: [RegExp, string][] = [
  [/^\0vite\//, "vite"],
  [/^\0rolldown\//, "rolldown"],
];

interface Package {
  name: string;
  version: string;
  license: string;
  text: string;
}

// The package a module of the bundle is in: the directory under its last node_modules/ (a scoped name's two).
export function packageDir(id: string): string | null {
  const path = id.split("?")[0];
  const at = path.lastIndexOf(`${sep}node_modules${sep}`);
  if (at === -1) return null;
  const rest = path.slice(at + `${sep}node_modules${sep}`.length).split(sep);
  const parts = rest[0].startsWith("@") ? rest.slice(0, 2) : rest.slice(0, 1);
  return path.slice(0, at + `${sep}node_modules${sep}`.length) + parts.join(sep);
}

function readPackage(dir: string): Package {
  const meta = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
  const file = readdirSync(dir).find((name) => /^(licen[cs]e|copying)(\.(md|txt))?$/i.test(name));
  if (file === undefined) {
    throw new Error(`${meta.name}: no license file in ${dir}, so its notice can't go with the pages' scripts`);
  }
  let text = readFileSync(join(dir, file), "utf8");
  // Vite's license file goes on with the licenses of the dependencies it bundles into its own build tool, none of
  // which is in the pages' scripts (only its preload helper is): its own license, then.
  const bundled = /^#+ Licenses of bundled dependencies/m.exec(text);
  if (bundled) text = text.slice(0, bundled.index);
  return { name: meta.name, version: meta.version, license: meta.license ?? "?", text: text.trim() };
}

export function licensesText(packages: Package[]): string {
  const sorted = [...packages].sort((a, b) => (a.name < b.name ? -1 : 1));
  const head = [
    "Third-party software in spelunky.ai's pages",
    "",
    "The scripts the site's pages load include the following packages, each under its own license, as follows.",
    "The game at /play/ and Spelunky itself are covered by https://spelunky.ai/LICENSE.txt and the game's own notices.",
    "",
    ...sorted.map((p) => `- ${p.name} ${p.version} (${p.license})`),
  ];
  const bodies = sorted.map((p) => [`${"=".repeat(78)}`, `${p.name} ${p.version} (${p.license})`, `${"=".repeat(78)}`, "", p.text].join("\n"));
  return `${[head.join("\n"), ...bodies].join("\n\n")}\n`;
}

export function thirdPartyLicenses(): Plugin {
  return {
    name: "third-party-licenses",
    applyToEnvironment: (environment) => environment.name === "client",
    generateBundle(_options, bundle) {
      const dirs = new Set<string>();
      for (const output of Object.values(bundle)) {
        if (output.type !== "chunk") continue;
        for (const id of output.moduleIds) {
          const runtime = RUNTIMES.find(([pattern]) => pattern.test(id));
          const dir = runtime ? packageDir(require_resolve(runtime[1])) : packageDir(id);
          if (dir !== null) dirs.add(dir);
        }
      }
      if (dirs.size === 0) this.error("the client's bundle has no third-party package, which can't be right");
      const packages = [...dirs].map(readPackage);
      this.emitFile({ type: "asset", fileName: LICENSES_FILE, source: licensesText(packages) });
    },
  };
}

// The directory a package is installed in, for the bundlers' runtimes (whose modules are virtual).
function require_resolve(name: string): string {
  let dir = dirname(new URL(import.meta.url).pathname);
  for (;;) {
    const candidate = join(dir, "node_modules", name, "package.json");
    if (existsSync(candidate)) return candidate;
    const up = dirname(dir);
    if (up === dir) throw new Error(`${name} isn't installed`);
    dir = up;
  }
}
