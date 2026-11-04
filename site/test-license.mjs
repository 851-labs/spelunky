// The /license page's reading of the repo's LICENSE (src/build/license.ts), which site/test.sh runs, as does a
// change to it locally: the real file, read whole into its title, header, sections and conditions; and the files
// it refuses, rather than put a license with a part missing on the page.
//
//   node test-license.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { parseLicense } from "./src/build/license.ts";

let failures = 0;
function test(name, run) {
  try {
    run();
    console.log(`ok   ${name}`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${name}\n${error.stack ?? error}`);
  }
}

const LICENSE = new TextDecoder("windows-1252").decode(readFileSync(new URL("../LICENSE", import.meta.url)));

test("the repo's LICENSE: its title, header and five sections, the conditions a list of four", () => {
  const license = parseLicense(LICENSE);
  assert.equal(license.title, "Spelunky® User License v1.1b");
  assert.deepEqual(license.header, [
    "Copyright (c) 2008, 2009 Derek Yu and Mossmouth, LLC",
    "Last Updated on December 25th, 2009",
  ]);
  assert.deepEqual(
    license.sections.map((section) => section.heading),
    ["1. Preamble", "2. Definitions", "3. Rights Granted", "4. Disclaimer of Warranty", "5. Limitation of Liability"],
  );
  const rights = license.sections[2].blocks;
  assert.deepEqual(
    rights.map((block) => block.kind),
    ["paragraph", "paragraph", "conditions", "paragraph"],
  );
  const conditions = rights[2].kind === "conditions" ? rights[2].items : [];
  assert.deepEqual(
    conditions.map((item) => item.marker),
    ["(1)", "(2)", "(3)", "(4)"],
  );
  // A condition's lines are one paragraph, its indents gone.
  assert.equal(
    conditions[2].text,
    "You must display an appropriate copyright notice somewhere in the beginning of all Covered Works and in any " +
      "documentation accompanying such Covered Works.",
  );
  assert.equal(license.sections[1].blocks.length, 4);
});

test("Windows line endings read the same", () => {
  assert.deepEqual(parseLicense(LICENSE.replace(/\n/g, "\r\n")), parseLicense(LICENSE));
});

const refuses = (name, text, message) =>
  test(`refuses ${name}`, () => assert.throws(() => parseLicense(text, "LICENSE"), message));
refuses("a file without its title between rules", LICENSE.replace(/^=+\n/, ""), /no title between two rules/);
refuses("a file without sections", "===\nTitle\n===\n\nJust a line.\n", /no sections/);
refuses("a stray rule", LICENSE.replace("Happy Spelunky-ing!", "Happy Spelunky-ing!\n-----"), /a rule that's no heading's/);
refuses("a condition before the first section", "===\nTitle\n===\n\n(1) First.\n\n---\n1. A\n---\n\nText.\n", /a condition, \(1\), before/);

if (failures > 0) {
  console.error(`site/test-license.mjs: ${failures} failed`);
  process.exit(1);
}
console.log("site/test-license.mjs: all passed");
