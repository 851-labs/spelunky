# Comparing what the port draws (851-3790, design)

The oracle comparison covers state: each frame's instances, globals and view 0's rectangle (and, once 851-3842
lands, the room's tiles), hashed into every committed replay's fingerprint ([formats.md](../formats.md),
"Fingerprints").
Nothing compares what the port draws. A wrong sprite frame, a HUD number in the wrong font, a blend mode left set, an
`image_alpha` that only drawing reads, or a renderer that rasterizes a scaled sprite a pixel off would all pass
`fidelity-check`, `fidelity-all` and the soak. The oracle draws every frame (Mesa GL under Xvfb), so what it draws is
there to take.

This is the design: three layers, cheapest first, each catching what the one before can't. Each is its own ticket.

## What drawing reads that the trace doesn't

| What | Where the game sets it | Layer that sees it |
|---|---|---|
| `image_alpha` (set by `oWeb`), `image_blend` (never set, so white) | read by `characterDrawEvent`, `oShopkeeper`, `oDamsel`, `oEnemy`, `oTombLord`, `oYetiKing`, `oDice`, the `oPDummy` objects | 1 |
| `image_angle` (also GM8's rotated masks, so partly seen already) | set by `oArrow`, `oFishBone` | 1 |
| `draw_set_color` (148 calls), `draw_set_font` (89), `draw_set_alpha` (17), `draw_set_blend_mode` and `_ext` (8) | HUD, menus, `oScreen`'s dark levels and pause | 1 (only what a frame leaves set), 2 (each call) |
| Background layers, `background_color`, views 1-7, ports | `scrInitLevel`, rooms | 1 |
| What each draw event draws: sprites, text, shapes, surfaces | the 36 objects' draw events, `scrDrawHUD`, `oScreen`'s begin step | 2 |
| How the renderer turns calls into pixels | the `draw` crate and `app`'s renderer | 3 |

## Layer 1: draw state in the trace

Add to the trace, as extensions like `deactivated` and `tiles`:

- per instance: `image_alpha`, `image_blend`, `image_angle`;
- per frame, after its draw: the drawing state a frame leaves (`draw_color`, `draw_alpha`, the font, the blend mode),
  the background layers (each one's index, visibility, foreground flag, position, speed, tiling, stretch),
  `background_color` and whether the room shows it, and every view (enabled, rectangle, port, follow, borders).

Both sides already hold all of it; the oracle's dump (patch 0006) and the port's `World::trace_frame` and
`FrameEncoder` gain fields, `trace-diff` compares them as it does `builtins`. It catches every value only drawing
reads, and drawing state leaking from one frame into the next. It doesn't see what draw events draw, nor state set
and reset within a frame: a frame that draws the wrong text and then sets the colour back looks right, and so do
`oScreen`'s blend modes, which its begin step sets and resets around the frame it draws by hand. Layer 2 sees both.
Cost: one re-record of every fingerprint (about an hour of the oracle).

## Layer 2: each frame's draw calls in the trace

The port already records each frame as a list of GML-level draw commands (`draw::Frame`: `Sprite` with its frame,
position, scale, angle, blend and alpha; `Text` with its bytes, font, colour and alpha; `Rectangle`, `Line`,
`Circle`, `Surface`, `Tile`, `Background`, `SetView`, `SetBlend`, `SetTarget`, `CreateSurface`, `FreeSurface`,
`ResizeScreen`, `Clear`, `Present`). An oracle patch
records the same calls where OpenGMK makes them: its `draw_*` built-ins (`gml/kernel.rs`), the default instance draw
and the tile and background draws (`game/draw.rs` `draw_view`), surfaces, and the view setup. The trace gains
`draws`: a hash of the frame's canonical command list, so fingerprints pin every frame's picture at the level of
calls, and with `OPENGMK_DUMP_DRAWS=1` (and the port's `--draws`) the list itself, which `trace-diff` compares call by
call to say which call differs.

The canonical list compares what GML asked for, not how either side carries it out:

- text is the string, font, position, colour and alpha, not glyph quads (OpenGMK draws per glyph);
- a sprite's frame is `floor(image_index)` wrapped into the sprite's frame count, as both renderers do;
- tiles and backgrounds outside the view are dropped on both sides if either culls them (GM8 culls tiles; check
  what OpenGMK and the port each draw before choosing);
- reals are compared bit for bit, as the trace's are: the port computes every coordinate with GM8's arithmetic.

It catches every semantic draw error: the wrong sprite, frame, position, scale, colour, alpha, text, font, blend
mode, target, a missing tile or background, a view. It can't see the renderer. Cost: `fidelity-check` would record
draw commands, which it skips headless today; measure it (the clips' capture records them for every frame already),
and if it's too slow, hash only every Nth frame and every frame with a room change (nearly every frame has a
`Present`, so that wouldn't save anything). One more re-record of every
fingerprint, so land it with layer 1 if both are ready.

## Layer 3: pixels at chosen frames

An oracle patch writes the back buffer (`glReadPixels`) at chosen frames, `--screenshots F1,F2,...`, as PNGs at the
game's own size. `app --screenshot --frames F` already renders the port's picture of a frame headless. A harness
command (`fidelity-shots`) renders the port's frames and compares them with committed references from the oracle:
exactly where the picture is pixel art at an integer scale, and within a documented tolerance (a count of pixels a
channel step apart) where GL and wgpu may rasterize differently: rotated or scaled sprites, lines, circles, alpha.
The references are PNGs under `fidelity/screenshots/`, recorded with `scripts/fidelity --refresh-screenshots` like
fingerprints, from committed replays: the HUD in a level, a dark level, the transition's tally, the credits, the title,
the level editor, the pause screen, a surface (`oScreen`'s), the settings program. CI's media job runs it on the Mac
minis, which already render on the GPU.

It catches what only the renderer does: the `draw` crate's rasterization, blending, surfaces, font glyph placement,
`docs/quirks.md` §6. It costs the most, and covers a few frames, so it comes last: after layer 2, a pixel difference
is the renderer's, not the game's.

## Tickets

1. **851-3842** (done): tiles in the trace.
2. **851-3870, layer 1** (done, oracle patch 0040): draw-only instance fields and the drawing state, background
   layers and views in the trace. High: reached in every level (its background layers and views; `oWeb`'s alpha).
3. **851-3871, layer 2** (done, oracle patch 0041): the draw-call digest. High: the only check of what draw events
   draw. As built ([formats.md](../formats.md), "Draw calls"): neither side culls tiles, so every tile is a call on
   both; a view's and a surface draw's angle are left out (the port's commands have none, and Spelunky rotates
   neither); every frame is hashed, which costs `fidelity-check` about a fifth more.
4. **851-3872, layer 3** (done, oracle patch 0042): oracle screenshots and pixel checks of a committed set. Medium:
   after layer 2, it only checks the renderer. As built ([formats.md](../formats.md), "Screenshots"): 16 pictures
   of 7 replays, all exact; the settings program's 2x view is a pixel off (851-3967), so it's left out for now.
