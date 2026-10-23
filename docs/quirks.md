# Preserved quirks

Behaviour the port keeps on purpose although it looks wrong, because the original does it. Each entry says
where it comes from and where the port reproduces it. "Runner" quirks are GameMaker 8's (sourced from
OpenGMK, so each is **verify against oracle** until a trace confirms it); "game" quirks are Spelunky's own.

## Runner: instances and events (`crates/gm`)

| Quirk | Source | Port |
|---|---|---|
| Destroying an instance twice runs its destroy event twice. Code after `instance_destroy()` keeps running, and the instance's variables stay readable until the end-of-step cleanup. | OpenGMK `gml/kernel.rs` `instance_destroy` | `kernel.rs` `destroy` |
| `instance_exists` and `instance_number` treat 100000 as an object index; `with` and `obj.v` treat it as an instance id. | OpenGMK `gml/kernel.rs` | `kernel.rs` |
| With a negative `image_xscale` the bounding box is one pixel wider than the sprite's mask, because the inclusive-edge `- 1` is applied before the flip. | OpenGMK `instance.rs` `update_bbox` | `instance.rs` `bbox_for` |
| Reading an array element that was never set gives 0 if a higher element of the same row was set, and is an error otherwise. | OpenGMK `instance.rs` `Field::get` | `var.rs` `Slot::get` |
| The `instance_count` built-in counts destroyed instances until the end-of-step cleanup. | OpenGMK `instancelist.rs` `count_all` | `access.rs` |
| A per-object loop can visit an instance created during it: the loop stops at the starting count, but each step scans past destroyed or deactivated entries into newly appended ones. | OpenGMK `instancelist.rs` `nb_il_iter` | `list.rs` `InstIter` |
| If a (de)activation rebuilds the per-object lists in the middle of a loop over one, the loop continues at the same index of the rebuilt list, so it can skip or repeat instances. | OpenGMK `instancelist.rs` (`ObjectIter`, `IdentityIter` keep `count` "to match GM8 weird behaviour") | `list.rs` `InstIter` |
| While a room change is pending, no event runs at all: the rest of the current stage is skipped, `instance_destroy` destroys without the destroy event, and so on. | OpenGMK `game/events.rs` `run_instance_event` | `kernel.rs` `run_handler` |
| `v op= x` on a user variable, global or local keeps the old value when the operator fails on the types (`s -= 1` on a string does nothing); on built-ins and arguments it is an error. | OpenGMK `gml/compiler.rs` (`type_unsafe`) | `ops.rs` `lenient` |
| A room-placed instance destroyed or deactivated by an earlier create event of the same room load gets no create event. | OpenGMK `game.rs` `load_room` | `world.rs` `start_room` |
