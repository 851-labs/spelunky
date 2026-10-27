# What the wiki may show

The spelunky.ai wiki (851-3329) shows the game's sprites, clips from the port, numbers and behaviour taken
from the code, and maybe short GML. This is our reading of the [Spelunky User License v1.1b](../LICENSE),
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
| Gameplay clips recorded from the port | Yes | The clip caption below, and the app's notice (`crates/app/src/notice.rs`) burned into each clip's first seconds by the clip renderer itself: the app only draws it at the start of a session. No music until open question 1 is settled; sound effects are fine. |
| Sound effects (`sound/*.wav`) | Yes, in clips or a page's small player | The page notices, plus the audio credit below |
| Music (`sound/*.ogg`) | Not yet (open question 1) | If allowed: the audio credit below |
| Numbers and behaviour derived from the code (speeds, damage, drop rates, quirks) | Yes | Facts aren't the work itself; cite the source as for GML. |
| Short GML excerpts | Yes | The excerpt caption below. Keep them short and only where they explain something, and keep any credit or copyright comment of the script's (condition 1). |
| Generated OG images with sprites | Yes | Our own layout and the text "Unofficial fan wiki · spelunky.ai"; never the Spelunky logo as the main element |
| JSON-LD | Yes | `VideoGame` names Derek Yu and Mossmouth as its author and publisher; our `WebSite` and `Organization` are ours, never Mossmouth's. Media get `creditText`, `copyrightNotice` and `license` (`https://spelunky.ai/LICENSE.txt`). |

## Notice text for the templates

**Site-wide header line** (every page, at the top, as on the landing page): "Unofficial fan wiki.
Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC."

**Site-wide wiki footer** (every page):

> Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC, and is used under the
> [Spelunky User License v1.1b](/LICENSE.txt). spelunky.ai is an unofficial fan project, not affiliated with
> or endorsed by Mossmouth. The game and its assets may not be sold.

**Clip caption** (under every clip; `vX.Y.Z` is the port's release, `docs/releasing.md`):

> Recorded from the spelunky.ai port vX.Y.Z, an unofficial port (a mod) of Spelunky Classic 1.1.
> Spelunky © 2008, 2009 Derek Yu and Mossmouth, LLC.

**GML excerpt caption**:

> From `spelunky.gmk`, Spelunky Classic 1.1's source, © 2008, 2009 Derek Yu and Mossmouth, LLC
> ([Spelunky User License v1.1b](/LICENSE.txt)).

A few of the source's scripts are others' work that Derek Yu credits in the original `readme.txt`: Martin
Piecyk's platform engine, ChevyRay's view scaling scripts and tsg1zzn's Super Sound System. Name the author
when quoting one of them.

**Audio credit** (pages that play the game's audio): "Music by George Buzinkai and Jonathan Perry; sound
effects made in SFXR. From Spelunky Classic 1.1, © 2008, 2009 Derek Yu and Mossmouth, LLC."

**Per-page notice**: none beyond the header line and footer on HTML pages. Files read without them (the
`.md` twins, `/llms.txt`, `/llms-full.txt`) start with the footer's text, its link made absolute
(`https://spelunky.ai/LICENSE.txt`).

## Don'ts

- **No selling, paywalls, ads or sponsorships.** The license only says Covered Works may not be sold. Ads
  or paid tiers on pages built around the game's assets earn money from them, so the conservative reading
  treats them as selling. So do affiliate links and donation buttons, until open question 2 is answered.
- **Don't call it the official wiki** or imply Mossmouth runs or endorses it. Don't use the Spelunky logo
  or Mossmouth's marks as our own branding (the wiki's masthead, favicon or site name).
- **No Spelunky HD, Spelunky 2 or other Mossmouth assets.** Those games aren't under this license. The same
  goes for text and images copied from other fan wikis (Fandom's are CC BY-SA, a different license).

## Trademark

"Spelunky" is Mossmouth's name for its games, so we use it only to say what the wiki is about, e.g. "a
wiki for Spelunky Classic" or "Spelunky Classic fan wiki", always with "unofficial" or "fan" nearby. Page
titles follow that, like "Shotgun – Spelunky Classic fan wiki".

## Open questions for the user

1. **Music.** The original's credits name George Buzinkai and Jonathan Perry as the composers. The license
   is Mossmouth's and calls the whole Game redistributable, and the port already plays the music, but it
   doesn't say whether Mossmouth licensed the music onward. Until you decide, clips and pages carry no
   music. The question isn't the wiki's alone: the releases and `/play/` already ship the `.ogg` files.
2. **Donations and affiliate links.** Is a donation link (GitHub Sponsors or similar), or an affiliate
   link, acceptable? Until then, neither.
3. **The logo and domain.** The landing page uses the game's logo sprite as its heading, and the domain is
   spelunky.ai. Both carry clear "unofficial" notices; the wiki won't use the logo as its masthead unless
   you say so. Asking Mossmouth directly would settle questions 1 to 3.
