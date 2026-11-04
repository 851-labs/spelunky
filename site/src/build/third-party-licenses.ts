// The third-party software in the pages' scripts, with its licenses: /third-party-licenses.txt (docs/web-deploy.md,
// "Third-party licenses"). The client's bundle is React, Base UI, TanStack Router and the rest, MIT-licensed, whose
// notices the minifier strips, and the MIT license asks that they go with every copy. So the build writes every
// package whose code is in the bundle, by what the bundle holds (not what package.json lists, which has the
// build's tools and the Worker's too), with its license file as it ships it; and fails on a package without one,
// rather than ship it without its notice. Only the client's: the Worker's runs on Cloudflare, and isn't
// distributed.
//
// Vite 8's own `build.license` does most of this, but leaves out the bundlers' runtime code it puts in the bundle
// (its preload helper, Rolldown's runtime: virtual modules), and lists a package without a license file as if
// that were fine.
import { readdirSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import type { Plugin } from "vite";

const LICENSES_FILE = "third-party-licenses.txt";
const NODE_MODULES = "/node_modules/";

interface Package {
  name: string;
  version: string;
  license: string;
  text: string;
}

// The package a module of the bundle is in: the directory under its last node_modules/ (a scoped name's two).
// Bundlers' ids use forward slashes on every platform, and a plugin's virtual id for a real file may start with a
// NUL.
function packageDir(id: string): string | null {
  const path = id.replace(/^\0/, "").split("?")[0].replaceAll("\\", "/");
  const at = path.lastIndexOf(NODE_MODULES);
  if (at === -1) return null;
  const rest = path.slice(at + NODE_MODULES.length).split("/");
  const parts = rest[0].startsWith("@") ? rest.slice(0, 2) : rest.slice(0, 1);
  return path.slice(0, at + NODE_MODULES.length) + parts.join("/");
}

// The bundlers' own runtime code in the bundle, by its virtual modules' ids, and the directory of the package it's
// from: the Vite this build runs, and the Rolldown it runs with.
function runtimes(): [RegExp, string][] {
  const fromHere = createRequire(import.meta.url);
  const vite = dirname(fromHere.resolve("vite/package.json"));
  const rolldown = dirname(createRequire(join(vite, "package.json")).resolve("rolldown/package.json"));
  return [
    [/^\0vite\//, vite],
    [/^\0rolldown\//, rolldown],
  ];
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

function licensesText(packages: Package[]): string {
  const order = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);
  const sorted = [...packages].sort((a, b) => order(a.name, b.name) || order(a.version, b.version));
  const head = [
    "Third-party software in spelunky.ai's pages",
    "",
    "The scripts the site's pages load include the following packages, each under its own license, as follows.",
    "The game at /play/ and Spelunky itself are covered by https://spelunky.ai/license and the game's own notices.",
    "",
    ...sorted.map((p) => `- ${p.name} ${p.version} (${p.license})`),
  ];
  const rule = "=".repeat(78);
  const bodies = sorted.map((p) => [rule, `${p.name} ${p.version} (${p.license})`, rule, "", p.text].join("\n"));
  return `${[head.join("\n"), ...bodies].join("\n\n")}\n`;
}

export function thirdPartyLicenses(): Plugin {
  return {
    name: "third-party-licenses",
    applyToEnvironment: (environment) => environment.name === "client",
    generateBundle(_options, bundle) {
      const runtime = runtimes();
      const dirs = new Set<string>();
      for (const output of Object.values(bundle)) {
        if (output.type !== "chunk") continue;
        for (const id of output.moduleIds) {
          const dir = runtime.find(([pattern]) => pattern.test(id))?.[1] ?? packageDir(id);
          if (dir !== null) dirs.add(dir);
        }
      }
      if (dirs.size === 0) this.error("the client's bundle has no third-party package, which can't be right");
      // Two directories of one package at one version (a copy nested elsewhere) are one entry.
      const packages = new Map([...dirs].map(readPackage).map((p) => [`${p.name}@${p.version}`, p]));
      this.emitFile({ type: "asset", fileName: LICENSES_FILE, source: licensesText([...packages.values()]) });
    },
  };
}
