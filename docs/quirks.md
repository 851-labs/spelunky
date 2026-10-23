# Preserved quirks

Behaviour the port keeps on purpose although it looks wrong, because the original does it. Each entry says
where it comes from and where the port reproduces it. "Runner" quirks are GameMaker 8's (sourced from
OpenGMK, so each is **verify against oracle** until a trace confirms it); "game" quirks are Spelunky's own.

## Runner: instances and events (`crates/gm`)

| Quirk | Source | Port |
|---|---|---|
| An instance whose sprite doesn't exist has 0 frames, so with a non-negative `image_speed` its animation end event fires every frame. | OpenGMK `game.rs` `Game::frame` (animation) | `frame.rs` `animate` |
| Alarms only count down for objects that have that alarm's event (holders), so `alarm[n]` set on an object without an alarm-`n` event never changes. `alarm[n] = 0` never fires: it is counted down to -1 first. | OpenGMK `game/events.rs` `run_alarms` | `frame.rs` `run_alarms` |
| The keyboard stages run the "no key" and "any key" events twice: once in the pass over key codes (codes 0 and 1) and once more at the end. | OpenGMK `game/events.rs` `run_keyboard_events` and siblings | `frame.rs` `run_key_events` |
| Destroying an instance twice runs its destroy event twice. Code after `instance_destroy()` keeps running, and the instance's variables stay readable until the end-of-step cleanup. | OpenGMK `gml/kernel.rs` `instance_destroy` | `kernel.rs` `destroy` |
| `instance_exists` and `instance_number` treat 100000 as an object index; `with` and `obj.v` treat it as an instance id. | OpenGMK `gml/kernel.rs` | `kernel.rs` |
| `instance_(de)activate_object(self/other/all)` and `instance_(de)activate_all` rebuild the per-object instance lists even when nothing changed, which reorders them by depth. | OpenGMK `gml/kernel.rs` | `kernel.rs` `set_active_object` |
| With a negative `image_xscale` the bounding box is one pixel wider than the sprite's mask, because the inclusive-edge `- 1` is applied before the flip. | OpenGMK `instance.rs` `update_bbox` | `instance.rs` `bbox_for` |
| Reading an array element that was never set gives 0 if a higher element of the same row was set, and is an error otherwise. | OpenGMK `instance.rs` `Field::get` | `var.rs` `Slot::get` |
| The `instance_count` built-in counts destroyed instances until the end-of-step cleanup. | OpenGMK `instancelist.rs` `count_all` | `access.rs` |
| A per-object loop can visit an instance created during it: the loop stops at the starting count, but each step scans past destroyed or deactivated entries into newly appended ones. | OpenGMK `instancelist.rs` `nb_il_iter` | `list.rs` `InstIter` |
| If a (de)activation rebuilds the per-object lists in the middle of a loop over one, the loop continues at the same index of the rebuilt list, so it can skip or repeat instances. | OpenGMK `instancelist.rs` (`ObjectIter`, `IdentityIter` keep `count` "to match GM8 weird behaviour") | `list.rs` `InstIter` |
| While a room change is pending, no event runs at all: the rest of the current stage is skipped, `instance_destroy` destroys without the destroy event, and so on. | OpenGMK `game/events.rs` `run_instance_event` | `kernel.rs` `run_handler` |
| `v op= x` on a user variable, global or local keeps the old value when the operator fails on the types (`s -= 1` on a string does nothing); on built-ins and arguments it is an error. | OpenGMK `gml/compiler.rs` (`type_unsafe`) | `ops.rs` `lenient` |
| A room-placed instance destroyed or deactivated by an earlier create event of the same room load gets no create event. | OpenGMK `game.rs` `load_room` | `world.rs` `start_room` |
| Intersect-boundary events fire for instances entirely outside the room too: the test is "not entirely inside". Without a mask, both it and outside-room use the same position test. | OpenGMK `game/events.rs` `run_bound_events` | `frame.rs` `run_bound_events` |
| Negative friction does nothing at speed 0 but speeds a moving instance up. | OpenGMK `game/movement.rs` `process_speeds` | `frame.rs` `motion` |
| Pressing a key that is already down reports it pressed again (as keyboard auto-repeat would); releasing any mouse button sets `mouse_button` to 0 even if another is still down. | OpenGMK `input.rs` `button_press`, `mouse_release` | `input.rs` `InputState::apply` |
| Default drawing (objects without a draw event) still happens after a draw event requested a room change; GM8 then doesn't show that frame. | OpenGMK `game/draw.rs` `draw_instance`, `game.rs` `Game::frame` | `frame.rs` `redraw`, `World::presents` |

## DLL: supersound.dll (`crates/audio`)

All sound goes through `supersound.dll`. Its behaviour comes from its disassembly (PureBasic's Sound library over
DirectSound) and the `SS_*` GML wrappers; anything inferred rather than observed is marked "verify" in the crate
docs. "Model" is `model.rs` (`AudioModel`), "wrappers" is `ss.rs` (`SuperSound::call`).

| Quirk | Source | Port |
|---|---|---|
| `SS_Unload` calls the DLL's `SS_Init` (copy-paste bug in its binding), which starts a new sound table: every handle stops working. The DLL's paused list survives, so a sound paused before still reads as paused. | `SS_Init.gml`; DLL `SS_Init` | model `init` |
| `SS_LoadSound` returns `"No"` for a missing file, not `"0"` as its wrapper says. | DLL `SS_LoadSound` | model `load_sound` |
| `SS_SetSoundPosition` returns 2 on success and 1 for an unknown handle: the wrapper adds 1 to the DLL's `"1"`/`"0"`, though its comment promises 1 and 0. | `SS_SetSoundPosition.gml` | wrappers |
| `SS_GetSoundVol` of an unknown handle is 9999: the DLL's `"-1"` plus the wrapper's 10000. | `SS_GetSoundVol.gml`; DLL `SS_GetSoundVol` | wrappers |
| `SS_SetSoundVol`, `SS_SetSoundPan`, `SS_SetSoundFreq` and `SS_SetSoundPosition` report success for any known handle, even when DirectSound rejects the value and nothing changes (volume outside -10000..0, pan outside ±10000, frequency outside 100..200000 except 0, a position past the buffer). | DLL setters ignore the `HRESULT` | model `set_sound_*` |
| Numeric arguments reach the DLL as GML `string()`s and are read with PureBasic's `Val()`, which keeps the integer part: the music volume 8666.67 becomes `"-1333.33"` and then -1333. | DLL (every numeric argument) | `ss.rs` `pb_val` |
| A static (WAV) sound that played to its end reads position 0 (verify). | DirectSound | model `tick` |
| `SS_ResumeSound` doesn't start a stopped sound, despite its wrapper's comment: the DLL only resumes sounds in its paused list. | DLL `SS_ResumeSound` | model `resume_sound` |
| A playing stream (the music) always reports looping, because its DirectSound ring buffer loops even when the track plays once. Its position and length are the 64 KiB ring's. So a paused stream resumes looping, from the start of the file with its ring cursor moved. | DLL streaming | model |
| A stream that plays once ends at the next half-buffer boundary after the end of the file, padded with silence. | DLL refill thread | model `tick` |
| Return types are inconsistent: for a real (invalid) handle `SS_GetSoundVol` returns the string `"-1"` and `SS_GetSoundPan` the string `"0"`, the other queries reals. | `SS_*.gml` | wrappers |
| Under the oracle, OpenGMK's stub makes `SS_LoadSound` return 0, so every wrapper takes its `is_real` branch: `SS_IsSoundPlaying`, `SS_IsSoundLooping`, `SS_IsSoundPaused`, `SS_GetSoundFreq`, `SS_GetSoundPosition`, `SS_PauseSound` and `SS_ResumeSound` answer -1. Reproduced by `Mode::OracleCompat`, for oracle diffs only. | OpenGMK `external.rs` `should_dummy`; `SS_*.gml` | wrappers |
