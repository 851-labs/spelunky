# Instance model and GM8 event loop (851-3126, design)

This is the design of the GameMaker 8 runtime that the translated game runs on: the `gm` crate's world,
instances, variables, events and frame loop, and how the `game` crate plugs in. It goes with
[translation.md](translation.md), which decides that all game logic is transpiled from GML.

**Goals.** Tick-for-tick the same state as GM8, bugs included. The world is a plain value: deterministic,
`Clone` for savestates, `Send` for parallel rollouts, no globals, no wall-clock time, no hidden randomness.
Thousands of headless ticks per second per core for a full level.

**Sources.** Runner behaviour isn't in the game's source. Every runner claim below cites OpenGMK
(<https://github.com/OpenGMK/OpenGMK>, read at commit `9e4282e`; paths are under `gm8emulator/src/`).
OpenGMK is GPLv2: we read it and copy no code. OpenGMK is a model of GM8 built for TAS replays, not GM8
itself, so every such claim is **verify against oracle** until a trace confirms it. Claims about what
Spelunky does come from its GML ([`docs/source-inventory.md`](../source-inventory.md),
[`crates/transpile/examples/analyze.rs`](../../crates/transpile/examples/analyze.rs)).

## 1. The frame

One call to `World::step(input)` is one GM8 frame (room speed is 30 in every room). The order, from
`game.rs` `Game::frame` unless noted:

| # | Stage | What happens | Citation | Spelunky uses it |
|---:|---|---|---|---|
| 0 | Input | The frame's input (keys down, pressed, released; mouse) is fixed before the frame. Pressed/released flags are cleared at the very end of the frame. | `game.rs` `Game::frame` (last line, `input.step()`); `game/replay.rs` sets input per frame | yes |
| 1 | Previous positions | `xprevious`/`yprevious` = `x`/`y` for every active instance, in instance order. | `Game::frame`, first loop | indirectly (solid move-back) |
| 2 | Begin step | Begin-step triggers, then the begin-step event (step sub 1). | `Game::frame`; `game/events.rs` `run_object_event` | `oScreen` (draws the frame by hand) |
| 3 | Timelines | Advance timelines. | `Game::frame` | no timelines |
| 4 | Alarms | For alarm 0 to 11 in order, for each holder object in id order, for each instance: if the alarm is ≥ 0, decrement it; run the alarm event when it reaches 0. | `game/events.rs` `run_alarms` | 159 alarm events, alarms 0–11 |
| 5 | Keyboard | Key-held events by key code, then "any key"/"no key" (sub 1/0); then key-press events and their any/none; then key-release events and theirs. | `game/events.rs` `run_keyboard_events`, `run_key_press_events`, `run_key_release_events` | 22 key-press events, nothing else |
| 6 | Mouse | Button held/pressed/released per instance under the cursor, enter/leave, then global button events, then wheel. | `game/events.rs` `run_mouse_events` | menus and level editor only |
| 7 | Step | Step triggers, then the step event (sub 0). | `Game::frame` | 196 step events |
| 8 | Motion | For every instance: friction, then gravity (`process_speeds`); then for every instance: `x += hspeed`, `y += vspeed`, and path advance (`apply_speeds`). | `game/movement.rs` `process_speeds`, `apply_speeds` | `speed`/`direction` on three "sight" objects |
| 9 | Outside / boundary | Outside-room (other 0) and intersect-boundary (other 1) events, then outside-view and intersect-view (other 40–47, 50–57). Bounding box against the room; a point test if the instance has no mask. | `game/events.rs` `run_bound_events` | 11 outside events |
| 10 | Collisions | See [Collisions](#7-collisions). | `game/events.rs` `run_collisions` | 123 collision events |
| 11 | End step | End-step triggers, then the end-step event (step sub 2). | `Game::frame` | `oPlayer1`, `oBomb`, `oScreen` |
| 12 | Cleanup | Instances destroyed this frame are removed from the instance list. | `Game::frame` (`remove_with` on `Deleted`) | |
| 13 | Draw | Views follow their targets, the instance list is **sorted by depth**, then draw events run per visible view (default drawing for objects without a draw event). | `game/draw.rs` `Game::draw` | 36 draw events; they change state |
| 14 | Animation | For every instance: `image_index += image_speed`; if it reaches the sprite's frame count, subtract the count and run animation end (other 7). | `Game::frame`, after `draw` | 147 `image_speed` writes, 51 animation-end events |

After any stage, if a room change is pending (`room_goto`, `room_restart`, `game_end`) the frame stops there.
While a change is pending, no further events run at all: `run_instance_event` returns immediately
(`game/events.rs` `run_instance_event`). So the rest of the current stage is skipped, and motion,
collisions, end step, draw and animation are skipped for the frame. The room switch happens between frames
(see [Rooms](#10-rooms-persistence-and-the-game-start)).

Two consequences that matter for a headless port:

- **Draw is part of the simulation.** Draw events run GML that writes state (`oGame`'s HUD code,
  `characterDrawEvent`, `oScreen`'s surface juggling), view following moves `view_xview`/`view_yview`, which
  `oLevel` uses to cull, and `draw_sort` reorders instance iteration. Headless mode runs stage 13 in full with
  a no-op draw sink. Only pixel output is skipped. Spelunky's `oScreen` also calls `screen_redraw()` from its
  begin-step event (when pausing), which runs every draw event an extra time; that has to be the same code
  path.
- **Animation end comes after draw.** An animation-end handler that destroys its instance takes effect next
  frame. That instance has already been drawn on its last frame.

## 2. Instances and ids

```rust
pub struct World {
    program: &'static Program,      // generated code + object metadata (see §11); not cloned
    assets: Arc<Assets>,            // sprites, masks, rooms; immutable, shared between clones
    insts: Instances,               // all instances of the current room (§2, §5)
    globals: Globals,               // global.* and globalvar variables (§3)
    room: RoomState,                // id, size, views, backgrounds, tiles, speed
    rng: Rng,                       // GM8's generator state (the `gm` crate's)
    input: InputState,              // this frame's keys/mouse, plus pressed/released
    next_instance_id: i32,          // GM8's last_instance_id, incremented by instance_create
    pending: Option<SceneChange>,   // room_goto / room_restart / game_end
    stored_rooms: Vec<RoomState>,   // persistent rooms (unused by Spelunky, kept for fidelity)
    host: HostState,                // virtual files, highscore table, sound-command queue
    cursor: Cursor,                 // last (unit, line) executed, for error and divergence reports
}
```

- **Handles vs ids.** An `Inst` is a `u32` slot in a slab (`Vec<Instance>` with a free list). GML never sees
  handles. It sees **instance ids**: room-placed instances keep the id stored in the room (≥ 100001), and
  `instance_create` takes `++next_instance_id` (`gml/kernel.rs` `instance_create`). Ids are never reused.
  The world maps id → handle with a `BTreeMap<i32, Inst>`: deterministic, and cheap to clone at a few
  thousand entries. No `HashMap` with `RandomState` appears anywhere in the world.
- **State per instance**: `Active`, `Inactive` (deactivated), or `Deleted` (destroyed this frame, removed at
  stage 12). This matches `instance.rs` `InstanceState`.
- **Looking up an id** finds the instance only if it is active. A deactivated or deleted instance behaves as
  if it doesn't exist (`instancelist.rs` `get_by_instid`, `gml/runtime.rs` `get_target`). Spelunky holds ids
  in variables everywhere (`trapID`, `holdItem`), and `oLevel` deactivates off-screen instances every step,
  so this rule is hit constantly.

## 3. Variables

GML has three kinds of storage, and the transpiler knows statically which one each access uses (see
[translation.md, "Name resolution"](translation.md#how-the-transpiler-works)):

**Built-in instance variables** (`x`, `y`, `sprite_index`, `image_speed`, `alarm[0..11]`, `depth`,
`visible`, `speed`, `direction`, ...) are **typed fields of `Instance`**, as in the GM8 runner
(`instance.rs` `Instance`). Some writes have side effects, handled in the setter: `speed`/`direction` and
`hspeed`/`vspeed` update each other, writing `x`/`y`/`sprite_index`/`image_*` marks the bounding box stale,
and `depth` changes draw order at the next sort. Read-only built-ins (`id`, `object_index`, `sprite_width`,
`bbox_*`) are computed. A built-in is never "unset".

**User instance variables** (675 distinct names) are dynamic: any code can write any name on any instance,
and scripts write into whichever instance runs them (256 names are assigned inside scripts; `rand` writes
`randNum` on every caller). The design:

- The transpiler interns every variable name into a `Var(u16)`. Built-ins take the low ids.
- For each object it computes a **layout**: the variables an instance of that object can ever hold. That is
  everything assigned in the object's and its ancestors' events, plus the scripts reachable from them, plus
  `with`/`other.`/`obj.` writes whose target object is known. The layout maps `Var` → slot, as a dense
  `[u16; NUM_VARS]` table per object in the static `Program` (about 1.4 KB per object, shared, not cloned).
  Measured: from event code alone, objects have 4 variables at the median, 13.8 on average, 112 at most
  (`oPlayer1`).
- An instance stores `vars: Box<[Slot]>` sized to its object's layout, plus `extra: Vec<(Var, Slot)>` for
  any variable the static analysis missed (writes through dynamic instance ids). So the layout is an
  optimisation that can't change behaviour.
- A `Slot` is unset, a `Value`, or an array (`BTreeMap<u32, Value>` keyed `i * 32000 + j`; each index is
  rounded and must be in `0..32000`, per `gml/runtime.rs` `get_array_index`; verify the combined key against
  oracle). GM8 treats `a` and `a[0]` as the same storage (`instance.rs` `Field`).
- Reading an unset variable is a GML error: the game is built with "treat uninitialized variables as 0" off
  ([`docs/source-inventory.md`, Game settings](../source-inventory.md#1-counts)). The port stops the tick and reports `unit:line`.

**Globals** (`global.name`, 318 names, and the 3 `globalvar` names) are all known statically, since the game
uses no `variable_global_*` or `execute_string`. They live in a dense `Vec<Slot>` indexed by `GVar(u16)`, with
names in the `Program` for traces. **Built-in globals** (`room`, `view_xview[]`, `keyboard_key`, `mouse_x`,
`room_width`, ...) are typed fields of `World`/`RoomState`.

**Locals** (`var`) are Rust locals in the generated function (`Local`), scoped to one code action or script
call, as in GM8.

**`Value`** is `gm::Value` (851-3125): `Real(f64)` or `Str(GmString)`, where `GmString` is shared
`Arc<[u8]>` bytes (GM8 strings are Windows-1252 bytes). That meets this design's needs: it is `Send`, and
cloning a world copies no text, which matters because every instance holds a string (`type = "Snake"`).
String literals in generated code should be built once (statics), not per evaluation.

## 4. Objects, parents and event inheritance

Object metadata (parent, sprite, mask, solid, visible, persistent, depth) comes from `gmk` and is baked into
the static `Program` with:

- `parents[obj]`: the object and all its ancestors. `children[obj]`: the object and all its descendants.
  Both include the object itself (`game.rs`, "Populate identity lists").
- **Handlers**: `handler[obj][event] = Option<(EventFn, defining_obj)>`, resolved at generation time by walking
  up the parents to the first object that defines the event. This is what `run_instance_event` does at run
  time (`game/events.rs`).
- **Holders**: for each event, the sorted list of objects whose instances receive it: the children of every
  object that defines it, ascending by object id, without duplicates (`game.rs` `fill_event_holders`).
  Stages 2, 4, 5, 7 and 11 run `for obj in holders[event] { for inst in instances_of_exactly(obj) {...} }`
  (`game/events.rs` `run_object_event`). So **within a stage, instances run grouped by object id ascending**,
  not by instance id or depth.
- **`event_inherited()`** (Spelunky only uses the DnD form, 284 times) runs the handler resolved from the
  parent of the *defining* object, for the same event, `self` and `other` (`gml/kernel.rs`
  `event_inherited`). The generated code calls `g.event_inherited(cx)`; `cx` carries the event and the
  defining object.
- Collision handlers resolve on both sides: for an instance of A meeting B, look for A's handler for B, then
  A's ancestors' handlers for B, then the same for B's parent, and so on (`run_instance_event`'s collision
  branch). The generator precomputes this as `collision_handler[a][b]`.

## 5. Instance order

Iteration order decides who runs first, which instance `instance_nearest`/`instance_place`/`collision_*`
return on ties, and which instance `obj.x` reads. The port copies the runner's data structure exactly instead
of reasoning about it (`instancelist.rs`):

- `draw_order: Vec<Inst>`: GM8's array of active instances, destroyed ones included until the cleanup.
  `instance_create`, room loading and reactivation **append**. At each draw (stage 13, and `screen_redraw`) it
  is **stably** sorted by depth, descending (`InstanceList::draw_sort`). Stage 12 removes deleted entries without
  reordering. Deactivated instances are in a second array, `inactive_order` (§8).
- `by_object: Vec<Vec<Inst>>` (exact object) and `by_ancestor: Vec<Vec<Inst>>` (object or any descendant):
  `instance_create` appends to the instance's object and to each of its ancestors. After any removal and any
  (de)activation that moves an instance, both are **rebuilt from `draw_order`**, destroyed instances included
  (`InstanceList::refresh`).
- Iterators don't borrow. They hold `(position, count)`, with `count` taken when the iteration starts, and on
  each `next` they scan the *current* vector from `position` for the next active entry (`instancelist.rs`
  `ObjectIter`, `IdentityIter`, `nb_il_iter`). This reproduces GM8's behaviour when the list changes mid-loop:
  - instances created during the loop are **not** visited (they are past `count`);
  - destroyed instances are skipped (not active);
  - if a (de)activation rebuilds the vector mid-loop, iteration continues at the same index into the new
    vector, which can skip or repeat instances. That is GM8's behaviour too (OpenGMK keeps `count` "to match
    GM8 weird behaviour"). Verify against oracle.
- Which list is used where: events use `by_object` per holder; `with(obj)`, `obj.x`, `instance_number`,
  `instance_nearest`, `collision_*(obj)` and `instance_exists(obj)` use `by_ancestor`; `all` uses `draw_order`
  (`game.rs` `find_instance_with`; `gml/runtime.rs` `Instruction::With`).

A consequence: in a level, `oLevel` (de)activates every step, so `by_object` is rebuilt every step and per-object
order follows the draw order: by depth as of the last draw, with ties in the order the array had them, and the
instances reactivated since at the end (§8). In rooms that don't (de)activate or destroy,
per-object order is creation order. A new instance whose object id is *higher* than the object currently
running its step event will run its own step in the same frame. One with a lower id waits for the next frame.

## 6. `with` and `other`

`with (target) body` (`gml/runtime.rs` `Instruction::With`):

- `other` becomes the current `self`; `self` becomes each target in turn. After the loop both are restored.
- Target values: `self` (-1) runs once on self; `other` (-2) runs once on other; `all` (-3) iterates
  `draw_order`; `noone` and other negatives do nothing; `0..100000` is an object (iterates `by_ancestor`);
  `≥ 100000` is an instance id (runs once if it's active).
- `break` and `continue` act on the `with` loop; `exit` leaves the whole code unit.
- Mid-loop creation and destruction follow the iterator rules in §5. Destroying `self` inside `with` doesn't
  stop the body: the code keeps running on the deleted instance, whose variables stay readable until stage 12.
- `var` locals are shared with the enclosing code.

`other` elsewhere (701 uses): in a collision event it's the partner instance. In events run by the frame
loop (stages 2–14) it's the instance itself (`run_object_event` passes `other = self`). In `instance_create`'s
create event and in `instance_destroy`'s destroy event OpenGMK also passes `self`; Spelunky never reads
`other` in a create event, and reads it twice in destroy events, so verify those two against oracle. Scripts
inherit `self` and `other` from the caller.

## 7. Collisions

`game/events.rs` `run_collisions`, with holders from `game.rs` `fill_event_holders`:

- The holder table pairs objects symmetrically and keeps each unordered pair once, under the lower object id:
  `pairs[a] = [b ≥ a, ...]` expanded to all children on both sides.
- `for a in pairs (ascending) { for i in by_object[a] { for b in pairs[a] { for j in by_object[b] { if
  i and j collide: ... }}}}`. Each meeting runs **both** events: i's event with `other = j`, then j's event
  with `other = i`.
- **Solid rule.** If either instance is solid, both are moved back to `xprevious`/`yprevious` **before** the
  events. After the events, if either is solid, both get `x += hspeed; y += vspeed` again, and if both are still
  active and still colliding, both are moved back again.
- If i is no longer active after a meeting, i is done for this pass.
- The collision test is GM8's: bounding boxes, then every pixel of their intersection against both
  instances' frame masks (`game.rs` `check_collision`). Masks come from `gmk::Sprite::collision_masks`;
  rectangle masks are just full ones. Implemented in `crates/gm/src/collision.rs` (851-3127), with the
  query built-ins; their quirks are in [quirks.md §5](../quirks.md#5-runner-frame-instances-and-events).
- Queries and the stage go through a spatial index (`crates/gm/src/index.rs`): bounding boxes cached in a
  16-pixel grid, kept up to date through the accessor every write goes through. A query tests the
  instances near it and keeps the one a linear scan would reach first (by its rank in the list scanned);
  the stage jumps straight to the overlapping instances of a long target list until its first meeting,
  then walks the list. Property tests compare both with plain scans on random worlds.

Spelunky: 24 objects are solid (mostly treasure, including `oItem` via inheritance), and 12 collision events
involve one. Its own movement doesn't use `hspeed`/`vspeed`, so the re-apply step is a no-op except on the
sight objects.

## 8. Deactivation

`oLevel`'s step runs every frame in a level: `instance_deactivate_region(view ± 96, false, true)`, then
`instance_activate_region(...)`, then `instance_activate_object` for a whitelist (characters, ropes,
controllers, explosions, ghosts, boulders, Olmec) and a few regions around specific instances. The pause screen
does `instance_deactivate_all(true)` and reactivates `oGamepad`.

Semantics (`gml/kernel.rs` `instance_deactivate_*`, `instance_activate_*`):

- Deactivated instances run no events (they're not active), aren't found by `with`, collisions,
  `instance_exists`, `instance_number` or id lookups, and keep their state, alarms included (alarms don't count
  down: stage 4 only visits active instances).
- `*_region` tests the bounding box against `left, top, width, height`. Instances with no mask use a point
  test. Spelunky passes `x2, y2` as width and height in one call ([source-inventory surprise 5](../source-inventory.md#9-notable-surprises)): keep it.
- As in GM8, deactivated instances live in their own array (`InstanceList::inactive_order`), in the order they
  were deactivated. A call only flags instances (destroyed ones too), then `InstanceList::settle` moves the
  ones whose flag changed to the end of the other array, so a reactivated instance comes after the others,
  and, if any moved, rebuilds `by_object`/`by_ancestor` (§5) from the draw order (quirks.md §7,
  "(De)activation"). Cost: O(instances) per call, around fifteen calls per frame. With a few thousand
  instances that's microseconds, but it must not allocate per call: move and rebuild in place.

## 9. Alarms, motion, animation, views

- **Alarms**: `alarm[n]` is an `i32`, -1 when off. Stage 4 decrements non-negative alarms and fires at 0, so
  `alarm[1] = 1` fires on the next frame and `alarm[n] = 0` never fires (it goes to -1). Writing a fractional
  value rounds (verify against oracle).
- **Motion**: §1 stage 8; `speed`/`direction` and `hspeed`/`vspeed` are kept consistent by the setters
  (`instance.rs` `set_direction`, `set_speed`, `set_hvspeed`).
- **Animation**: §1 stage 14. `image_index` wraps by subtracting the frame count, not by modulo. Setting
  `sprite_index` resets `image_index` to 0 only if `floor(image_index)` is past the new sprite's last frame
  (`instance.rs` `set_sprite_index`).
- **Views**: at the start of stage 13, each visible view with a follow target recentres per GM8's border
  rule, then clamps to the room (`game/draw.rs` `Game::draw`; `room.rs` `View::follow`). Then the window is
  cleared to the settings' colour outside the room and each visible view is drawn; with views off the
  room is drawn once. After the draw, background layers scroll by their speeds. `oScreen` rewrites view 0 of
  26 rooms at startup with `room_set_view`, which applies from each room's next load.

## 10. Rooms, persistence and the game start

A pending room change is handled after the frame (`game.rs` main loop, then `load_room`):

1. The cleanup: destroyed instances leave `draw_order` (the frame skips its own once a change is pending).
   Then room-end events (other 5) for every instance in `draw_order` (`run_other_event(5)`); room changes
   requested here are ignored.
2. The persistent instances that are active (not deactivated, not destroyed in a room-end event) are taken
   out, in `draw_order`'s order: by depth at the last draw, then those created or reactivated since. The
   rest go with the room. That's GM8's (Spelunky.exe 0x53e270, probe `persistent-carry`); OpenGMK took every
   persistent instance, in its slot order (`remove_as_vec`), until oracle patch 0033 (851-3315). The old
   room is stored if it's persistent; no Spelunky room is.
3. The new room's tiles are added, then its placed instances in room order (skipping ids that already exist),
   then the persistent instances are appended after them, in that order. Spelunky's four (`oGlobals`,
   `oScreen`, `oMusic`, `oGamepad`) are created during `rScreenInit`'s load; `oLevel` deactivates `oMusic`
   while it's far from the view, so it's left behind at most level exits. A persistent instance replaces a
   placed one with its id.
4. Create events for the new placed instances, in placement order. All of them already exist, so a create
   event can see instances placed after it. Instance creation code would run here; Spelunky has none.
5. On the first room only: game-start events (other 2).
6. Room creation code (none in Spelunky), then room-start events (other 4; none in Spelunky).
7. If no new room change is pending, **draw once** ("frame 0"). That runs the draw events again, so it
   belongs to the simulation (§1).

`persistent` comes from the object itself (`instance.rs` `Instance::new_ext`). Per the inventory,
`oCharacter`'s and `oEnemy`'s flag doesn't reach their children; check that against the oracle on `oPlayer1`.

The game starts in room order index 0 (`rScreenInit`), whose `oScreen` creates the controllers and calls
`room_goto_next()`. The world constructor runs the equivalent of `load_room(room_order[0])` with the game-start
flag set: `World::start_game`. `World::step` performs a pending change after the frame
(`crates/gm/src/room.rs`). A change requested during a load is left for the next step, whose frame runs no
events (OpenGMK's loop runs `frame()` before looking at the change). The exception is the first room: GM8
performs a change its load requests once before entering the frame loop (OpenGMK `init` then
`handle_scene_change!`), so `rScreenInit` leads straight into `rIntro` and frame 0 is a full `rIntro` frame.

## 11. How `game` plugs in

```
crates/gm         runtime: World, Instances, Value, Rng, collision, built-in functions, frame loop
crates/game       generated code + overrides + Program table; the public Game API
crates/transpile  the transpiler (gmk + gml -> crates/game/src/generated/); scripts/transpile runs it
```

`gm` can't depend on `game`, so the generated code reaches the runtime through a static table:

```rust
// in gm
pub type EventFn = fn(&mut World, &mut Ctx) -> R<()>;
pub type ScriptFn = fn(&mut World, &mut Ctx, &[Value]) -> R<Value>;

pub struct Program {
    pub objects: Vec<ObjectInfo>,                 // name, parent, sprite, mask, solid, visible, persistent, depth
    pub parents: Vec<Vec<ObjectId>>,              // §4
    pub children: Vec<Vec<ObjectId>>,
    pub handlers: Vec<EventTable>,                // per object: resolved (EventFn, defining object) per event
    pub collision: Vec<Vec<Option<(EventFn, ObjectId)>>>,
    pub holders: HolderTable,                     // per event: object ids; collision pairs
    pub layouts: Vec<VarLayout>,                  // §3
    pub var_names: Vec<&'static str>,             // for traces and errors
    pub global_names: Vec<&'static str>,
    pub room_order: Vec<RoomId>,
}

// in game (generated)
pub static PROGRAM: Program = /* ... */;
```

`game` exposes the API the app, the harness and RL use, and nothing GML-specific:

```rust
#[derive(Clone)]
pub struct Game { world: gm::World }

impl Game {
    pub fn new(assets: Arc<Assets>, config: Config) -> Game;   // RNG seed, settings.cfg/keys.cfg contents, options
    pub fn step(&mut self, input: &Input) -> Result<StepInfo, GmlError>;
    pub fn frame(&self, out: &mut dyn DrawSink);              // replays the last frame's draw calls
    pub fn observe(&self) -> Observation;                     // structured state for agents
    pub fn trace(&self, out: &mut trace::Writer);             // per-tick state for oracle diffs
}
```

- **Overrides** ([translation.md](translation.md#how-the-transpiler-works)): `SS_*` scripts push sound commands to a queue in `HostState`; file-reading
  scripts and `file_text_*` read a virtual file system given in `Config`; the highscore table lives in
  `HostState`. Everything the original read from disk or a DLL is an explicit input, so runs are reproducible.
- **Drawing**: draw built-ins (`draw_sprite`, `draw_text`, surfaces, blend modes) record commands into a
  per-frame buffer when a sink is attached and do nothing otherwise. Any built-in whose result feeds back into
  logic (`surface_exists`, `surface_create`) returns what GM8 would return on a working machine.
- **Clone cost.** A level has on the order of 1,000–2,000 instances (terrain is instances). Per instance:
  about 200 bytes of typed built-ins plus about 24 bytes per user-variable slot (median 4, a few hundred for
  characters), one allocation for `vars`. Roughly 1 MB and 2,000 allocations per clone, so well under a
  millisecond. If savestate-heavy search needs more, move all slots into one world-wide arena (`Vec<Slot>`
  plus an offset per instance) so a clone is a few large copies. The layout design allows that without
  touching generated code.

## 12. The API the translation targets

This was the transpiler prototype's stub; the real API is `gm`'s, and generated code reaches it through
`crates/game/src/rt.rs`. In short:

```rust
pub struct Ctx { pub this: Inst, pub other: Inst, pub ret: Value, /* args, event, defining object */ }
impl Ctx {
    fn arg(&self, n: usize) -> R<Value>;          // argumentN
    fn call(&self, args: &[Value]) -> Ctx;        // script call: same self/other
    // `with` sets cx.this = target and cx.other = old this in place, restoring both after the loop.
}
impl World {
    fn get(&self, h: Inst, v: Var) -> R<Value>;                         // built-in or user variable
    fn set(&mut self, h: Inst, v: Var, x: Value) -> R<()>;
    fn get_a / set_a (.., i: u32, j: u32 ..);                           // arrays
    fn get_on(&self, cx: &Ctx, target: &Value, v: Var) -> R<Value>;     // obj.v: first instance
    fn set_on(&mut self, cx: &Ctx, target: &Value, v: Var, x: Value);   // obj.v = x: every instance
    fn gget / gset / gget_a / gset_a (GVar ..);                         // globals
    fn bget / bset (Var, i);                                            // game-wide built-ins
    fn with_iter(&self, cx: &Ctx, target: &Value) -> R<WithIter>;       // non-borrowing iterator (§5)
    fn event_inherited(&mut self, cx: &mut Ctx) -> R<()>;
    fn at(&mut self, unit: &'static str, line: u32);                     // cursor for reports
}
// GM8 value semantics: gm::Value's add, sub, mul, div, int_div, modulo, bit ops, neg, gml_eq..gml_ge (epsilon),
// is_truthy; plus case_eq (exact), and/or/xor over evaluated bools, lenient (op=), ix,
// repeat_count.
// Built-in functions: b::name(g, cx, [Value; N]) -> R<Value> (crates/game/src/builtins.rs), converting to typed
// gm calls.
```

## 13. What this needs from the other crates

- **`gmk`** (merged): everything used here is there: objects (parent, flags, depth, events with action
  lists and typed DnD arguments), scripts, rooms (placed instances with ids, scale, blend, angle, creation code;
  views; backgrounds; tiles), sprites with GM8 collision masks, room order, `last_instance_id`, settings.
- **`gm` values and RNG** (merged, 851-3124/851-3125): `Value` and its GM8 operators (`add`, `gml_lt`, ...,
  returning `Result<_, ValueError>`; `and`/`or`/`xor` on already evaluated operands), `math::CMP_EPSILON`,
  string conversion, and `Rng` as plain state. The transpiler emits these `gm` names. Still needed on top: `case_eq`
  (exact equality for `switch`), the lenient `op=` helper, array-index rounding, and `repeat_count`.
- **`trace`**: a per-tick record. At minimum the frame number, RNG state, room, and per active instance
  `(id, object, x, y, sprite_index, image_index, alarms)`, plus selected user and global variables by name
  (names come from `Program`). On a GML error, the `(unit, line)` cursor.

## 14. Verify against oracle

Each of these is one small OpenGMK test room, or a check on a recorded Spelunky trace:

1. The frame order in §1, especially animation after draw and the stop-after-stage rule for room changes.
2. Per-object iteration order: creation order vs depth order, and the mid-loop rebuild behaviour (§5).
3. A new instance's step running in the same frame depending on object ids (§5).
4. The solid move-back and re-apply rule (§7), and the order of the two events in a meeting.
5. Deactivated instances' alarms not counting down, and region tests with masked vs unmasked instances (§8).
6. `other` in create and destroy events (§6).
7. Order of persistent instances after a room change (§10).
8. Alarm writes with fractional values; the 2-D array index rule (§3, §9).
9. The GML-level rules in [translation.md](translation.md#evidence-gm8-semantics-the-translation-must-reproduce): no short-circuit, operator precedence, `obj.v op= x`, and script
   return values.
