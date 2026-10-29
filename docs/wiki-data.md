# Wiki data

The spelunky.ai wiki's numbers (a monster's health and damage, an item's price, a gem's value) and
relations (where a monster appears, what a shop sells, what a crate holds, what hurts what) come from the
game's own code, not from memory or another wiki, and each carries the file and line it came from, so a
page can cite the code and a reviewer can check it. Unlike the [media](wiki-media.md), the output isn't
committed: it's built from the extracted assets.

```sh
scripts/extract     # once: assets/
scripts/wiki-data   # target/wiki-data/stats.json and relations.json, and the site's snapshot
scripts/wiki-data --find 'hp = ?' --in oCaveman:create   # a pattern's matches, while writing a spec
```

`scripts/wiki-data` runs `tools`' `wiki-data` binary on the extracted bundle with the specs in `wiki/data/`.
It reads the original GML (in `bundle.bin`, the same code as the `gml/` tree beside it), not the transpiled
`crates/game`: that is generated from the GML, and a citation is only checkable against the original. The
same bundle and specs always give the same bytes (`crates/tools/tests/wiki_data.rs`, which also checks
values read off the GML by hand and that every citation's line holds the code it quotes).

## Specs

`wiki/data/*.toml` say what to extract: one `[[entity]]` per wiki page, with its stats.

```toml
[[entity]]
slug = "caveman"          # the page's slug
name = "Caveman"
kind = "monster"          # monster, character, weapon, item, equipment, treasure, trap, terrain or mechanic
object = "oCaveman"       # where its stats are looked for, unless a stat says otherwise
templates = []            # named lists of stats ([[template.<name>]]) that come first

[[entity.stat]]
id = "charge-speed"
label = "Charging speed"
unit = "px/step"          # optional, as are note, object, script, event, where, nth, matches and table
event = "step"            # the event, named as the gml/ tree names its file; create if not given
find = "xVel = -?"        # the pattern
where = ["status == ATTACK"]
matches = 1
```

A **pattern** is a GML statement or expression with `?` standing for any expression: `hp = ?`,
`global.plife -= ?`, `dist < ?`, `rand(1, ?) == 1`, `repeat (?) {}` (a compound statement's head, its body
`{}`). It matches code with the same syntax tree, whatever its spacing, parentheses or comments. A
statement pattern (an assignment) matches statements; an expression or a call matches anywhere. `-?`
matches a negated value only, which picks one side of `if (facing == LEFT) xVel = -3; else xVel = 3;`.

It's looked for in `object`'s `event` (the entity's object by default), or in `script`. The event follows
GM's inheritance: an object without it runs its parent's (the Snake's touch is oEnemy's), and
`action_inherited` runs the parent's where it stands. In a create event, a plain `name = ?` that a later
unconditional assignment replaces is dropped, since the game never sees it (oEnemy's `hp = 1` under the
Caveman's `hp = 3`).

Every match counts unless the stat narrows them: `where` keeps those under an `if` or `case` whose GML
contains each of its texts (an `else` doesn't count, so `"status == HANG"` doesn't keep the branch where
the bat isn't hanging), then `matches` says how many there must be and `nth` keeps one (0-based). A
pattern that ends up matching nothing, or not `matches` times, is an error, so a spec can't quietly go
stale. Write the condition texts as `--find` prints them: canonical GML (`&&`, `!`, `x + 8`).

A **table** evaluates a stat's one match over a range of a variable: `var`, `from` and `to` (inclusive),
optional `labels` for the rows, and `bind`, the other variables, each a number or an earlier stat's id
(its `value`). An assignment's row is the variable's value after it runs: a shop's price on each level is
`cost += (cost/100)*10*(global.currLevel-2)` with `cost` bound to the item's price, and where the
condition around it (`global.currLevel > 2`) is false, the price unchanged. Conditions the table can't
evaluate (a call, an unbound name) are taken to hold and listed in `assumed`.

## Output

`stats.json`:

| Field | Contents |
| --- | --- |
| `format` | 1, bumped when the shape changes |
| `notice`, `license` | The copyright notice and the license (docs/wiki-license.md): the `source` fields quote the game's code |
| `room_speed` | Steps a second (30): a speed in px/step times this is px a second, a timer in steps over it is seconds |
| `entities` | One per spec entity, in spec order (files by name): `slug`, `name`, `kind`, `object`, `stats` |
| `objects` | Every descendant of oEnemy, oItem and oTreasure, and every entity's object, by name: `parent`, `type` (the game's `type` string), and `init` |

A **stat** has the spec's `id`, `label`, `unit` and `note`; `find`, `in` (`object:event` or a script),
`where` and `nth`, saying where it looked; `value` when every fact gives the same constant, otherwise `min`
and `max` when they're all numbers or ranges; `facts`; and `table` (`var`, `rows` of `at`, `label` and
`value`, and `assumed`).

A **fact** is one match:

| Field | Contents |
| --- | --- |
| `expr` | The value's expression, as canonical GML: what the first `?` matched (or the whole match) |
| `holes` | Every `?`'s match, when the pattern has more than one |
| `value` | The expression's value, when it's a constant (arithmetic on numbers is folded; `true` is 1) |
| `range` | When it's random: `min`, `max` and `integer`. `rand(a, b)` is the game's script, every whole number from `a` to `b`; `random(n)` is a real from 0 up to but not including `n` |
| `when` | What must hold for the code to run, outermost first: `{"if": cond}`, `{"not": cond}` (an `else`), `{"loop": head}`, `{"with": target}`, `{"case": "value == c"}`, `{"not_exited": head}` (after a statement that may `exit`), `{"question": action}` (after a drag-and-drop question) |
| `at` | `file` (in the `gml/` tree, relative to the assets) and 1-based `line`; `object` and `event`, or `script`; and `source`, the matched code verbatim (a compound statement's first line) |

The conditions are honest rather than tidy: a snake's every step is under the on-screen check its step
event starts with, and the Snake's contact damage sits in oEnemy's `else` branches. A page picks what to say
from them; the data doesn't flatten them away.

An object's **`init`** is what its create event (with its parents') sets at the top level, by variable:
`expr`, `value`, `range`, `file` and `line`, and `changed_later` when the event may set it again under a
condition. Assignments inside an `if`, or in code that runs on another instance, aren't in it.

Numbers are the ones the code writes: GM8's runner reads a few decimals 1 ulp off (`0.6` is
0.6000000000000001, `gml::lexer::decimal_value`), which the port reproduces and a page shouldn't show.

A monster and an item, cut down:

```json
{
  "slug": "snake",
  "name": "Snake",
  "kind": "monster",
  "object": "oSnake",
  "stats": [
    {
      "id": "damage",
      "label": "Contact damage",
      "unit": "hearts",
      "note": "Hearts lost on touching it, unless the player is invincible (for 30 steps after a hit) or stomping it.",
      "find": "global.plife -= ?",
      "in": "oSnake:collision_oCharacter",
      "value": 1,
      "facts": [
        {
          "expr": "1",
          "value": 1,
          "when": [
            { "not": "abs(other.x - (x + 8)) > 12" },
            { "not": "!other.dead && (other.state == 15 || other.state == 16) && other.y < y + 8 && !other.swimming" },
            { "if": "other.invincible == 0" },
            { "if": "global.plife > 0" }
          ],
          "at": {
            "file": "gml/objects/oEnemy/collision_oCharacter.gml",
            "line": 25,
            "object": "oEnemy",
            "event": "collision_oCharacter",
            "source": "global.plife -= 1"
          }
        }
      ]
    }
  ]
}
```

```json
{
  "id": "price-by-level",
  "label": "Price by level",
  "unit": "$",
  "find": "cost += ?",
  "in": "scrShopItemsGen",
  "facts": [
    {
      "expr": "cost / 100 * 10 * (global.currLevel - 2)",
      "when": [{ "if": "obj" }, { "if": "global.currLevel > 2" }, { "with": "obj" }],
      "at": {
        "file": "gml/scripts/scrShopItemsGen.gml",
        "line": 186,
        "script": "scrShopItemsGen",
        "source": "cost += (cost/100)*10*(global.currLevel-2)"
      }
    }
  ],
  "table": {
    "var": "global.currLevel",
    "rows": [{ "at": 1, "value": 15000 }, { "at": 2, "value": 15000 }, { "at": 3, "value": 16500 }, "..."],
    "assumed": [{ "if": "obj" }, { "with": "obj" }]
  }
}
```

## The site's snapshot

The site's build (Workers Builds) has neither Rust nor the extracted assets, so it can't run `wiki-data`. It
reads trimmed copies of its output instead, committed under `site/content/wiki-data/`:

- `stats.json`: each entity's stats (`label`, `unit`, `value` or `min` and `max`, a `table`'s rows, and
  `at`, the `file:line`s they came from, in the `gml/` tree as pages cite them) and each object's
  create-event constants and ranges, one stat or variable a line so a diff reads well. The wiki's pages read
  their `<Stat>`s from it.
- `relations.json`: what the wiki checks a page's relations against (docs/wiki-style.md, "Relations"), by
  object, one a line, without the code: `places` (the areas and special levels, by slug: `name` and `kind`)
  and `shops` (by slug, their names); `found_in` (per object, the `places` its spawns' `areas` and
  `only_in` name, and the `rooms` it's `placed_in`); `sold_in`; and `drops` and `damaged_by`, per object
  the objects it makes and those that hurt it, with inheritance followed: `own` is what the data says of
  the object by name (a drop's key, a damage statement's victim, and the sources that statement names),
  and `inherited` what it gets as a descendant, from code it runs (the objects in `runs_as`, so not one
  that overrides the event) or a collision with its parent, and what the named sources' descendants do
  (a Snake's test for being inside `oSolid` is one for being inside Olmec, a solid, which never meets a
  Snake outside his lair). A source or a key that's the code's own object is likewise the objects that run
  the code, and any other is it and its descendants. Its `format` is its own, bumped with its shape.

`scripts/wiki-data` writes both along with `stats.json` (`--site FILE` elsewhere, with `relations.json` beside
it), and `scripts/wiki-data --check`, which `scripts/check` runs, fails if either isn't what the GML and the
specs give, writing nothing. So a change to a spec or to the extractor commits the new snapshots with it
(docs/wiki-style.md, "The pipeline").

## The pages' citations

A page cites the GML by hand too (`<Cite gml="objects/oSnake/step.gml:41-54" />`, a `cite`, `sources`, a
```` ```gml ```` excerpt), and the site's build can't check those either. `scripts/wiki-cites`, which
`scripts/check` runs after `scripts/wiki-data --check`, runs `tools`' `wiki-cites` binary on
`site/content/wiki/` (or the pages and directories given) against the same `gml/` tree the stats cite:
each path must be a file in it, each range inside the file, and each excerpt the lines it cites, whitespace
aside (docs/wiki-style.md, "Citing the code", has the rules and the messages). It doesn't parse a page's
YAML or MDX: it finds the paths by their shape wherever they are, so no syntax can hide one, and reads only
the excerpts' fences. Node, which could reuse the site's own compile step, would need the site's
`node_modules` in `scripts/check`, which has the Rust toolchain and the extracted assets but not those.
Its tests (`crates/tools/src/bin/wiki-cites.rs`) check a good page and a broken one.

## What's covered

The Mines monsters for the pilot pages (Snake, Bat, Spider, Giant Spider, Caveman, Skeleton), the Jungle's and
the Ice Caves' (Yeti, Yeti King, UFO, Alien, Alien Boss: `wiki/data/monsters.toml`), the weapons, the shop
items with their prices on every level, bombs and ropes (`items.toml`), and gold, gems and idols with their
value in each area (`treasure.toml`: gold and gems through Olmec's Lair, which the code counts as a fifth
area, idols through the Temple, and the gems' wait before they can be collected), the traps (`traps.toml`),
and the characters (`characters.toml`: the Damsel's, the Shopkeeper's and the Tunnel Man's health, speeds
and stuns, the price of a kiss and a Dice House bet on each level, and the shortcuts' prices, read from
`oGlobals`; the Spelunker's hearts, bombs, ropes and speeds; and the favour Kali's gifts take, and the
favour at which she devours a sacrifice). Every
other monster, item and treasure has its create-event numbers in `objects`, for the pages a later section adds
specs for.

## Relations

`relations.json`, beside `stats.json`, says how things relate: where level generation makes each one
(`spawns_in`), what each shop sells (`shops`, `sold_in`), what an object leaves behind or holds (`drops`),
and what hurts what (`damage`). Most of it isn't written down per thing: the tool reads it off the code,
and `wiki/data/relations.toml` only names what the code can't (the areas, the special levels, the shops)
and says where to look.

### How it's read

**Level generation** is followed from its start: `[generation]`'s `roots`, scrInitLevel (which oGame's
create event runs) and oGame's step (the ghost). From each, every `instance_create(x, y, oThing)` is found,
following the scripts called on the way, with their arguments bound where they're constants
(`scrGenerateItem(x, y, 2)` only reaches its `argument2 == 2` branch), and the create events of what's made
(a Mines brick, oBrick, may hold a gem or an item). It's walked once per `[[area]]`, with the area's
variable bound (`global.levelType = 0`): code under a condition that's then false (`global.levelType == 1`,
`global.darkLevel && rand(1, 40) == 1` with `global.darkLevel` false) isn't reached. A condition that
depends on anything else (a `rand`, a collision, an unbound variable) is taken to be possible, unless it's a
`rand` with whole-number arguments whose every value decides it alike: `if (rand(1,2))` is always true, so
its `else` isn't reached (docs/quirks.md §1). An area can
bind more than its variable: a global that's fixed there (the Ice Caves are never dark, `global.darkLevel =
0`), or a call, by its text as `--find` prints it (Olmec's Lair is the room rOlmec, `isRoom("rOlmec") = 1`,
which keeps oGame's ghost out of it). A `with (obj)` runs for no one where `instance_exists` is bound false
for `obj` or an ancestor of it: Olmec's Lair has no exit when it's made (`instance_exists(oExit) = 0`:
rOlmec's doors are oEntrances, and Olmec's death opens an oXEnd), so scrInitLevel's `with oExit` puts no
thief's Shopkeeper there. Such a bind is checked, not trusted: the area must bind its room (its `isRoom`),
and the tool fails if that room places one of the object or its descendants, or the area's walk makes
one.

A room script (scrRoomGen and its kin) lays a room out from strings, one character a tile, then places a
thing per character: `if (tile == "s") ... instance_create(xpos, ypos, oSnake)`. So whether a character
can come up is read off the room strings that hold it (`[generation]`'s `templates`, the variables they're
assigned to) and their conditions: the snake pit's `"s"` is only in the string under `roomPath == 8`. An
obstacle's string is placed by a character of the room's (`tile == "5"`), and is traced back to the rooms
holding that one. Which rooms the room path (`global.roomPath`) asks for isn't followed: scrLevelGen sets
it with loops the walk doesn't run. A `[[level]]`'s `rooms` say which room conditions make that level, and
`[generation]`'s `unreachable` lists the rooms it never asks for, with why (scrRoomGenYeti's copy of the
snake pit: a yeti lair never has a room 8 or 9).

A room's string is mostly picked by a `switch` on a roll, and which cases the roll reaches can hang on a
flag: the Temple's main rooms roll `if (global.cityOfGold) n = rand(1,12); else n = rand(1,10);`, so the
two layouts with a Tomb Lord in them (`case 11`, `case 12`) are only the City of Gold's. A case is ruled
out when every value the switch's variable can hold misses it: what its assignments before the switch set
(a constant, or a `rand` with constant bounds), counting those whose conditions can hold. Those are the
last assignment that always runs before the switch and any after it, or else ones that cover every way
there between them (both sides of an `if`, as above); anything else (a variable they don't pin down, a
`default:`) leaves every case possible. A `switch` on a `rand` itself reaches only the cases in its bounds.

A **special level** (`[[level]]`) is a `flag`, the global that's true on it (`global.darkLevel`), and its
`rooms`, the room scripts' conditions that build its own rooms (`{ script = "scrRoomGen", where =
"roomPath == 8" }`). A spawn is `only_in` a level when it can't happen off it: with the flag false and
outside its rooms. It's `not_in` a level when it can't happen on it, with the flag true. The rooms alone
never rule a spawn out (a level with a snake pit has its ordinary rooms too).

**Shops** (`[[shop]]`) are walks too, from the script that stocks one (`in`), with its variables bound
(`shopType = "Weapon"` in scrShopItemsGen) and, for those a room script stocks directly, the tiles to keep
(`where = ['tile == "q"']`, the dice house's prize, scrGenerateItem's high-end set).

**Drops** are what each object's own events (but its create event, which is generation's) make of the
monsters, items and treasure (the descendants of oEnemy, oItem and oTreasure), through the scripts they
call. They're keyed by what the conditions test touching where there is one (the player's step opens the
crate it's touching: `kUp && kAttackPressed && collision_point(x, y, oCrate, 0, 0)`), or else by the
object whose code it is (a jar's destroy event, a hanging spider's step that makes the spider). What a script
`[drops]`'s `not_via` names makes itself isn't a drop: a Monkey's trip makes the player drop what he holds
(scrDropItem), which puts the item he'd picked up back in his hand (scrHoldItem's `instance_create`) and
shoots a drawn bow's arrow (scrFireBow), and those are his, not the Monkey's. Each must be a script that
would otherwise make a drop, or the tool fails. What a script `given_by` names makes is its `object`'s,
though that isn't a monster, an item or a treasure, which touching would key: a body left on Kali's altar
is sacrificed (oEnemy's and oDamsel's step), and her gifts (scrGetFavorMsg's) are the altar's
(oSacAltarLeft), not every monster's. The tool fails if the object is one touching keys already, if code
reaches the script without testing touching the object (or being it), or if the script makes no drop.

**Damage** is every statement that matches a `[[damage]]` pattern (`global.plife -= ?` hurts the player,
`hp -= ?` the instance running it, `?.hp -= ?` the one the `?` names; `hp = 0` kills outright) in any
object's own events or any script. Who's hurt and what hurts are read from where the code is: in a
collision event, `self` is the object and `other` the collision's other object; in a `with (obj)`, `self`
is the target and `other` the instance that ran it (a bullet's `with other { hp -= 4; }` hurts the enemy
it hit); and a variable is the object it was found by (`obj = instance_nearest(x, y, oEnemy)`). The code's
own object is what hurts, or, when the code hurts itself, the collision's other object (and its
descendants), less those for which a test of the other's own variables can't hold as their create events
set them for good: a yeti's whip code hurts it only `if (other.puncture)`, so a machete's slash and a
mattock's hit do, and the whip, whose `puncture` is false, doesn't; its wind-up code only
`if (other.type == "Machete")`, so the machete's wind-up does, and the whip's and the mattock's (`type`
"Whip" and "Mattock") don't. A test is evaluated as generation's conditions are
(`other.type == "Machete" && hit == 0` is false when either side is), with each object's create-event
constants bound to `other`'s variables; one counts only if it isn't a built-in (`x`, `image_index`, which
the runner changes) and no code but a create event's sets it on the object's instances (an event of its
own, `obj.v = ...` on one created or named, a script's or a `with`'s, which could be any). When a test rules some out, the others are named one by one, and the ones ruled out
are left out of what a kept one's descendants count. Plus the objects the conditions test touching
(`collision_point(x, y+6, oLava, 0, 0)`), including through a variable set from such a test (the player's
`colSpikes`).

### Output

| Field | Contents |
| --- | --- |
| `format`, `notice`, `license` | As stats.json's |
| `areas`, `levels` | The specs' areas (`slug`, `name`, `bind`) and special levels (`slug`, `name`, `flag`, `rooms`) |
| `unreachable` | The spec's unreachable rooms (`script`, `where`, `why`) |
| `not_drops` | The spec's scripts that make no drops (`script`, `why`) |
| `given_by` | The spec's scripts whose drops are an object's (`script`, `object`, `why`) |
| `slugs` | Each stats entity's object, to its page's slug |
| `spawns_in` | Per object, each place generation makes it (below) |
| `placed_in` | Per object, the rooms the room editor placed it in (Olmec in rOlmec), and how many |
| `shops` | Per shop: `slug`, `name`, `in`, `bind`, `where`, and `sells`: `object`, `slug`, `price` (its create event's `cost`, stats.json's `objects`), `when`, `at`, `via` |
| `sold_in` | Per object, the shops (by slug) that stock it |
| `drops` | Per object, what it leaves behind or holds: `object` (and `slug`), `by` (the `object:event`), `runs_as` (the objects that run that code: it and the descendants that inherit the event), `trigger` (a collision event's other object), `involves`, `when`, `at`, `via` |
| `damage` | Each statement: `by`, `runs_as`, `trigger`, `find`, `target` (as the code says it: `player`, `self`, `other`, a variable), `victims` and `sources` (objects), `ruled_out` (the collision's other object and descendants its tests leave out), `amount` (with `value` or `range`), `when`, `at` |

A **spawn** has `areas` (the areas whose walk reaches it), `only_in` and `not_in` (special levels),
`tile` (a room script's character), `when` (the conditions around the `instance_create`, as a stat fact's),
`at`, and `via`, each chain of places the walk got there by (`scrInitLevel > scrRoomGen > oBrick:create`).
`when` and `at` are as in stats.json. The odds are left as the code gives them: a spawn's `when` ends in its
`rand(1, 60) == 1`, and an `else if` chain's earlier `not`s are the rolls that came first. The tool doesn't
multiply them into a probability, since that depends on how many tiles qualify.

The snake (the scrRoomGen spawn's `when` cut: between its loops and its tile, the `not`s of the tile
`if` chain's earlier branches):

```json
"oSnake": [
  {
    "areas": ["mines"],
    "only_in": ["snake-pit"],
    "tile": "s",
    "when": [{ "loop": "for (...; j < 8; ...)" }, "...", { "if": "tile == \"s\"" }, { "if": "rand(1, 10) == 1" }],
    "at": { "file": "gml/scripts/scrRoomGen.gml", "line": 464, "script": "scrRoomGen", "source": "instance_create(xpos, ypos, oSnake)" },
    "via": ["scrInitLevel > scrRoomGen"]
  },
  {
    "areas": ["mines"],
    "when": [
      { "if": "global.levelType == 0" },
      { "with": "oSolid" },
      { "if": "!isInShop(x, y) && y > 16" },
      { "if": "scrGetRoomX(x) != global.startRoomX || scrGetRoomY(y - 16) != global.startRoomY" },
      { "if": "!collision_point(x, y - 16, oSolid, 0, 0)" },
      { "if": "rand(1, 60) == 1" }
    ],
    "at": { "file": "gml/scripts/scrEntityGen.gml", "line": 69, "script": "scrEntityGen", "source": "instance_create(x, y-16, oSnake)" },
    "via": ["scrInitLevel > scrEntityGen"]
  }
]
```

### What's partial

- **Spawns** follow generation's code, not its numbers: the odds stay conditions, and which rooms the room
  path holds comes only from the `rooms` and `unreachable` specs. Things that appear during play are in
  `drops` instead (the Spider from oSpiderHang's step, the Giant Spider from oGiantSpiderHang's). The level
  editor's levels (scrCreateTileObj, from scrLoadLevel) aren't followed. A `switch`'s `default:` is
  never ruled out by a bound value (none of the code generation reaches has one). A case's values are read
  off its variable's assignments in the same code: a script called in between could change it too, and
  none in the room scripts does.
- **Drops** count only what an object's code makes of monsters, items and treasure. The key is a guess
  from the code's shape (what's touched, a `given_by`'s object, or else whose code), and the conditions say
  the rest.
- **Damage** is the patterns' statements. A kill by `instance_destroy()` (an enemy whose head is in lava)
  isn't one, nor is a stun with no damage. `victims` and `sources` are as far as the code says: an empty
  list is code that hurts whatever a variable holds, set some other way.
