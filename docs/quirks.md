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
