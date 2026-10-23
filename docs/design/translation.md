# How GML becomes Rust (851-3121)

**Decision: a hybrid with a transpiler baseline.** All of the game's GML (139 scripts and 969 code
actions, plus the drag-and-drop actions) is translated mechanically into Rust by a GML-to-Rust
transpiler. Hand-written Rust is limited to the GameMaker runner (`gm`: instances, events, collision, built-in
functions) and a small, explicit table of **script overrides**: scripts that wrap things a headless port
can't or shouldn't run, such as the SuperSound DLL (`SS_*`) and file I/O. No game logic is hand-ported.

The reasons, in order of weight:

1. **Fidelity is mechanical, not a matter of care.** The game depends on at least a dozen GM8 semantic rules
   that differ from Rust's (see [the evidence](#evidence-gm8-semantics-the-translation-must-reproduce)):
   no short-circuit `and`/`or` (2,255 sites, 68 with `rand()` in the right operand), GM8's own operator
   precedence, epsilon comparisons (5,429 sites), scripts that return their last assignment, `obj.v += 1`
   that reads one instance and writes all, code that keeps running after `instance_destroy()` (207 sites), and
   more. A transpiler encodes each rule once. A hand port has to get each one right at every site, by
   reading, across 30k lines, and the bugs it introduces show up only as RNG drift tens of seconds later.
2. **The transpiler is already mostly done.** The prototype (about 1,000 lines) translates **all 1,176 units with no
   unsupported constructs**, and the generated 125k-line crate type-checks against the runtime API sketch.
   A hand port of 30k lines is months of work, with review for each line.
3. **Divergences map straight back to GML.** Each generated statement carries its GML unit and line
   (`g.at("oSnake/step#1", 33)`) and a comment with the GML. A trace diff or a runtime error reports
   `oSnake/step#1:33`, which is the line to read in the original.

The cost is verbose, less idiomatic Rust (about 3.6 generated lines per GML line, including comments and
line markers) and dynamic `Value` arithmetic everywhere. Both are acceptable: the GML is the readable source
and the generated code is a build product, and performance has headroom (see [Performance](#performance)).

## What was built and measured

- **`crates/gml`**: a lexer and parser for GM8 GML producing an AST with byte spans, a canonical printer,
  and a visitor. It parses **every unit of the game: 139 scripts, 969 code actions and 68 drag-and-drop
  expression arguments (1,176 units, 34,960 lines), with no failures**, and every one round-trips
  (`parse ∘ print ∘ parse` gives the same tree). The test (`crates/gml/tests/spelunky.rs`) reads the units
  with the `gmk` crate straight from `reference/spelunky_1_1_src.zip`, so it runs in `scripts/check`. A
  committed corpus of real and synthetic snippets is snapshot-tested too.
- **`crates/gml/examples/analyze.rs`**: counts the GM8 semantics the translation must reproduce. All numbers
  in this document come from it.
- **`crates/gml/examples/transpile/`**: the transpiler prototype. It emits Rust against `proto_rt.rs`, a stub
  of the runtime API sketched in [runtime.md](runtime.md#the-api-the-translation-targets). `--crate` mode
  writes the whole game as a standalone crate, which then passes `cargo check`. A cold release build of it
  takes 43 s.
- **Prototypes on three objects** in [`prototypes/`](prototypes/):

  | Object | Transpiled | Hand-ported |
  |---|---|---|
  | `oSnake` (create, step) | [`oSnake.generated.rs`](prototypes/oSnake.generated.rs) | [`oSnake.hand.rs`](prototypes/oSnake.hand.rs) |
  | `oArrowTrapRight` (all 5 events) | [`oArrowTrapRight.generated.rs`](prototypes/oArrowTrapRight.generated.rs) | [`oArrowTrapRight.hand.rs`](prototypes/oArrowTrapRight.hand.rs) |
  | `oPlayer1` core step (`characterStepEvent`, 1,031 lines) | [excerpt](prototypes/characterStepEvent.generated.rs) (3,612 lines in full) | discussed below |

  The hand ports mark every place where an idiomatic version differs from GM8 with `FIDELITY:`. Even these
  two small objects have 14 of them.

## Evidence: GM8 semantics the translation must reproduce

Counts are over the whole game. "Source" is where the rule comes from; runner rules come from OpenGMK
(`gm8emulator/src/...`), and each needs confirming against the oracle.

| Rule | Count in Spelunky | Example | Source |
|---|---:|---|---|
| `and`/`or`/`xor` evaluate **both** operands (no short-circuit) | 2,255 operators; 68 call `rand`/`random` on the right, 278 have a side-effecting call (RNG, script, `instance_create`/`destroy`) | `oManTrap` step: `status == SLEEPY and image_index == 6 and rand(1,8) == 1` draws from the RNG every step. `scrEntityGen`: `global.darkLevel and rand(1,60) == 1` draws during level generation even on lit levels. With Rust `&&`, the RNG stream, and so every later level, diverges. | `gml/runtime.rs` `Node::Binary`, `gml/value.rs` `bool_and` |
| `and`, `or`, `xor` share **one** precedence level (left-associative); bitwise operators bind tighter than comparisons | 3 sites where it changes the meaning | `oItem` step: `abs(xVel) > 2 or abs(yVel) > 2 and collision_rectangle(...) and enemyID == 0` means `((a or b) and c) and d`, not C's `a or (b and c and d)`: it decides when thrown items hit enemies. | `gml-parser/src/ast.rs` `get_op_precedence` |
| Comparisons use an epsilon of 1e-13 | 5,429 | `x > view_xview[0]-20` | `gml/value.rs` `gml_eq`/`gml_lt`, `math.rs` `CMP_EPSILON` |
| `switch` matches with exact equality and falls through | 107 switches | `case vk_up: { return "UP ARR" break; }` | `gml/runtime.rs` `Instruction::Switch` (`almost_equals`) |
| A script with no `return` on some path returns the last assigned or called value | 3 scripts used as values (`scrGetKey`, `scrGetJoy`, `scrGetChar`, 47 call sites) | `scrGetKey` of an unmapped key | `gml/runtime.rs` `SetField`/`SetVariable`/`EvalExpression` set `return_value` |
| `obj.v op= x` reads `v` from the first instance of `obj` and writes the result to all of them; the base is evaluated twice | 15 sites (and 237 plain writes through an object name write *every* instance) | `oMoonRoom.baskets += 1`, `oPlayer1.x += ...` in `oOlmec` | `gml/compiler.rs` `make_modify_instruction`; `runtime.rs` `Target::Objects` |
| `op=` on user variables ignores type errors (keeps the old value); on built-ins it errors | every `+=` (several thousand) | | `gml/compiler.rs` (`type_unsafe: true` for fields only) |
| Evaluation order: assignment target, then array index, then value; arguments and operands left to right | everywhere; matters with RNG calls in arguments | `oArrowTrapRight` destroy: `instance_create(x+8+rand(0,8)-rand(0,8), y+8+rand(0,8)-rand(0,8), ...)` | `gml/runtime.rs` `SetField`, `Node::Field` |
| `instance_destroy()` runs the destroy event, then the rest of the calling code keeps running | 207 calls with code after them | `oSnake` step destroys itself, then still runs its AI and draws `rand(1,100)` | `gml/kernel.rs` `instance_destroy` |
| `break`/`continue` inside `with` act on the `with` loop | 371 `with` blocks | `moveTo`: `with solidId { if place_meeting(...) break ... }` leaves the `with`, not the enclosing `for` | `gml/runtime.rs` `Instruction::With` |
| `var` locals are unit-wide; bare names are instance variables; `globalvar` names are global | 2 `var`, 3 `globalvar`; 675 instance and 318 global variable names | `rand` writes `randNum` on whichever instance calls it | `gml/compiler.rs` |
| A real is true when it is at least 0.5; strings are false | every condition | `if (shp)` on an instance id | `gml/value.rs` `is_truthy` |
| `other` is the collision partner, or the outer `self` inside `with` | 701 uses; `with` targets: 201 object names, 40 `other`, 130 instance-id expressions | `oExplosion` collision: `with other { ... instance_destroy() }` | `game/events.rs`, `gml/runtime.rs` |
| Built-in variables are typed and some writes have side effects | `sprite_index` (1,168 refs), `alarm` (223), `depth` (46), `speed`/`direction` | | `instance.rs` |

The rules in the first half are properties of *expressions and statements*, so they appear thousands of
times across the source. That is the pattern a compiler handles well and a human handles badly.

## The three options

**(a) Transpiler.** A GML-to-Rust compiler, built on `crates/gml`, emits one Rust function per code action and
script against the `gm` runtime API. Values are GM8 dynamic values; variables go through `g.get`/`g.set`
with interned ids; `and`/`or` are plain functions, so both operands are always evaluated; `switch` becomes an
entry index with fall-through sections; `with` becomes a non-borrowing iterator loop.

**(b) Hand port.** People (or LLMs) rewrite each object as idiomatic Rust: typed structs per object, enums for
states, `&&`, `match`.

**(c) Hybrid.** Transpile everything; hand-write only the runner and a declared set of script overrides.
Optionally, later, swap individual hot or OS-facing scripts for hand-written versions, each proven equal by
trace diffs against its generated twin.

| | (a) Transpiler | (b) Hand port | (c) Hybrid (chosen) |
|---|---|---|---|
| **Fidelity risk** | Low and *systematic*: a semantic rule is right or wrong everywhere at once, and one oracle trace exposes it. | High and *scattered*: each of the rules above has to be applied correctly at every site. The prototypes show the traps: an idiomatic `return` after `instance_destroy`, `&&` around `rand`, treating `LEFT = 0` instance variables as constants, `precise = -1` read as true. | As (a). Overrides are few, declared in one table, and each is checked by diffing against the generated version. |
| **Effort for ~30k lines** | Transpiler: ~3k lines (the prototype is about 1,000 lines and covers 100% of the syntax). The runner and the 112 built-ins are needed by every option. | 20–30k lines of new Rust, plus review of each against the GML. Months. | As (a), plus the override shims (SuperSound, files, highscores), which every option needs in some form. |
| **Performance** | Dynamic `Value` ops and variable lookups by id. Compiled, it should far outrun GM8's own interpreter. Typed fast paths can come later (see below). | Best possible: typed fields, no tagging. | As (a), with room to hand-optimise hot scripts behind trace-diff checks. |
| **Readability** | Mechanical and verbose (3.6 lines per GML line). Each statement is annotated with its GML, and names match the GML (`v::xVel`, `scr::moveTo`, `obj::oSolid`). You read the GML for intent. | Best, for a reader who trusts it. But it is a second source of truth that drifts from the GML. | As (a). |
| **Debuggability on divergence** | Exact: `g.at(unit, line)` points at the GML line that ran last, and the structure is 1:1, so you can step through the generated code next to the GML. A GML line-coverage report is free. | You have to map Rust back to GML by hand; refactors break the mapping. | As (a). |
| **Iteration** | A fix to one semantic rule is one change in the transpiler, then regenerate. | A fix to one rule is an audit of every site. | As (a). |

Two other hybrids were considered and rejected:

- **Hand-port the core (player physics, `characterStepEvent`), transpile the rest.** The core is where
  fidelity matters most and where the dense logic (dozens of `and` chains, script calls inside conditions,
  instance variables shared with scripts) is most error-prone. It is the worst place for a hand port.
- **An embedded GML interpreter instead of a compiler.** Simple to get right (like OpenGMK), but slower, and it
  makes the world hold ASTs or bytecode and an interpreter stack. Compiled functions keep the world as plain
  data and are 1:1 debuggable. The parser built here would also serve an interpreter, so this stays a fallback.

## How the transpiler works (prototype to production)

The prototype already does the following. Production needs the items marked *TODO*.

- **Name resolution** (`examples/common/mod.rs`): a bare name is a `var` local, `argumentN`, a built-in
  instance or global variable, a built-in constant, a resource (object, sprite, room, background, script), a
  `globalvar` global, or otherwise an instance variable of `self`. Each GML name keeps its spelling in Rust
  (Rust keywords become raw identifiers, e.g. `v::r#type`).
- **Variables**: instance variables go through `g.get(inst, v::NAME)` / `g.set(...)` with ids interned at
  generation time; globals through `g.gget(gv::NAME)`; locals are Rust `Local` variables. `other`/`self`/
  `global` bases resolve statically; any other base (`obj.x`, `arrow.xVel`) is resolved at run time by
  `g.get_on(cx, &target, v)` (first instance) and `g.set_on(...)` (all instances).
- **Expressions**: `add`/`sub`/... return `R<Value>`; comparisons return `R<bool>`; `and(a, b)` takes two
  evaluated `bool`s, which gives GM8's evaluation by construction. Call arguments are bound first
  (`{ let a = [...]; scr::moveTo(g, cx, &a)? }`), which keeps left-to-right order and lets the call borrow `g`
  mutably. Subexpressions that call functions are hoisted out of `g.method(...)` arguments for the same
  reason.
- **Statements**: native `if`/`while`. `for` and `do-until` use a labeled body block so `continue` still runs
  the step or the condition. `repeat` evaluates its count once. `with` is a `while let` over a non-borrowing
  iterator with `cx` shadowed (`other` becomes the old `self`). `switch` computes an entry index by exact
  comparison in case order, then runs the sections with `if entry <= k`. Stray `break`/`continue`/`exit` end
  the unit. Scripts that can end without `return` keep `cx.ret` updated after each assignment and call.
- **Events**: each event function calls its actions in order. DnD actions are translated too: `action_inherited`
  becomes `g.event_inherited(cx)`, set-variable becomes an assignment, `action_if` guards the next action,
  `action_execute_script` a script call, `action_kill_object` `instance_destroy`.
- **Spans to lines**: `g.at(unit, line)` before each statement, and a `// line: GML` comment. In release
  builds `at` compiles to nothing unless the `trace-lines` feature is on.
- *TODO*: emit the object/event dispatch tables and variable layouts ([runtime.md](runtime.md)); the override
  table; typed signatures for built-ins (the prototype passes `&[Value]`); the compound-assignment and array
  details of built-in arrays (`alarm[n]`, `view_xview[n]`); a check that regenerating gives no diff.

**Where the output lives.** The translator is a tool (`crates/tools`, or its own crate) that reads the `.gmk`
from `reference/` and writes `crates/game/src/gen/`. **Commit the generated code**: it keeps builds free of
the generator, makes changes reviewable as diffs, and lets `grep` and debuggers see real files. `scripts/check`
regenerates it and fails on any diff.

**Script overrides** (the hybrid part). A table maps script names to hand-written Rust:

- `SS_*` (23 SuperSound wrappers around `external_call`): record sound commands in an output queue and keep a
  deterministic model of handle state. Whether that model needs sound lengths to answer
  `SS_IsSoundPlaying` the way the oracle does is an open question.
- File-backed scripts (`scrReadStats`, `scrWriteStats`, config reading in `scrInit`): read from an in-memory
  virtual file system given at startup.
- Any later performance override must keep the original's observable side effects (for example `rand` writes
  `randNum` on the caller) and must match its generated twin in trace diffs.

## Performance

The generated code does a tagged `Value` operation per GML operator and an id-indexed lookup per variable
access. This is the same work GM8's interpreter does per node, minus the interpretation overhead, so compiled
code should run well over 10x faster than the original runner. That puts a full level (a few hundred active
instances after `oLevel` culling) comfortably in the thousands of ticks per second. **This is an estimate**;
it can't be measured until `gm` exists. The first benchmark is a recorded level replayed headless.

If it falls short, the levers in order are: collision broad-phase in `gm` (546 `collision_point` calls, most
over `oSolid`); typed fast paths in the transpiler (built-ins like `x` and `y` are typed reals, and a
whole-program pass can prove that most user variables only ever hold reals); per-object typed variable
slots; and hand-written overrides of hot pure scripts (`isCollision*`), each checked by trace diff.

## Risks and open questions

- **Runner rules come from OpenGMK, not from GM8 itself.** In particular: no short-circuit evaluation, the
  operator precedence table, `obj.v op= x` semantics, and script return values. Each is one switch in the
  transpiler, so a wrong assumption is cheap to fix, but each should get an oracle test early. A tiny test
  game (one object per rule) run in OpenGMK and in the port settles most of them at once. Verify against
  oracle.
- **Literal parsing**: Rust parses decimals with correct rounding; GM8 (Delphi) may round long decimals
  differently. Spelunky's literals are short. Verify against oracle.
- **`globalvar` is a run-time declaration in GM8.** The transpiler treats `globalvar` names as global
  everywhere; GM8 treats them as instance variables until the declaration runs. The three declarations run at
  startup (`scrInit`, `oGame`, `oGlobals` create), so this only matters if one of those names is touched
  earlier.
- **Errors.** GM8 shows a dialog and can continue; the port stops the tick and reports `unit:line`. Code that
  only works because a player clicked "Ignore" can't be replicated, and probably doesn't need to be.
- **Compile time.** 125k generated lines take 43 s for a cold release build. Splitting into one module per
  object (already done) and keeping `gm`'s hot helpers `#[inline]` keeps this manageable.
