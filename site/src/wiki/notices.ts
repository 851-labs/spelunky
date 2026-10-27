// The license's notices for the wiki (docs/wiki-license.md, "Notice text for the templates"), in one place:
// the pages' footer (layout.tsx) and the files read without it (the Markdown twins, /llms.txt and
// /llms-full.txt: build/markdown.ts) say the same. No dependencies, since the pages' layout imports it as
// well as the build.

// The footer's notice, around its link to the license.
export const NOTICE = {
  before: "Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC, and is used under the ",
  license: "Spelunky User License v1.1b",
  after:
    ". spelunky.ai is an unofficial fan project, not affiliated with or endorsed by Mossmouth. The game and its " +
    "assets may not be sold.",
};

// After the notice on a page whose clips have the game's sound.
export const AUDIO_CREDIT =
  "Music by George Buzinkai and Jonathan Perry; sound effects made in SFXR. From Spelunky Classic 1.1, © 2008, " +
  "2009 Derek Yu and Mossmouth, LLC.";
