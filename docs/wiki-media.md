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
captures the frames headless (`app --capture-from F`), `wiki-clips` scales them down to
the game's 320x240 and up again by `scale`, nearest neighbour, mixes the sound, and ffmpeg encodes them.

**The sound.** A clip has the game's sound effects and music, as the app plays them (`Clip::sound`). The
app's player (`audio::backend::Player`, kira) follows the commands the sound model makes each tick; to
record them, `wiki-clips` plays the source again headless and hands each tick's commands to the same
player on a kira backend with no device (`audio::offline`), which mixes exactly one tick, 1600 stereo
frames at 48 kHz, after each. So the volumes, pans, rates, loops, kira's resampling and its 10 ms fade on a
cut-off sound are the live player's, and a frame's sound starts with its picture, as the app hands the
player a tick's commands right after drawing it. The mix runs from boot, so music already playing at the
clip's first frame is there, from where it had got to; only the clip's frames' ticks are kept. Two things
differ from the device: the clock (live, the commands land between the device's buffers at wall-clock
times; here, between ticks), and the music, which kira streams on a thread of its own live but which is
decoded whole when it starts here (as on wasm), since that thread's timing would decide what got mixed.
Nothing touches an audio device or the wall clock, and the same commands always mix the same samples
(`crates/harness/tests/clip_sound.rs` checks `snake/bite`'s, with the reference zip's sounds, and that the
bite's sound starts on the bite's frame). ffmpeg encodes them as AAC and Opus, which decode to exactly the
clip's samples, no more and none shifted (the MP4's edit list and the WebM's padding trim the codecs'
priming and last packet); a clip that's silent throughout gets no sound track.

**The notice.** Clips carry no notice in the picture: the pages that show them have it in their header
line and footer (docs/wiki-license.md, Decisions). It's in the files' metadata
(`copyright`, and the caption as `comment`), and in the poster's `Copyright` text chunk. A clip with sound
has the audio credit after the notice in `copyright`, and the pages that show one have it in their footer.

| Path | Contents |
| --- | --- |
| `index.json` | `format`, the notice, `license`, `port` (the port's version, `git describe`), the `caption` (the videos' `comment`), the `audio_credit` (docs/wiki-license.md), `fps`, and every clip's `clip.json` |
| `<id>/clip.json` | `id`, `title`, `description`, `spec`, `source` (`{"kind":"replay","replay":...}` or `{"kind":"scene","seed":...}`), `from`, `to`, `frames`, `fps`, `duration` (seconds), `width`, `height`, `scale`, `loop`, `poster_frame`, `frames_hash`, `has_audio` (whether the videos have a sound track), `audio_hash` (or `null`), `files`, the notice, `license` and `caption` |
| `<id>/clip.mp4` | H.264, 4:2:0, `+faststart` (playable before it's all in), for every browser; AAC, 128 kb/s |
| `<id>/clip.webm` | VP9, 4:2:0, smaller where it plays; Opus, 96 kb/s |
| `<id>/poster.png` | The `poster` frame, at the clip's size |
| `<id>/replay.txt` | A scene's replay |

Paths in the JSON are relative to the output directory. The scale is even so that the videos' colour, at
half resolution, never mixes two game pixels. The frames are the same on every run of the same port and
assets, and `frames_hash` says so (`harness::fingerprint`'s hash of each frame in game pixels, then of
those hashes); the video files needn't be, since ffmpeg's encoders change between versions. `audio_hash`
is the same hash of the mixed samples (little-endian `f32`s) before ffmpeg has them: the same on every run
on the same kind of machine (kira mixes with the platform's float functions, which needn't round alike
everywhere, unlike the game's state). A render of
every spec replaces the output directory as a whole (if it holds a render: one without `index.json` is
left alone); with spec paths it replaces only those clips, and `index.json` lists every clip there.

The first set, for the Mines monsters' pilot pages (851-3347): `snake/bite`, `bat/swoop`, `spider/drop`
and `caveman/charge` are scenes; `snake/stomp`, `snake/thrown-rock`, `bat/jump-kill` and `skeleton/rise`
are cut from replays. The pilot's pages added six scenes: `giant-spider/drop`, `giant-spider/web` (seed 9,
so its web shot comes early), `giant-spider/bomb`, `caveman/stun`, `caveman/ledge` (seed 9) and
`skeleton/whip`. The quirks' pages (851-3360) have theirs under `quirks/`, each a quirk of
[docs/quirks.md](quirks.md) played out: `quirks/thrown-item-nearest-enemy`, `quirks/pickup-nearest-enemy`,
`quirks/caveman-sight-alerts-others`, `quirks/caveman-ledge-slide`, `quirks/skeleton-faces-right`,
`quirks/whip-hits-every-step`, `quirks/yeti-king-touch-custom-level`, `quirks/yeti-king-whip-wind-up`,
`quirks/yeti-never-charges`, `quirks/tomb-lord-whip-wind-up`, `quirks/mitt-gravity-one-step`,
`quirks/mitt-catch-every-rock`, `quirks/yeti-stomp-rising`, `quirks/mattock-wind-up-twice`,
`quirks/mattock-wrong-block`, `quirks/giant-spider-whip-wind-up`, `quirks/bomb-arrow-free`,
`quirks/teleporter-old-target`, `quirks/parachute-torn-moving-right`, `quirks/jetpack-vanishes-player-invisible`,
`quirks/jar-whip-rolls-twice`, `quirks/skull-breaks-on-ceiling`, `quirks/jar-shot-in-hands`,
`quirks/chest-trap-bomb-taken-out`, `quirks/arrow-trap-sensor-left-wider` and `quirks/smash-trap-random-throw` are
cut from staged scenes' replays
(`action-throw-rock-nearest-enemy.txt`, `action-pickup-nearest-enemy.txt`, `enemy-caveman-sight-alerts-hawkman.txt`,
`enemy-caveman-ledge-slide.txt`, `enemy-skeleton-tile-faces-right.txt`, `enemy-megamouth-whip-every-step.txt`,
`enemy-yeti-king-custom-level-touch.txt`, `enemy-yeti-king-whip-wind-up.txt`, `enemy-yeti-sight-never-charges.txt`,
`enemy-tomb-lord-whip-wind-up.txt`, `item-mitt-gravity-one-step.txt`, `item-mitt-catch-every-rock.txt`,
`enemy-yeti-stomp-rising.txt`, `weapon-mattock-wind-up-twice.txt` (the `mattock/wind-up` scene),
`weapon-mattock-wrong-block.txt`, `enemy-giant-spider-whip-wind-up.txt` (the `giant-spider/wind-up` scene),
`weapon-bow-bomb-arrow-free.txt`, `weapon-teleporter-old-target.txt`, `item-parachute-torn-moving-right.txt`,
`item-jetpack-player-invisible.txt`, `item-jar-whip-rolls-twice.txt` (the `jar/whip` scene),
`item-skull-breaks-on-ceiling.txt`, `item-held-jar-shot.txt`, `item-chest-trap-bomb-unarmed.txt`,
`trap-arrow-sensor-left-wider.txt` and `trap-smash-random-throw.txt`, in the form `--replays` writes),
committed so that the oracle's fingerprint pins what each clip shows. `quirks/olmec-slam-drifting-left` is cut
from `ending-seed-1.txt`, the route through Olmec's fight, whose story facts pin it.
`quirks/ash-graves` is cut from a real level's replay (`item-seed-244-ash-graves.txt`): level generation makes no grave
in a custom level. So is `quirks/ankh-thrown-out-of-shop` (`item-seed-9-ankh-thrown-out.txt`), as no level editor tile
makes the Ankh or a shop, and `quirks/crystal-skull-sold-as-idol` (`item-seed-213-crystal-skull-sold.txt`), as a
Crystal Skull is sold only on a shop's floor.
The Ice Caves monsters' pages (851-3349) added eleven scenes, `yeti/throw`, `yeti/ceiling`, `yeti/stomp`,
`yeti-king/yell`, `yeti-king/bomb`, `yeti-king/thrown-rock`, `ufo/laser`, `alien/stomp`, `alien-boss/psychic`,
`alien-boss/whip` and `alien-boss/bomb`, and three cut from replays: `ufo/crash` (`ice-seed-14.txt`),
`ufo/thrown-rock` and `alien/parachute` (`enemy-ufo-alien-seed-14.txt`).
The Temple's pages (851-3350) added nine scenes, `tomb-lord/flies`, `tomb-lord/wind-up`, `tomb-lord/whip`,
`hawkman/throw`, `hawkman/ceiling`, `hawkman/whip`, `magma-man/spawn` (seed 53), `magma-man/touch` and
`magma-man/bomb` (seed 106: a Magma Man comes only from a lava spurt, so a scene with lava waits for the
right seed), and seven cut from replays, as neither Olmec nor a Scarab has a level editor tile:
`olmec/awakening`, `olmec/slam` and `olmec/lava` (`ending-seed-1.txt`), `olmec/crush`
(`enemy-olmec-crush-seed-1.txt`, made for it), `tomb-lord/bomb` (`city-of-gold-seed-2-sceptre.txt`),
`scarab/collect` (`trap-dark-smash.txt`) and `ghost/touch` (`enemy-ghost-seed-2.txt`).
The shop equipment's pages (851-3351) added five scenes, each picking the item up off the floor:
`spectacles/reveal` (seed 0, whose bricks bury Climbing Gloves and four big gems near the player),
`climbing-gloves/climb`, `spring-shoes/jump`, `spike-shoes/mantrap` and `pitchers-mitt/rock`; and five cut
from the items' theft replays: `pitchers-mitt/throw` (`item-seed-11-mitt-throw.txt`), `spike-shoes/stomp`
(`item-seed-7-spike-stomp.txt`), `compass/arrow` (`item-seed-3-compass.txt`), `paste/wall`
(`item-seed-53-paste.txt`) and `paste/yeti` (`item-seed-53-paste-yeti.txt`).
The weapons' pages (851-3352) added nine scenes, `whip/crack`, `machete/caveman`, `mattock/dig`,
`mattock/wind-up` (a Yeti King behind the player), `bow/arrow`, `bow/bomb-arrow`, `web-cannon/caveman`,
`teleporter/snake` (seed 1, whose roll lands the player on the snake) and `giant-spider/wind-up`; and five cut
from replays: `shotgun/shopkeeper` (`weapon-seed-66-shop.txt`), `shotgun/spiders`
(`weapon-seed-38-web-cannon.txt`), `pistol/shopkeeper` (`weapon-seed-39-pistol.txt`), `teleporter/rock`
(`weapon-seed-40-teleporter.txt`) and `sceptre/hawkman` (`weapon-seed-2-sceptre.txt`, made for it: no level
editor tile makes a Sceptre, and a Tomb Lord leaves one only on level 13).
The back items' and relics' pages (851-3437) added nine, all cut from replays, as the relics have no level
editor tile and the back items' effects show best in real levels: `parachute/open`
(`item-seed-26-parachute.txt`), `cape/glide` (`item-seed-35-cape.txt`), `jetpack/fly`
(`item-seed-35-jetpack.txt`), `kapala/heart` (`item-seed-11-kapala.txt`), `udjat-eye/chest`
(`item-seed-35-udjat-eye.txt`), `udjat-eye/sealed-key` (`item-seed-35-udjat-eye-sealed-key.txt`), `ankh/revive`
(`item-seed-9-ankh.txt`), and `hedjet/moai` and `hedjet/gold-door` (`city-of-gold-seed-9.txt`). A story fact
can't read a built-in such as `depth`, so what the Udjat Eye shows (two big gems in the rock going to depth 0
on the pickup step) was checked in a `replay-run` trace instead.
The carryables' pages (851-3353) added three scenes, `jar/whip` (seed 13, whose Jar lets out two things: a whipped
Jar rolls its contents twice), `crate/open` (seed 4, a Pitcher's Mitt, from a Crate opened in the player's
hands) and `crate/rope` (seed 2, a Rope Pile collected as it appears), and five cut from replays:
`jar/throw` (`action-throw-jar-snake.txt`), `skull/throw` and `arrow/trap` (`action-skull-bomb-rope-arrow.txt`),
`chest/open` (`equip-seed-13-chest-gems.txt`) and `ball-and-chain/drag` (`item-seed-38-ball-and-chain.txt`).
The Bomb's, the Rope's, the gold's and the idols' pages (851-3474) added twelve, all cut from replays (the
gold's had to be: a level editor block never holds gold, as the editor sets each block's sprite back to plain
rock after the block has rolled for its vein): `bomb/throw` (`action-skull-bomb-rope-arrow.txt`), `bomb/chain` and `gold-bar/tutorial`
(`action-bomb-chain.txt`), `bomb/brick` (`terrain-bomb-brick.txt`), `rope/climb` (`movement-items-rope.txt`),
`rope/ledge` (`action-rope-tunnel.txt`), `rope/burn` (`terrain-rope-burn-lava.txt`), `golden-idol/boulder`
(`trap-boulder-crush.txt`), `golden-idol/exit` (`trap-boulder-outrun.txt`), `crystal-skull/ghost`
(`item-seed-29-crystal-skull.txt`), `gold-chunk/mattock` (`item-seed-35-mattock-break.txt`) and
`gold-nugget/shopkeeper` (`item-seed-22-red-lamp.txt`).
The traps' pages (851-3354) added five scenes, `arrow-trap/fire`, `arrow-trap/miss`, `spikes/fall`,
`spear-trap/stab` (eight hearts, so the strike's three hits show) and `lava/burn`, and cut the rest from the
trap replays: `arrow-trap/dark` (`trap-dark-arrow.txt`), `spear-trap/bomb` (`trap-dark-spear.txt`),
`smash-trap/crush` (`trap-smash-crush.txt`), `smash-trap/lit` (`trap-dark-smash.txt`), `spring-trap/bounce` and
`spring-trap/no-floor` (`trap-spring.txt`), `boulder/crush` and `boulder/outrun` (`trap-boulder-crush.txt`,
`trap-boulder-outrun.txt`) and `lava/flow` (`terrain-bomb-lava.txt`). The idol's traps can't be staged: picking
the idol up in a custom level sets nothing off (`oPlayer1`'s step skips it in `rLoadLevel`), so
`ceiling-trap/crush` is cut from `trap-ceiling-crush.txt`, made for it: Temple seed 8's idol room is sealed in
rock, which `area-route` can't get into, so the route there (a bomb through the floor beside it) is by hand.
The four areas' pages (851-3355) added seven, all cut from replays, each showing the way into an area (a level
editor scene is no area's level): `mines/start` (`mines-seed-1.txt`), `mines/exit` (`mines-seed-2.txt`),
`jungle/from-mines` (`tunnelman-seed-2-donate.txt`), `jungle/shortcut` (`title-shortcut5.txt`),
`ice-caves/from-jungle` (`tunnelman-seed-9-jungle-paid.txt`), `temple/shortcut` (`temple-seed-113.txt`) and
`temple/to-olmec` (`temple-seed-1.txt`).

### Staging a scene

How the pilot's scenes were made, since a scene takes a few rounds:

1. Write the spec with the level and the inputs, and a placeholder story that holds (a variable that
   changes inside the clip, `oPlayer1.xVel>0`): a spec whose story fails can't be rendered.
2. `scripts/wiki-clips --check --replays DIR SPEC` writes the scene's replay, and `replay-story --watch
   oCaveman.status,plife DIR/<entity>-<move>.txt` (`crates/harness`) lists what happens and when. Its frames
   are the replay's: subtract 241 for the scene's. `--watch` reads globals and objects' own variables, not
   built-ins such as `x` and `y`.
3. Look at it: `scripts/wiki-clips --out DIR SPEC` renders it, and ffmpeg can tile every Nth frame into
   one image (`-vf "select='not(mod(n\,8))',scale=320:240,tile=5x4" -frames:v 1`). The picture catches what
   the facts don't: a web catching the drops, the view cutting a monster off.
4. For something random (a Giant Spider's web shot, a Caveman's direction), edit the replay's `seed` line
   and run `replay-story` over a range of seeds, then put the one that shows it soonest in the spec's
   `seed`. The seed is the game's at startup, so changing the level changes what each seed does.
5. Write the story from the run: the frames each fact happens on, and a `!` fact for what mustn't (the
   player losing a heart). A fact names "some instance" of an object, so a story can't tell two Cavemen
   apart; a clip about several of one monster needs a different one in each role.

What surprised the pilot: a custom level makes its objects before the player, so code that turns toward
the player when it's created doesn't (a Skeleton from tile `K` faces right); a hanging monster with no
block above it drops at once (`S` with an open row above it is a Giant Spider that starts on the ground);
and monsters are only active near the view, which follows the player, so a scene can't show a Bat more
than 160 pixels from him going back to its perch. And a scene isn't one of the game's own levels, so code
behind `isRealLevel()` doesn't run in it: the Yeti King's touch knocks the player back there without
costing a heart, so a clip of what it costs has to come from a replay of a real level. A custom level does
count as a level to the blocks, though (`isLevel()` is true in it), so its bricks bury gems and items as
generation's do, and the seed decides which: a scene about buried things searches seeds for them. Its exit
has no position for the Compass to point at, as only the room scripts set one (`global.exitX`), so the
Compass's clip is cut from a real level's replay too. A Giant Spider's tile (`S`) is the top left of its
two blocks by two: one placed a row above the floor stands on it, and one on the floor's row is inside it and
dies as the level loads.

## Publishing

The media go out as a GitHub Release asset the site pins, like the web build (`site/web-release.txt`).
Two workflows make them, both on a Mac mini, whose GPU the app's capture needs, with the same step
(`.github/actions/wiki-media-render`): it renders the sprites and every clip from freshly extracted assets,
packages them, and checks the package as the site will unpack it.

- **Every game release** makes its own: `release.yml`'s native build (`build-native.yml`) renders them, the
  clips with the app it ships (its arm64 half), as `wiki-media-TAG.tar.gz`, which the release publishes with
  the other files (and in `SHA256SUMS`). Its clips' caption names the release.
- **Between releases**, `wiki-media.yml` makes them from `main` when what they're made from changes, so new
  clips don't wait for a game release ([below](#between-releases)).

By hand, the same is:

```sh
scripts/wiki-sprites && scripts/wiki-clips
scripts/wiki-media-package wiki-media-v1   # target/wiki-media-v1.tar.gz and .sha256 (on macOS)
```

The tarball holds `sprites/`, `clips/`, `LICENSE.txt` and `NOTICE.txt` (the notice, and the audio credit
for the clips' sound), with fixed owners, times and order,
so the same media give the same bytes. `site/wiki-media.txt` pins one, `tag`, `sha256` and the `date` its
release was published, or `none`: a game release's `wiki-media-<tag>.tar.gz`, or a `wiki-media-*` release's
`<tag>.tar.gz`. `scripts/bump-wiki-media TAG` writes the pin from the published release (downloading the
package and checking it as the site will unpack it). `site/wiki-media.sh DIST`
downloads the pinned package, checks its SHA-256 and unpacks it at `DIST/wiki/media/HASH/`, `HASH` being the
first 16 hex digits of that SHA-256 (nothing while the pin is `none`; `--local PATH` takes a package instead).
`site/build.sh` runs it, so the site serves the media at `/wiki/media/HASH/`, cached for good
(docs/web-deploy.md, "Wiki media"), and a PR that changes the pin is a deploy.

**The newest commit's media win.** `scripts/bump-web-release TAG` pins the release's media along with its web
build (`scripts/bump-wiki-media --game`), so the game at `/play/` and the clips of it go live together,
unless the release has none (made before the release workflow rendered them), or the pin is a `wiki-media-*`
release of a commit the release's tag doesn't contain: a later one on `main`, which shows everything the
release's media do and more, or `main` beside a release tagged off it (a hotfix), as `wiki/clips/` lives on
`main`. Then the pin stays. It goes by the commits (GitHub's compare), not by when each was published: a
game release takes over an hour, so media of a later commit can come out before it. A game release's pin is
always replaced, so bumping to an older release rolls its media back too.

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

### Between releases

`wiki-media.yml` puts new or changed clips live without a game release, which used to be the only way (v0.1.8
to v0.1.10 were releases with no game change: six builds, signing, the fidelity run, and an update offered
to every installed app for nothing). On every push to `main`:

1. It finds the last published media at or before the commit: of the tags, a game release's or a
   `wiki-media-*` one's, whose release is published with its package, the one fewest commits back.
2. If nothing the media are made from changed since that tag, it stops there and succeeds. That's checked in
   the job (the workflow's `PATHS`, git pathspecs), as `mini.yml`'s `paths` are, rather than by the push's
   own `paths` filter: the specs (`wiki/clips/`) and the replays clips are cut from; the renderer, the
   app's capture, the drawing and the sound (`crates/harness`, `app`, `draw`, `audio`, and what they share,
   `config` and `trace`); the sprite exporter and the asset extractor (`crates/tools`, `assets`, `gmk`,
   `gml`, `reference/`); the scripts, the workspace's `Cargo.toml` and `Cargo.lock`, the pinned ffmpeg and
   toolchain, `LICENSE`, and the workflow itself. Not the game's own logic (`crates/game`, `gm`,
   `transpile`): its changes reach the clips with the next game release, or a hand run.
3. Otherwise, on one Mac mini, it builds the app (the arm64 development build, with a branch native build's
   cached target dir and settings), extracts the assets, renders and packages everything as
   `wiki-media-<date>-<sha>` (the commit's date and short SHA; the caption names `git describe`'s
   `v0.1.10-3-g0e401b9`, as `scripts/wiki-clips` does).
4. `scripts/wiki-media-diff` compares the package with the last published one as the site shows it:
   every file but the clips' videos, without what names the package (the port's version in the captions,
   the package's name in the notice; a change to their other words shows). A clip's `frames_hash` and
   `audio_hash` vouch for its videos. If nothing differs, it stops: a change
   that renders the same media publishes nothing.
5. It publishes the package as the GitHub Release `wiki-media-<date>-<sha>`, tagged at the commit, with
   `--latest=false` so the game's release stays the latest (and the tag isn't `v*`, so `release.yml`
   doesn't run). The notes list what changed.
6. It commits the pin (`scripts/bump-wiki-media`) through GitHub's API onto a new branch,
   `wiki-media/<tag>`, at the commit (made by `github-actions[bot]`), and deletes older
   `wiki-media/*` branches, closing any pull request from them: the new one supersedes them.
7. Someone opens the pull request from that branch, titled `Wiki media: pin <tag>`: the run's summary has
   the `gh pr create` command. `pr-ok` runs on it like on any other (`site/test.sh` too, as it changes
   `site/`), and merging it is the deploy. It has no Linear ticket; the branch name isn't one's.

The workflow can't open that pull request itself: the repository doesn't let GitHub Actions create pull
requests, and one its token opened would start no workflow (GitHub runs none for events its `GITHUB_TOKEN`
causes), so `pr-ok`, which the "Protect main" ruleset requires, would never report. A person (or the
supervising agent) opening it starts `pr.yml` as usual. If the ruleset's approval for unattributed changes
ever holds one back, the commit's author is the reason: `scripts/bump-wiki-media <tag>` on a branch of
one's own makes the same change.

Runs never overlap (a concurrency group). Pushes that land while one renders coalesce: GitHub keeps only the
newest pending run, which compares with the last published media rather than with its push's parent, so it
includes the skipped runs' changes. A render takes about as long as a branch's native build of the app plus
the clips (a quarter of an hour at most). The job's token can write the repository's contents, for the
release and the branch; only the steps that publish, and the one that lists the releases, get it, never one
that builds the repository's code or runs what it built. Pushes to `main` are reviewed code, and the cached
target dir is shared only with `build-native.yml`'s unsigned runs (by hand, or a release's dry run), never
with pull requests' jobs.

**By hand.** Actions › Wiki media › Run workflow. With `dry_run` (the default) it renders, packages and
compares any branch, reports the differences in the run's summary, and leaves the package as the run's
artifact, but publishes nothing; off `main` it never publishes. On `main` without it, it publishes whatever
it rendered, changed or not: for a change only to how the clips are encoded, which the comparison doesn't
see, or to replace a pin whose pull request was closed. A re-run of a published run keeps its release and
branch, as the release's name comes from the commit's date.

Without CI, from `main` (a set made by hand is `wiki-media-v1`, `wiki-media-v2`, and so on; a release is
never replaced, so an old pin keeps working):

```sh
git switch main && git pull
scripts/extract && scripts/wiki-sprites && scripts/wiki-clips
scripts/wiki-media-package wiki-media-v1
gh release create wiki-media-v1 target/wiki-media-v1.tar.gz --title "Wiki media v1" --latest=false \
  --notes "The spelunky.ai wiki's sprites and clips (docs/wiki-media.md). Spelunky (c) 2008, 2009 Derek Yu and Mossmouth, LLC; Spelunky User License v1.1b (LICENSE.txt inside)."
git switch -c wiki-media-v1 && scripts/bump-wiki-media wiki-media-v1
```

and open a pull request with the change to `site/wiki-media.txt`: like the web pin, merging it is the
deploy.
