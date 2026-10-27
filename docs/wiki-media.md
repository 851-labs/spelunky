# Wiki media

The spelunky.ai wiki's images come from the game's own assets, exported by scripts rather than drawn or
screenshotted, so a page can't show a sprite the game doesn't have. The output isn't committed: the
one-time `wiki-media` release asset packages it, with `LICENSE` and a notice (`docs/wiki-license.md`).

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
