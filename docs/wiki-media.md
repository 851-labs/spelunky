# Wiki media

The spelunky.ai wiki's images come from the game's own assets, exported by scripts rather than drawn or
screenshotted, so a page can't show a sprite the game doesn't have, and its clips are recorded from the port
playing committed inputs, so a page can't show a move the game doesn't make. The output isn't committed:
a `wiki-media` release asset packages it, with `LICENSE` and a notice (`docs/wiki-license.md`), and the site
pins one ([Publishing](#publishing)). The pages' numbers come from the game's code the same way
([wiki data](wiki-data.md)).

## Sprites

```sh
scripts/extract        # once: assets/
scripts/wiki-sprites   # target/wiki-media/sprites; --scale N for another upscale factor (default 4)
```

`scripts/wiki-sprites` runs `tools`' `wiki-sprites` binary on the extracted bundle (`bundle.bin` and the
atlas pages). It replaces the output directory as a whole, and the same bundle always gives byte-identical
files (`crates/tools/tests/wiki_sprites.rs`). Every PNG carries the copyright notice in a `Copyright` text
chunk.

| Path | Contents |
| --- | --- |
| `index.json` | The notice, `scale`, `room_speed`, `default_fps`; every sprite (`name`, `id`, `frames`, `width`, `height`, `fps`, `meta`: its `sprite.json`); and every object that uses a sprite (`sprite`, its own; `mask`; `sprites`, all it uses) |
| `<sprite>/sprite.json` | Size, origin, collision `bbox` (inclusive; the union of the frames'), `fps` and `fps_source`, the file paths, each frame's `bbox`, `objects` (whose own sprite it is) and `used_by` |
| `<sprite>/frames/<i>.png`, `<i>@4x.png` | Each frame, at native size and upscaled (nearest neighbour) |
| `<sprite>/strip.png`, `strip@4x.png` | The frames side by side, left to right: a sprite sheet for a CSS `steps()` animation |
| `<sprite>/anim.png`, `anim@4x.png` | Sprites with several frames: an APNG that loops at `fps` |

Paths in the JSON are relative to the output directory. Sprite directories use the sprite's name.
`sSnakeWalkL`'s strip and preview, at 4×:

![sSnakeWalkL's strip](wiki-media/sSnakeWalkL-strip@4x.png) ![sSnakeWalkL's preview](wiki-media/sSnakeWalkL-anim@4x.png)

An object's `sprites` are its own sprite, its mask, and every sprite its events name in GML or in an
action's argument. Sprites it only gets through a script it calls aren't followed (the player's come from
`characterSprite`), so curated entity pages may need to add some.

**Frame rate.** GM advances `image_index` by `image_speed` every step, at 30 steps a second. The speed is
known when an object whose own sprite it is sets `image_speed` to a number at the top level of its create
event, or inherits one, with nothing conditional changing it (`fps_source.via` is `"sprite"`). It is a
weaker hint when only an object that uses the sprite does (`"uses"`; oSnake's 0.4 for `sSnakeWalkL`).
Otherwise `fps_source` is `null` and the preview plays at `default_fps`: 15, from an `image_speed` of 0.5,
the value the game's code sets most often. An `image_speed` of 0 means the object holds the sprite still
and picks frames in code (oBow's `sBowRight`); its preview plays at `default_fps` too.
In the game, speeds often change with state (the player's run speeds up with its velocity), so `fps` is a
suggestion for a preview, not a fact for a page.

**Why APNG.** The `png` crate the workspace already uses writes APNG, every current browser plays it, and it
keeps full alpha losslessly. Animated WebP would need libwebp (a C library) or a hand-written muxer, for
files a few hundred bytes smaller.

## Clips

```sh
scripts/extract              # once: assets/
scripts/wiki-clips           # target/wiki-media/clips (needs ffmpeg: brew install ffmpeg)
scripts/wiki-clips --check   # only check the specs' story facts, as scripts/check does
```

A clip is a spec in `wiki/clips/<entity>/<move>.toml`; its id is that path (`snake/bite`). Its source is a
committed replay, cut to a frame range, or a **staged scene**: a custom level (docs/source-inventory.md 7.3)
the player starts in, with a few inputs, to show one monster and the player, like a game's ability videos.

```toml
title = "Spider drop"                     # the clip's name, for a page and its VideoObject
description = "A spider hanging from ..." # a sentence or two (the VideoObject's description)
from = 0                                  # the first and last frames shown, inclusive (90 to 600 frames)
to = 109
poster = 66                               # the frame the poster shows, after the notice's 60
story = "-oSpiderHang@62 +oSpider@62 plife=3@88 !plife<3"
# scale = 2                               # output pixels per game pixel: 2 (640x480, the default) or 4
# loop = true                             # whether the page loops it (the default)

[scene]                                   # or, instead: replay = "fidelity/replays/enemy-mines-seed-1-kills.txt"
# seed = 0                                # the RNG seed at startup
# lives = 4                               # the player's hearts, bombs and ropes (the editor's default, 4)
level = '''
1111111111111111111111
1000000000000s00000001
10000@0000000000000001
1111111111111111111111
'''
inputs = '''
20 hold right 45                          # replay event lines, in scene frames
'''
```

- **Frames** are the source's: the replay's, or the scene's, whose frame 0 is its first in the level.
- **The level** is rows of the level editor's tile codes (`scrCreateTileObj`: `1` a brick, `@` the
  entrance, `n` a snake, `b` a bat, `s` a hanging spider, `h` a caveman, `K` a skeleton...), up to 40 by
  32, padded with `0` (nothing), with exactly one entrance. Rows start at room pixel 16 and the view
  follows the player, so a scene a dozen rows tall fills the picture.
- **The story** is [story facts](formats.md#stories), checked on the frames the clip shows (each window
  cut to `from`..`to`, and a fact without one gets them all): at least one that must hold, so a game change
  that takes a clip away from what its page says fails `scripts/check`, as a replay's story would. A
  condition must become true in the clip. A scene also has to have loaded its level.

`wiki-clips` (in `crates/harness`, `harness::clip`) turns a scene into a replay: `file levels/wiki.lvl`
with the level, then the boot `custom-chain-end.txt` uses (the intro and title skipped with X, F3 for the
load-level menu, a click on its only level and Enter on frame 240; scene frame 0 is frame 241), then the
inputs. `--replays DIR` writes them, for `replay-story` while writing a spec (its frames, less 241, are
the scene's) or for `scripts/fidelity`: the first four scenes matched the oracle frame for frame. It plays
every source as the app does, with the sound DLL modelled from the game's sounds (replays are recorded
against the oracle's stub, which rarely makes a difference), and checks the facts. Rendering, the app
captures the frames headless (`app --capture-from F --notice-frames 60`), `wiki-clips` scales them down to
the game's 320x240 and up again by `scale`, nearest neighbour, and ffmpeg encodes them.

**The notice.** The app shows the copyright notice for a session's first six seconds, which a clip from
later doesn't have, so the license's notice at the start of every Covered Work (docs/wiki-license.md) is
burned into each clip's first two seconds (`NOTICE_FRAMES`), the last fading out, in the game's own
pixels and font. It's in the files' metadata too (`copyright`, and the caption as `comment`), and in the
poster's `Copyright` text chunk (the poster is a frame after the notice, so a page's first still is clear).
Clips are silent.

| Path | Contents |
| --- | --- |
| `index.json` | `format`, the notice, `license`, `port` (the port's version, `git describe`), the `caption` the page shows under each clip (docs/wiki-license.md), `fps`, and every clip's `clip.json` |
| `<id>/clip.json` | `id`, `title`, `description`, `spec`, `source` (`{"kind":"replay","replay":...}` or `{"kind":"scene","seed":...}`), `from`, `to`, `frames`, `fps`, `duration` (seconds), `width`, `height`, `scale`, `loop`, `poster_frame`, `notice_frames`, `frames_hash`, `files`, the notice, `license` and `caption` |
| `<id>/clip.mp4` | H.264, 4:2:0, `+faststart` (playable before it's all in), for every browser |
| `<id>/clip.webm` | VP9, 4:2:0, smaller where it plays |
| `<id>/poster.png` | The `poster` frame, at the clip's size |
| `<id>/replay.txt` | A scene's replay |

Paths in the JSON are relative to the output directory. The scale is even so that the videos' colour, at
half resolution, never mixes two game pixels. The frames are the same on every run of the same port and
assets, and `frames_hash` says so (`harness::fingerprint`'s hash of each frame in game pixels, then of
those hashes); the video files needn't be, since ffmpeg's encoders change between versions. A render of
every spec replaces the output directory as a whole (if it holds a render: one without `index.json` is
left alone); with spec paths it replaces only those clips, and `index.json` lists every clip there.

The first set, for the Mines monsters' pilot pages (851-3347): `snake/bite`, `bat/swoop`, `spider/drop`
and `caveman/charge` are scenes; `snake/stomp`, `snake/thrown-rock`, `bat/jump-kill` and `skeleton/rise`
are cut from replays. The quirks' pages (851-3360) have theirs under `quirks/`, each a quirk of
[docs/quirks.md](quirks.md) played out: `quirks/thrown-item-nearest-enemy` and `quirks/pickup-nearest-enemy`
are cut from staged scenes' replays (`action-throw-rock-nearest-enemy.txt`, `action-pickup-nearest-enemy.txt`,
in the form `--replays` writes), committed so that the oracle's fingerprint pins what each clip shows.

## Publishing

The media go out as a GitHub Release asset the site pins, like the web build (`site/web-release.txt`).
Every release makes its own: `release.yml`'s native build (`build-native.yml`, on a Mac mini, whose GPU the
app's capture needs) renders the sprites and every clip from the assets it extracted, the clips with the
app it ships (its arm64 half), packages them as `wiki-media-TAG.tar.gz`, checks the package as the site
will unpack it, and the release publishes it with the other files (and in `SHA256SUMS`). Its clips' caption
names the release. By hand, the same is:

```sh
scripts/wiki-sprites && scripts/wiki-clips
scripts/wiki-media-package wiki-media-v1   # target/wiki-media-v1.tar.gz and .sha256 (on macOS)
```

The tarball holds `sprites/`, `clips/`, `LICENSE.txt` and `NOTICE.txt`, with fixed owners, times and order,
so the same media give the same bytes. `site/wiki-media.txt` pins one, `tag` and `sha256`, or `none`: a game
release's `wiki-media-<tag>.tar.gz`, or a `wiki-media-*` release's `<tag>.tar.gz` (the fallback below).
`scripts/bump-web-release TAG` pins the release's along with its web build, so the game at `/play/` and the
clips of it go live together; a release without them leaves the pin alone. `site/wiki-media.sh DIST`
downloads the pinned package, checks its SHA-256 and unpacks it at `DIST/wiki/media/` (nothing while the pin
is `none`; `--local PATH` takes a package instead). `site/build.sh` doesn't call it yet: that waits for the
TanStack Start migration (851-3331), in 851-3379, and until then the pin goes nowhere.

**Every clip, every release.** The release renders every clip afresh rather than only those whose spec
changed: a clip takes about 3 seconds (on an M1 Pro; checking its story is a fraction of a second, the rest
is the app's capture and the two encodes), so a few hundred take a quarter of an hour at most on the mini,
in parallel with a fidelity run that takes longer. A cache wouldn't save much. Keyed by the spec and the
engine, it would miss on nearly every release, since a release is the engine changing; keyed by
`frames_hash`, it would need the frames captured first, and could skip only the encodes. And it would make a
release's media depend on an earlier release's files. If the render grows too slow, rendering clips in
parallel is the cheaper fix. The ffmpeg is a pinned static build (`scripts/release/tools ffmpeg`), since the
minis have none, with only the system's libraries; a clip's `frames_hash` vouches for its frames whichever
ffmpeg encoded them.

**By hand, between releases.** To publish media without a release (new specs the site needs before the
next one, or a release whose media are wrong), after the PR that changes the specs has merged, from `main`:

```sh
git switch main && git pull
scripts/extract && scripts/wiki-sprites && scripts/wiki-clips
scripts/wiki-media-package wiki-media-v1
gh release create wiki-media-v1 target/wiki-media-v1.tar.gz --title "Wiki media v1" --latest=false \
  --notes "The spelunky.ai wiki's sprites and clips (docs/wiki-media.md). Spelunky (c) 2008, 2009 Derek Yu and Mossmouth, LLC; Spelunky User License v1.1b (LICENSE.txt inside)."
```

Then, on a branch, set `site/wiki-media.txt` to `tag wiki-media-v1` and the `sha256` from
`target/wiki-media-v1.tar.gz.sha256`, check it with `site/wiki-media.sh` on a scratch directory, and merge
that: like the web pin, the PR is the deploy, once 851-3379 has `site/build.sh` run `site/wiki-media.sh`.
A later set is `wiki-media-v2`, and so on; a release is never replaced, so an old pin keeps working.
`--latest=false` keeps the game's own release the latest, and the tag doesn't match the release workflow's
`v*`, so nothing else runs. The next `scripts/bump-web-release` moves the pin to its release's media.
