# Quirks to preserve

The port is an exact replica of Spelunky Classic 1.1: the same state as the original, tick for tick, **bugs
included**. This is the living list of the bugs in the 1.1 source and the oddities of the GameMaker 8 runner
(and its renderer) that the port has to reproduce, plus the places where the reference oracle itself isn't GM8.
When you port something that looks wrong but is what 1.1 does, add it here.

Each entry says what happens, where (a GML location, or the runner behaviour), how we know, and which test pins
it; entries the `gm` runtime already reproduces also say where (§5). GML paths are relative to `assets/gml/`
(regenerate with `scripts/extract`); OpenGMK paths are under `gm8emulator/src/` at the commit pinned in
[`tools/oracle/common.sh`](../tools/oracle/common.sh). Port paths are relative to the crate's `src/` (`crates/gm`
in §5, `crates/audio` in §8).

**Status** is one of:

- **GML**: read in the 1.1 source. True by definition, but the runner rules it relies on may be assumed.
- **OpenGMK**: confirmed in OpenGMK's source (file named). OpenGMK models GM8; it isn't GM8.
- **Oracle**: verified against an oracle run (trace or screenshot).
- **DLL**: read in `supersound.dll`'s disassembly (PureBasic's Sound library over DirectSound).
- **Assumed**: neither yet. Verify before relying on it.

Most runner rules are "OpenGMK" until a trace confirms them; [runtime.md §14](design/runtime.md#14-verify-against-oracle)
lists the oracle checks still to do.

## Fixes go behind off-by-default options

Never change what 1.1 does by default, even when it's plainly a bug. A fix (or a deliberate deviation, such as
splitting the RNG stream for seeded levels) goes behind an option that is **off by default**, and with it off
the port must still match the oracle bit for bit. Name the option after the quirk, link its entry here, and
test both settings. Replays, traces and savestates must record which options were on, so a run with fixes enabled
is never compared against the oracle as if it were 1.1. No such options exist yet.

Where the oracle itself can't be GM8 (§7), the port follows a recorded decision instead; those aren't fixes.

## 1. Bugs and leftovers in the 1.1 source

| Quirk | Where | Status | Test |
|---|---|---|---|
| `rand(a, b)` writes `randNum` as an **instance variable** of the caller (or the `with` target), because it isn't declared `var`. 1,363 call sites, so nearly every instance gets a `randNum`; nothing reads it, but it shows up in `--fields` traces and in "variable exists" checks. | `scripts/rand.gml` | GML | test TODO |
| `instance_activate_region` is passed `x2, y2` where it takes `width, height`, so the reactivated region around the player is far larger than 32x32 and grows with the player's position. | `objects/oLevel/step.gml:94` | GML | test TODO |
| `SS_Unload` is bound to the DLL's `SS_Init` export, so the game-end handlers that "unload" SuperSound call its init again. What that does to the DLL is in §8. | `scripts/SS_Init.gml:11-13`; called from `other_game_end` of `oMusic`, `oScreen`, `core` | GML | see §8 |
| Dangling references to `config.exe` resources: `room_goto(rInit)` and `oInit.startGame = true`. `rInit` and `oInit` exist only in `config.gmk`, so reaching this code raises GML's unknown-variable error. | `objects/oKeyConfig/keypress_anykey.gml:54`, `objects/oJoyConfig/step.gml:67`, `objects/oOKButton/mouse_left_release.gml:3` | GML | test TODO |
| `global.sndStep1`/`sndStep2` are never assigned. They're read only in `oPlayer1` alarm 3, which nothing sets (its setters are commented out); reaching it would be an unknown-variable error. | `objects/oPlayer1/alarm_3.gml` | GML | test TODO |
| `oTemp` (solid) calls `sound_play` on `sndArrowTrap`, a deleted sound resource. Nothing creates `oTemp`. | `objects/oTemp/step.gml:8,19,30` | GML | test TODO |
| SuperSound demo leftovers: objects `core`, `mascara`, `mascara2`; room `room0`, the **last** room in the room order; two sprites in a `supersound` folder. `core` has an unknown mouse sub-event 12 and calls `show_message`, `show_menu`, `show_info` and `get_integer`. Nothing creates `core`. Keep them all, so resource ids and the room order match. | resource tree; [source-inventory §9](source-inventory.md#9-notable-surprises) | GML | test TODO |
| `collision_point(..., -1, -1)` 58 times: `-1` is false (below 0.5), so these checks are **not** precise and don't exclude the caller. | e.g. `scripts/characterStepEvent.gml:750,758` | GML + OpenGMK (`gml/value.rs` `is_truthy`) | test TODO |
| `abs(xVel) > 2 or abs(yVel) > 2 and collision_rectangle(...) and enemyID == 0` parses as `((a or b) and c) and d` (see §2), which decides when a sticky bomb latches onto an enemy or damsel. Both speed checks are gated by the collision test; with C precedence a bomb moving fast sideways would latch onto the nearest enemy anywhere. | `objects/oItem/step.gml:219,237` | GML + OpenGMK | [`gm8_precedence`](../crates/gml/tests/parser.rs) |
| Conditions that call `rand` on the right of `and` draw from the RNG even when the left is false (see §2): a sleeping man trap near the view draws every step, and level generation draws for lamps and scarabs on lit levels too. | `objects/oManTrap/step.gml:158`, `scripts/scrEntityGen.gml:61-62` | GML + OpenGMK | test TODO |
| `persistent` is set on the parent objects `oCharacter` and `oEnemy`, which are never instantiated; children don't inherit the flag, so it has no effect. | object properties | GML; inheritance rule Assumed | test TODO |

## 2. GML language semantics

The full list, with counts, is in [translation.md](design/translation.md#evidence-gm8-semantics-the-translation-must-reproduce).
These are the ones that read as bugs to a modern eye.

| Quirk | Where | Status | Test |
|---|---|---|---|
| `and`, `or` and `xor` evaluate **both** operands, side effects included (no short-circuit). With Rust's `&&`, the RNG stream, and every later level, diverges. | runner; 2,255 operators, 68 with `rand`/`random` on the right | OpenGMK (`gml/runtime.rs` `Node::Binary`) | test TODO (transpiler) |
| `and`, `or`, `xor` share **one** precedence level, left-associative; bitwise operators bind tighter than comparisons. | runner; 3 sites change meaning | OpenGMK (`gml-parser` `get_op_precedence`) | [`gm8_precedence`](../crates/gml/tests/parser.rs) |
| A real is true when it is **at least 0.5** (`0.5` true, `0.49` false, NaN false); strings are always false, but `not "x"` is an error. | runner | OpenGMK (`gml/value.rs` `is_truthy`, `Value::not`) | [`math::tests::truthiness_threshold_is_half_inclusive`](../crates/gm/src/math.rs), [`value::tests::truthiness_and_logic`](../crates/gm/src/value.rs) |
| Real comparisons use a tolerance of **1e-13**: `0.1 + 0.2 == 0.3`. `a != b` is `\|a - b\| >= 1e-13`, so with a NaN both `==` and `!=` are false. | runner; 5,429 comparisons | OpenGMK (`math.rs` `CMP_EPSILON`, `gml/value.rs`) | [`math::tests::comparisons_use_epsilon`](../crates/gm/src/math.rs), [`nan_compares_false_everywhere`](../crates/gm/src/math.rs) |
| Comparing a real with a string is an error, not false; strings compare bytewise (`"B" < "a"`). | runner | OpenGMK (`gml/value.rs` `gml_cmp_impl!`) | [`value::tests::comparisons`](../crates/gm/src/value.rs) |
| `switch` matches with exact equality (not the 1e-13 tolerance; a real never matches a string) and falls through. | runner; 107 switches | OpenGMK (`gml/runtime.rs` `Instruction::Switch`, `gml/value.rs` `almost_equals`) | `crates/gm/src/ops.rs` `switch_matching_is_exact` |
| A script with no `return` on some path returns the **last assigned or called value**, not 0. `scrGetKey`, `scrGetJoy` and `scrGetChar` are used as values (47 call sites). | runner | OpenGMK (`gml/runtime.rs` `return_value`) | test TODO |
| `obj.v op= x` reads `v` from the **first** instance of `obj` and writes the result to **all** of them, evaluating the base twice. Plain `obj.v = x` writes every instance. | runner; 15 sites, e.g. `oMoonRoom.baskets += 1` | OpenGMK (`gml/compiler.rs` `make_modify_instruction`) | `crates/gm/tests/vars.rs` `target_access_reads_the_first_and_writes_all` |
| `op=` on user, global and local variables ignores a type error and keeps the old value; on built-ins (`argumentN` included) it's an error, so `s -= 1` on a string does nothing. Port: `gm` `ops.rs` `lenient`. | runner | OpenGMK (`gml/compiler.rs` `type_unsafe`) | `crates/gm/src/ops.rs` `lenient_keeps_the_old_value_on_type_errors`; [`strict_and_lenient_compound_assignment`](../crates/gml/examples/transpile/main.rs) (prototype) |
| Evaluation order: assignment target, then array index, then value; arguments and operands left to right. Matters wherever `rand` appears twice in one statement. | runner | OpenGMK (`gml/runtime.rs`) | test TODO |
| `break` and `continue` inside `with` act on the `with` loop, not an enclosing loop. | runner; e.g. `scripts/moveTo.gml` | OpenGMK (`gml/runtime.rs` `Instruction::With`) | test TODO |
| `real * string` repeats the string `round(real)` times; `string * real` is an error. | runner (unused by the game) | OpenGMK (`gml/value.rs` `Value::mul`) | [`value::tests::real_times_string_repeats`](../crates/gm/src/value.rs) |
| Built-in functions never fail on types: a string argument reads as 0 where a real is expected, a real as `""` where a string is. | runner | OpenGMK (`gml/value.rs` `From<Value>`) | [`value::tests::function_argument_coercion`](../crates/gm/src/value.rs) |

## 3. Numbers and strings

| Quirk | Where | Status | Test |
|---|---|---|---|
| `round` is round-half-to-even (`round(2.5) == 2`), and a negative value that rounds to 0 gives +0. Integer arguments to built-ins are rounded the same way, then wrapped to 32 bits through `i64`. | runner; 78 `round` calls, every integer argument | OpenGMK (`math.rs` `ieee_round`, `Real::to_i32`) | [`math::tests::round_is_bankers`](../crates/gm/src/math.rs) |
| `a mod b` is `a - trunc(a / b) * b`: the sign follows `a`, and it's computed through a division, so it can differ from IEEE `fmod` in the last bits (`1 mod 0.1 == 0`). | runner; 5 uses | OpenGMK (`gml/value.rs` `Value::modulo`) | [`math::tests::div_floors_and_mod_truncates`](../crates/gm/src/math.rs) |
| Division by zero isn't an error: it gives ±inf or NaN. `sqrt` of a negative is an error. | runner | OpenGMK (`gml/value.rs` `Value::div`, `gml/kernel.rs` `sqrt`) | [`div_floors_and_mod_truncates`](../crates/gm/src/math.rs), [`frac_sign_sqr_sqrt`](../crates/gm/src/math.rs) |
| `sign` uses the comparison tolerance (`sign(1e-14) == 0`); `frac` keeps the sign (`frac(-1.25) == -0.25`). | runner | OpenGMK (`gml/kernel.rs`) | [`math::tests::frac_sign_sqr_sqrt`](../crates/gm/src/math.rs) |
| `point_direction` is `atan2` in degrees wrapped with `rem_euclid(360)`, so a tiny negative angle gives exactly **360**, not 0. `point_distance` is `sqrt(dx*dx + dy*dy)`, not `hypot`. | runner; 12 and 39 calls | OpenGMK (`gml/kernel.rs`) | [`math::tests::angles`](../crates/gm/src/math.rs) |
| `make_color_rgb` doesn't clamp, so out-of-range components bleed into the next channel. | runner | OpenGMK (`gml/kernel.rs`) | [`math::tests::make_color_rgb_packs_without_clamping`](../crates/gm/src/math.rs) |
| `string(x)` prints a whole number with no decimals and anything else with exactly two, rounded from the exact binary value (`string(0.125) == "0.12"`, `string(2.999) == "3.00"`); -0 prints as `"-0"`. | runner; 185 calls, the HUD and save files | OpenGMK (`gml/value.rs` `Value::repr`) | [`string::tests::real_to_string_uses_zero_or_two_decimals`](../crates/gm/src/string.rs) |
| `real(s)` trims whitespace (including 0xA0), reads `""` as 0, accepts Rust's float grammar (`"1e3"`, `"inf"`), and errors on anything else (`"12abc"`). | runner; 69 calls, config and save files | OpenGMK (`gml/kernel.rs` `real`) | [`string::tests::string_to_real_trims_and_parses`](../crates/gm/src/string.rs) |
| Strings are Windows-1252 **bytes** with 1-based positions. `string_char_at(s, 0)` reads the first byte; `string_delete` with a position below 1 does nothing; `string_insert` clamps its position; `string_upper`/`lower` change ASCII only. | runner | OpenGMK (`gml/kernel.rs`, GM 8.0 branches) | [`string.rs` tests](../crates/gm/src/string.rs) (`positions_are_one_based_bytes`, `string_delete_edges`, `string_insert_clamps`, `case_mapping_is_ascii_only`) |

## 4. Randomness

| Quirk | Where | Status | Test |
|---|---|---|---|
| The generator is Delphi's `Random`: `seed = seed * 0x08088405 + 1 (mod 2^32)`, and `random(x)` is `(seed as unsigned) / 2^32 * x`. `floor(random(n))` is exact for the game's integer ranges. | runner | OpenGMK (`gml/rand.rs`) + Oracle (per-frame seeds, `oDark` values) | [`rng::tests::oracle_rand_values_from_replay_seed`](../crates/gm/src/rng.rs), [`oracle_per_frame_seeds_are_reachable`](../crates/gm/src/rng.rs), [`random_is_in_range_and_rand_script_is_inclusive`](../crates/gm/src/rng.rs) |
| One global stream: level generation, gameplay and cosmetic particles share it, so a single extra or missing call anywhere changes every later level. The game never seeds it. | runner; [source-inventory §6](source-inventory.md#6-randomness) | OpenGMK | test TODO (needs the runtime) |
| At startup the runner randomizes, then draws `Random(100000)` (one more per name that already exists) to name its temp directory, before any game code. OpenGMK records the replay's start seed **after** those draws, so replays seed with the recorded value. | runner | OpenGMK (`game.rs` `Game::launch`, `make_temp_dir`) | [`rng::tests::startup_consumes_one_draw`](../crates/gm/src/rng.rs) |

## 5. Runner: frame, instances and events

The whole frame is in [runtime.md §1](design/runtime.md#1-the-frame). These are the rules most likely to be
"tidied up" by mistake. The **Port** column says where `crates/gm/src/` reproduces each one, and tests are in
`crates/gm/tests/` unless a file is named.

| Quirk | Where | Status | Port | Test |
|---|---|---|---|---|
| **Animation end runs after draw**: `image_index` advances and animation-end events fire at the very end of the frame, so an instance destroyed by its animation-end handler was already drawn on its last frame and goes away next frame. | runner; 51 animation-end events | OpenGMK (`game.rs` `Game::frame`) | `frame.rs` `World::step` | `frame.rs` `animation_end_comes_after_draw` |
| An instance whose sprite doesn't exist has 0 frames, so with a non-negative `image_speed` its animation-end event fires every frame. | runner | OpenGMK (`game.rs` `Game::frame`, animation) | `frame.rs` `animate` | test TODO |
| **A room change stops the frame after the stage that asked for it.** While `room_goto`, `room_restart` or `game_end` is pending no event runs at all (`instance_destroy` destroys without the destroy event, for one), so the rest of that stage's events are skipped, and so are the later stages. Work in the same stage that isn't an event still finishes: an animation-end handler's `room_goto` (`oPlayer1` at a level exit) doesn't stop the other instances' `image_index` advancing. The switch happens between frames. | runner; e.g. `objects/oPlayer1/other_animation_end.gml` | OpenGMK (`game/events.rs` `run_instance_event`, `game.rs` `Game::frame`) | `frame.rs` `World::step`, `kernel.rs` `run_handler` | `frame.rs` `a_room_change_stops_the_frame_after_the_stage`, `events.rs` `no_event_runs_while_a_room_change_is_pending` |
| **Draw is part of the simulation**: draw events write game state, view following moves `view_xview`/`view_yview` (which `oLevel` culls by), and the depth sort reorders instance iteration. A headless run must still run the draw stage. | runner; 36 draw events | OpenGMK (`game/draw.rs`) | `frame.rs` `redraw` | `frame.rs` `drawing_sorts_by_depth_and_rebuilds_change_object_order` |
| Within a stage, instances run **grouped by object id ascending**, not by instance id or depth; a new instance of a higher object id runs its step in the same frame, one of a lower id waits. | runner | OpenGMK (`game/events.rs` `run_object_event`) | `frame.rs` | `frame.rs` `new_instances_step_this_frame_only_if_their_object_comes_later` |
| A per-object loop can visit an instance created during it: the loop stops at the starting count, but each step scans past destroyed or deactivated entries into newly appended ones. | runner | OpenGMK (`instancelist.rs` `nb_il_iter`) | `list.rs` `InstIter` | `frame.rs` `an_object_loop_can_reach_an_instance_created_during_it` |
| If a (de)activation rebuilds the per-object lists in the middle of a loop over one, the loop continues at the same index of the rebuilt list, so it can skip or repeat instances. `oLevel` (de)activates every step. | runner | OpenGMK (`instancelist.rs` `ObjectIter`, `IdentityIter` keep `count` "to match GM8 weird behaviour") | `list.rs` `InstIter` | test TODO |
| A room-placed instance destroyed or deactivated by an earlier create event of the same room load gets no create event. | runner | OpenGMK (`game.rs` `load_room`) | `world.rs` `start_room` | `events.rs` `room_start_skips_create_events_of_instances_already_gone` |
| Intersect-boundary events fire for instances entirely outside the room too: the test is "not entirely inside". Without a mask, both it and outside-room use the same position test. | runner; 11 outside events | OpenGMK (`game/events.rs` `run_bound_events`) | `frame.rs` `run_bound_events` | `frame.rs` `intersect_boundary_fires_outside_the_room_too` |
| Negative friction does nothing at speed 0 but speeds a moving instance up. | runner | OpenGMK (`game/movement.rs` `process_speeds`) | `frame.rs` `motion` | `frame.rs` `negative_friction_only_acts_on_moving_instances` |
| Pressing a key that is already down reports it pressed again (as keyboard auto-repeat would); releasing any mouse button sets `mouse_button` to 0 even if another is still down. | runner | OpenGMK (`input.rs` `button_press`, `mouse_release`) | `input.rs` `InputState::apply` | `frame.rs` `input_follows_opengmk` |
| Default drawing (objects without a draw event) still happens after a draw event requested a room change; GM8 then doesn't show that frame. | runner | OpenGMK (`game/draw.rs` `draw_instance`, `game.rs` `Game::frame`) | `frame.rs` `redraw`, `World::presents` | test TODO |
| **Alarms** count down only for objects that have that alarm's event (holders), so `alarm[n]` set on an object without an alarm-`n` event never changes. The stage goes alarm 0 to 11, decrementing and firing at 0: `alarm[n] = 0` never fires (it's counted down to -1 first), `alarm[n] = 1` set in a step event fires next frame, but set in a lower-numbered alarm's event it fires in the same stage (`oMoonRoom` alarm 0 sets `alarm[11] = 1`). Deactivated instances' alarms freeze. | runner; 214 alarm writes; `objects/oMoonRoom/alarm_0.gml:19` | OpenGMK (`game/events.rs` `run_alarms`) | `frame.rs` `run_alarms` | `frame.rs` `alarms_fire_by_alarm_then_object_then_instance`, `deactivated_instances_are_skipped_everywhere` |
| The keyboard stages run the "no key" and "any key" events **twice**: once in the pass over key codes (codes 0 and 1) and once more at the end. | runner; 22 key-press events | OpenGMK (`game/events.rs` `run_keyboard_events` and siblings) | `frame.rs` `run_key_events` | test TODO |
| **Destroying** an instance twice runs its destroy event twice. Code after `instance_destroy()` keeps running, and that code can still read the instance's variables until the end-of-step cleanup; `with`, `obj.v` and other lookups skip it from the moment it's destroyed. | runner; 207 calls with code after them | OpenGMK (`gml/kernel.rs` `instance_destroy`) | `kernel.rs` `destroy` | `events.rs` `destroying_inside_with_skips_the_destroyed_and_keeps_running` |
| The `instance_count` built-in counts destroyed instances until the end-of-step cleanup. | runner | OpenGMK (`instancelist.rs` `count_all`) | `access.rs` | test TODO |
| `instance_exists` and `instance_number` treat 100000 as an object index; `with` and `obj.v` treat it as an instance id. | runner | OpenGMK (`gml/kernel.rs`) | `kernel.rs` | test TODO |
| `instance_(de)activate_object(self/other/all)` and `instance_(de)activate_all` rebuild the per-object instance lists even when nothing changed, which reorders them by depth. | runner; `oLevel` (de)activates every step | OpenGMK (`gml/kernel.rs`) | `kernel.rs` `set_active_object` | `frame.rs` `drawing_sorts_by_depth_and_rebuilds_change_object_order` |
| With a negative `image_xscale` the bounding box is one pixel wider than the sprite's mask, because the inclusive-edge `- 1` is applied before the flip. | runner; 26 `image_xscale` writes | OpenGMK (`instance.rs` `update_bbox`) | `instance.rs` `bbox_for` | `instance.rs` `bbox_scales_and_flips` |
| Reading an array element that was never set gives 0 if a higher element of the same row was set, and is an error otherwise. | runner | OpenGMK (`instance.rs` `Field::get`) | `var.rs` `Slot::get` | `vars.rs` `arrays_follow_gm8_index_rules` |
| **Solid collisions**: if either instance is solid, both move back to their previous positions *before* the two collision events; afterwards both get `x += hspeed; y += vspeed` again, and if they still overlap they move back a second time. | runner; 12 events involve a solid | OpenGMK (`game/events.rs` `run_collisions`) | not yet | test TODO (runtime.md §14 item 4) |

## 6. Renderer (GM8 on Direct3D 8)

From the [`draw` crate docs](../crates/draw/src/lib.rs). Pixel output isn't game state, but the oracle comparison
of screenshots depends on these.

| Quirk | Where | Status | Test |
|---|---|---|---|
| A view's **port is ignored while a surface is the target** (D3D8's viewport call does nothing then), so the view is stretched over the whole surface. `oScreen`'s view-scaling trick relies on it. | runner; `oScreen` | OpenGMK (`render/opengl.rs`, "DX8's viewport function doesn't do anything if a surface is set") | [`surfaces_ignore_the_port_and_frames_keep_their_start_state`](../crates/app/src/render.rs) |
| **Alpha is blended with the colour factors** (D3D8 had no separate alpha blend), so drawing on a surface with `bm_normal` leaves translucent alpha, which the game's "alpha fix" rectangle (`bm_src_color, bm_one` in black) repairs. | runner; `oScreen` | OpenGMK (`render/opengl.rs` uses `glBlendFunc`, not the separate form) | test TODO |
| `draw_rectangle`'s **far corner is inclusive**: `draw_rectangle(0, 0, 10, 10)` fills 11x11 pixels. Sprites, backgrounds, tiles and surfaces get GM's half-pixel shift instead; rectangles, lines and circles don't. | runner | OpenGMK (`render/opengl.rs` `draw_rectangle`) | test TODO |
| **Depth ties**: instances and tiles each keep a persistent list that is *stably* re-sorted by depth (highest first) at every draw, so equal depths keep their previous order (creation order unless a depth changed). At equal depth an instance draws before a tile. | runner | OpenGMK (`InstanceList::draw_sort`, `Game::draw_view`) + Oracle (rTutorial screenshot, 851-3130) | [`highest_depth_first_with_creation_order_ties`](../crates/app/src/preview.rs), [`instances_beat_tiles_on_equal_depth`](../crates/app/src/preview.rs) |
| **Drawing state persists across steps**: a blend mode set by `draw_set_blend_mode` stays until changed, and `oScreen` leaves its surface as the target at the end of every step for the next draw stage and begin step. | runner; `oScreen` | OpenGMK (nothing resets it per frame) | [`draw_state_follows_targets_views_and_blends`](../crates/draw/src/lib.rs), [`surfaces_ignore_the_port_and_frames_keep_their_start_state`](../crates/app/src/render.rs) |
| Unknown `bm_*` factors are `bm_zero`; `draw_set_blend_mode` with an unknown mode is `bm_normal`. | runner | OpenGMK (`gml/kernel.rs`) | [`gml_blend_values`](../crates/draw/src/lib.rs) |
| Sprite fonts are monospaced: every character advances by the cell width plus the separation; a byte the font lacks draws as a space, and if there's no space either, only the cursor moves. `#` starts a line, `\#` is a literal `#`. | runner; `font_add_sprite` fonts | OpenGMK (`create_chars_from_sprite`, `draw_string`) | [`draw` text tests](../crates/draw/src/lib.rs) (`missing_characters_become_spaces`, `missing_space_only_advances`, `hash_breaks_lines_unless_escaped`) |

## 7. Where the oracle isn't GM8

The oracle (OpenGMK in Docker, [tools/oracle/README.md](../tools/oracle/README.md#caveats)) is our best evidence,
not proof. These are known differences from 1.1 on Windows; trace diffs must allow for them.

| Caveat | Effect | Status | Test |
|---|---|---|---|
| OpenGMK is a reimplementation, and models Delphi's `Double` as plain IEEE `f64` arithmetic. The real runner ran on the x87 FPU, whose extended-precision intermediates can round differently. | Possible last-bit differences from real GM8 that neither the oracle nor the port reproduces. | Assumed | none |
| **Transcendentals.** GM8 computed `sin`, `cos`, `tan`, `arctan` and `atan2` (for `point_direction` and motion) on the x87, which nobody reproduces. Unpatched, OpenGMK would use its platform's libm: glibc on arm64 is 1 ulp off the correctly rounded result on about 0.1-0.25% of the inputs the game produces, and amd64 glibc differs again. **Decision (851-3167, 851-3173):** the port is correctly rounded on every platform (vendored pxfm), and the oracle's patch 0007 routes OpenGMK's `Real` trigonometry through the same crate. | None between port and oracle: they compute the same bits. Both may differ from real GM8 in the last bit. | Oracle (`tools/math-probe` built in the oracle image matches native) | [`math::tests::transcendentals_are_correctly_rounded`](../crates/gm/src/math.rs) |
| **SuperSound is stubbed**: `SS_Init` returns `"Yes"` and every other call returns 0, so the `global.snd*`/`global.mus*` handles are 0 instead of the DLL's handle strings, and `SS_IsSoundPlaying` is always false. | Play is unaffected. The port's `audio` crate has `Mode::OracleCompat` to answer like the stub for oracle diffs (§8). | OpenGMK (`game/external.rs` dummy externals, enabled by patch `0003`) | `crates/audio/src/ss.rs` `oracle_compat_answers` |
| **The highscore table is ours** (the real one is on disk); its fresh contents aren't verified. The game only compares values with its offsets and never reads names. | Unlikely to matter. | Assumed | none |
| **No joystick**: OpenGMK reports none, so `oGamepad` sees nothing even though the shipped `settings.cfg` enables the gamepad. | Gamepad input can't be checked against the oracle. | OpenGMK | none |
| Unimplemented functions panic: `show_message`, `show_menu`, `show_info` and `get_integer`, all only in the demo object `core`. | Unreachable in play. | OpenGMK | none |

## 8. SuperSound DLL

All sound goes through `supersound.dll`. Its behaviour comes from its disassembly and the `SS_*` GML wrappers;
anything inferred rather than observed is marked "verify" in the `audio` crate docs. Port: "model" is
`model.rs` (`AudioModel`), "wrappers" is `ss.rs` (`SuperSound::call`); tests are in those files.

| Quirk | Where | Status | Port | Test |
|---|---|---|---|---|
| `SS_Unload` calls the DLL's `SS_Init` (§1), which starts a new sound table: every handle stops working. The DLL's paused list survives, so a sound paused before still reads as paused. | `scripts/SS_Init.gml`; DLL `SS_Init` | GML + DLL | model `init` | `init_again_invalidates_every_handle`, `the_paused_list_survives_init` |
| `SS_LoadSound` returns `"No"` for a missing file, not `"0"` as its wrapper says. | DLL `SS_LoadSound` | DLL | model `load_sound` | `faithful_answers` |
| `SS_SetSoundPosition` returns 2 on success and 1 for an unknown handle: the wrapper adds 1 to the DLL's `"1"`/`"0"`, though its comment promises 1 and 0. | `scripts/SS_SetSoundPosition.gml` | GML | wrappers | `faithful_answers` |
| `SS_GetSoundVol` of an unknown handle is 9999: the DLL's `"-1"` plus the wrapper's 10000. | `scripts/SS_GetSoundVol.gml`; DLL `SS_GetSoundVol` | GML + DLL | wrappers | `faithful_answers` |
| `SS_SetSoundVol`, `SS_SetSoundPan`, `SS_SetSoundFreq` and `SS_SetSoundPosition` report success for any known handle, even when DirectSound rejects the value and nothing changes (volume outside -10000..0, pan outside ±10000, frequency outside 100..200000 except 0, a position past the buffer). | DLL setters ignore the `HRESULT` | DLL | model `set_sound_*` | `setters_ignore_values_directsound_rejects` |
| Numeric arguments reach the DLL as GML `string()`s and are read with PureBasic's `Val()`, which keeps the integer part: the music volume 8666.67 becomes `"-1333.33"` and then -1333. | DLL (every numeric argument) | DLL | `ss.rs` `pb_val` | `val_parses_like_purebasic` |
| A static (WAV) sound that played to its end reads position 0. | DirectSound | Assumed (verify) | model `tick` | `a_static_sound_stops_at_its_end_and_rewinds` |
| `SS_ResumeSound` doesn't start a stopped sound, despite its wrapper's comment: the DLL only resumes sounds in its paused list. | DLL `SS_ResumeSound` | DLL | model `resume_sound` | `pause_and_resume` |
| A playing stream (the music) always reports looping, because its DirectSound ring buffer loops even when the track plays once. Its position and length are the 64 KiB ring's. So a paused stream resumes looping, from the start of the file with its ring cursor moved. | DLL streaming | DLL | model | `streams_report_their_ring_buffer`, `a_paused_stream_resumes_looping_from_its_ring_cursor` |
| A stream that plays once ends at the next half-buffer boundary after the end of the file, padded with silence. | DLL refill thread | DLL | model `tick` | `a_stream_stops_at_the_half_buffer_after_its_end` |
| Return types are inconsistent: for a real (invalid) handle `SS_GetSoundVol` returns the string `"-1"` and `SS_GetSoundPan` the string `"0"`, the other queries reals. | `scripts/SS_*.gml` | GML | wrappers | `faithful_answers` |
| Under the oracle, OpenGMK's stub makes `SS_LoadSound` return 0, so every wrapper takes its `is_real` branch: `SS_IsSoundPlaying`, `SS_IsSoundLooping`, `SS_IsSoundPaused`, `SS_GetSoundFreq`, `SS_GetSoundPosition`, `SS_PauseSound` and `SS_ResumeSound` answer -1. Reproduced by `Mode::OracleCompat`, for oracle diffs only (see §7). | OpenGMK `external.rs` `should_dummy`; `scripts/SS_*.gml` | OpenGMK + GML | wrappers | `oracle_compat_answers` |
