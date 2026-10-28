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

// At the top of every source viewer page, above the code (docs/wiki-license.md, "Source notice"): a page that
// shows the Source Code whole is a Covered Work, so its notice comes first, not only in the footer. Around its
// link to the license.
export const SOURCE_NOTICE = {
  before: "From spelunky.gmk, Spelunky Classic 1.1's source, © 2008, 2009 Derek Yu and Mossmouth, LLC, shown whole under the ",
  license: "Spelunky User License v1.1b",
  after: ". Extracted by spelunky.ai's tools, an unofficial fan project, one file per script or event; the ",
  // In <code>, as are the file names.
  actionLines: "// ====",
  end:
    " lines are the extractor's, naming each action of the event. Some scripts are others' work that Derek Yu " +
    "credits in the source's readme.txt: Martin Piecyk's platform engine, ChevyRay's view scaling scripts and " +
    "tsg1zzn's Super Sound System.",
};

// After it on a Super Sound System script's page (SS_*), which names its author as an excerpt of it would.
export const SUPER_SOUND_CREDIT = "This script is part of tsg1zzn's Super Sound System.";
