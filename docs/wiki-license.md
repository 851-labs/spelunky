# What the wiki may show

The spelunky.ai wiki (851-3329) shows the game's sprites, clips from the port, numbers and behaviour taken
from the code, short GML excerpts, and the GML itself, whole, for its citations to link to. This is our reading of the [Spelunky User License v1.1b](../LICENSE),
not legal advice: where it's unclear we take the conservative option and list the question at the end.

## The license, in short

It lets anyone modify and redistribute the **Game** (the program "and all related files", so the released
game's `sound/` folder too) and the **Source Code** (the `.gmk` files "and all of the objects, scripts,
sprites, and tiles" in them). A modified version is a **Covered Work**: the port is one, and so is anything
that copies the game's assets or code, such as a sprite strip, a clip or a GML excerpt. Four conditions:

1. Keep the license and the source's copyright notices intact: `/LICENSE.txt` is served unchanged from the
   repo (`docs/web-deploy.md`).
2. Don't sell Covered Works.
3. Show an appropriate copyright notice at the beginning of every Covered Work and of its documentation.
4. Don't present a modified version as unmodified or official: make it clear it's a mod.

## What the wiki may show

| What | May show? | Conditions |
| --- | --- | --- |
| Sprites, tiles, animation strips (from `assets/`, extracted from the `.gmk`) | Yes, scaled or recoloured too | The page notices below. Downloadable files (the `wiki-media` release asset) ship with `LICENSE` and a notice, as the releases do (`scripts/release/NOTICE.txt`). |
| Gameplay clips recorded from the port | Yes | The page's footer, and the notice in the video files' metadata. No caption under clips and no notice burned into the picture (decided below). Music and sound effects may play, with the audio credit below when they do. |
| Sound effects (`sound/*.wav`) | Yes, in clips or a page's small player | The page notices, plus the audio credit below |
| Music (`sound/*.ogg`) | Yes (decided below) | The audio credit below |
| Numbers and behaviour derived from the code (speeds, damage, drop rates, quirks) | Yes | Facts aren't the work itself; cite the source as for GML. |
| Short GML excerpts | Yes | The excerpt caption below. Keep them short and only where they explain something, and keep any credit or copyright comment of the script's (condition 1). |
| Whole GML files (`/wiki/source/`, the source viewer) | Yes ([below](#the-source-viewer)) | The source notice at the top of every page, above the code; the file whole and unchanged, its comments too. The `wiki-media` package that carries them ships with `LICENSE` and the notice. |
| Generated OG images with sprites | Yes | Our own layout and the text "Unofficial fan wiki · spelunky.ai"; the Spelunky logo may appear (decided below) |
| JSON-LD | Yes | `VideoGame` names Derek Yu and Mossmouth as its author and publisher; our `WebSite` and `Organization` are ours, never Mossmouth's. Media get `creditText`, `copyrightNotice` and `license` (`https://spelunky.ai/LICENSE.txt`); a clip with sound's `creditText` names the music's authors too. |

## Notice text for the templates

**Masthead** (every wiki page, at the top): the Spelunky logo with a "Fan wiki" badge. No copyright line
there (decided below): the notice is the footer's.

**Site-wide wiki footer** (every page):

> Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC, and is used under the
> [Spelunky User License v1.1b](/LICENSE.txt). spelunky.ai is an unofficial fan project, not affiliated with
> or endorsed by Mossmouth. The game and its assets may not be sold.

**Clips** carry no caption of their own (decided below): the page's footer covers them.
The video files keep the notice in their metadata (`copyright`, and `comment`: "Recorded from the
spelunky.ai port vX.Y.Z, an unofficial port (a mod) of Spelunky Classic 1.1. Spelunky © 2008, 2009 Derek
Yu and Mossmouth, LLC."). Clips play the game's sound and music; a page with a clip that has them
(`has_audio`, docs/wiki-media.md) adds the audio credit below to its footer, and the video files have it
in `copyright` after the notice.

**GML excerpt caption**:

> From `spelunky.gmk`, Spelunky Classic 1.1's source, © 2008, 2009 Derek Yu and Mossmouth, LLC
> ([Spelunky User License v1.1b](/LICENSE.txt)).

A few of the source's scripts are others' work that Derek Yu credits in the original `readme.txt`: Martin
Piecyk's platform engine, ChevyRay's view scaling scripts and tsg1zzn's Super Sound System. Name the author
when quoting one of them.

**Source notice** (at the top of every `/wiki/source/` page, above the code):

> From `spelunky.gmk`, Spelunky Classic 1.1's source, © 2008, 2009 Derek Yu and Mossmouth, LLC, shown whole
> under the [Spelunky User License v1.1b](/LICENSE.txt). Extracted by spelunky.ai's tools, an unofficial
> fan project, one file per script or event; the `// ====` lines are the extractor's, naming each action of
> the event. Some scripts are others' work that Derek Yu credits in the source's `readme.txt`: Martin
> Piecyk's platform engine, ChevyRay's view scaling scripts and tsg1zzn's Super Sound System.

A Super Sound System script (`SS_*`) names tsg1zzn in its own line too.

**Audio credit** (in the footer of pages that play the game's audio): "Music by George Buzinkai and
Jonathan Perry; sound effects made in SFXR. From Spelunky Classic 1.1, © 2008, 2009 Derek Yu and
Mossmouth, LLC."

**Per-page notice**: none beyond the footer on HTML pages. Files read without them (the
`.md` twins, `/llms.txt`, `/llms-full.txt`) start with the footer's text, its link made absolute
(`https://spelunky.ai/LICENSE.txt`).

## The source viewer

The source viewer shows each of the game's scripts and object events whole, one page each (851-3382),
so that a page's citation can open the lines it cites. That is redistributing the **Source Code**, which the
license names as the `.gmk` files "and all of the objects, scripts, sprites, and tiles contained within
those files", and its section 3 grants the right to redistribute it, so the whole files are covered,
not only short excerpts. The repository, which is public, already redistributes the whole of
`spelunky.gmk` (`reference/spelunky_1_1_src.zip`) on the same terms. The conditions, as they apply:

1. **The license and the source's notices intact.** Every page links `/LICENSE.txt`, served unchanged, and
   shows the file whole, the scripts' own credit and copyright comments with it; the package the pages are
   built from (`gml/` in `wiki-media`, docs/wiki-media.md) has `LICENSE.txt` and `NOTICE.txt` beside it.
2. **Not sold.** As the rest of the wiki: no ads, paywalls or sponsorships.
3. **A notice at the beginning.** The source notice above, at the top of each page before the code, besides
   the footer's: unlike a wiki page, a source page is the Source Code itself, so the footer alone isn't
   enough.
4. **Not presented as official or unmodified.** The files are the extractor's arrangement of the `.gmk`
   (one per script or event, with its `// ====` action lines), so the notice says so, and that the
   extraction is an unofficial fan project's.

The pages are `noindex` (docs/wiki-media.md, "Source"): they're the game's code, not the wiki's
writing, and they're there for the pages' citations, not to rank in search.

## Don'ts

- **No selling, paywalls, ads or sponsorships.** The license only says Covered Works may not be sold. Ads
  or paid tiers on pages built around the game's assets earn money from them, so the conservative reading
  treats them as selling. No donation buttons or affiliate links either (decided below).
- **Don't call it the official wiki** or imply Mossmouth runs or endorses it. The Spelunky logo may head the
  wiki (decided below), always with the "Fan wiki" badge beside it; Mossmouth's own marks
  (its studio logo and name as ours) stay off.
- **No Spelunky HD, Spelunky 2 or other Mossmouth assets.** Those games aren't under this license. The same
  goes for text and images copied from other fan wikis (Fandom's are CC BY-SA, a different license).

## Trademark

"Spelunky" is Mossmouth's name for its games, so we use it only to say what the wiki is about, e.g. "a
wiki for Spelunky Classic" or "Spelunky Classic fan wiki", always with "unofficial" or "fan" nearby. Page
titles follow that, like "Shotgun – Spelunky Classic fan wiki".

## Decisions

The user settled the open questions on 2026-10-27:

1. **Music: allowed.** The releases and `/play/` keep shipping the original's `.ogg` files, and clips and
   pages may play the music, with the audio credit above (George Buzinkai and Jonathan Perry).
2. **Donations and affiliate links: none.** Not needed, and they'd sit badly with "may not be sold".
3. **The Spelunky logo: allowed,** including as the wiki's masthead, next to the "Unofficial fan wiki" header
   line (now the "Fan wiki" badge, decision 5). The domain stays spelunky.ai.
4. **No notice burned into clips, and no caption under them.** Every page that shows a clip already
   carries the notice in its footer, so the clips are clean from their first frame and
   show only their title and description. The notice stays in the video files' metadata and the poster's
   `Copyright` chunk. A page whose clips have sound adds the audio credit to its footer.
5. **No copyright line in the header.** The footer of every page carries the full notice and the license
   link, and the masthead's "Fan wiki" badge marks the site as unofficial, so the header shows only the
   logo, the badge and search. A wiki page isn't a Covered Work itself (the game at `/play/` is, and shows
   its own notice at start), so the footer is enough.
