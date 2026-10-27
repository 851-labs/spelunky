# Wiki data

The spelunky.ai wiki's numbers (a monster's health and damage, an item's price, a gem's value) and
relations (where a monster appears, what a shop sells, what a crate holds, what hurts what) come from the
game's own code, not from memory or another wiki, and each carries the file and line it came from, so a
page can cite the code and a reviewer can check it. Like the [media](wiki-media.md), the output isn't
committed: it's built from the extracted assets.

```sh
scripts/extract     # once: assets/
scripts/wiki-data   # target/wiki-data/stats.json and relations.json
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
kind = "monster"          # monster, character, weapon, item, equipment, treasure, trap or mechanic
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

## What's covered

The Mines monsters for the pilot pages (Snake, Bat, Spider, Giant Spider, Caveman, Skeleton:
`wiki/data/monsters.toml`), the weapons, the shop items with their prices on every level, bombs and ropes
(`items.toml`), and gold, gems and idols with their value in each area (`treasure.toml`). Every other
monster, item and treasure has its create-event numbers in `objects`, for the pages a later section adds
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
depends on anything else (a `rand`, a collision, an unbound variable) is taken to be possible.

A room script (scrRoomGen and its kin) lays a room out from strings, one character a tile, then places a
thing per character: `if (tile == "s") ... instance_create(xpos, ypos, oSnake)`. So whether a character
can come up is read off the room strings that hold it (`[generation]`'s `templates`, the variables they're
assigned to) and their conditions: the snake pit's `"s"` is only in the string under `roomPath == 8`. An
obstacle's string is placed by a character of the room's (`tile == "5"`), and is traced back to the rooms
holding that one. Which rooms the room path (`global.roomPath`) asks for isn't followed: scrLevelGen sets
it with loops the walk doesn't run. A `[[level]]`'s `rooms` say which room conditions make that level, and
`[generation]`'s `unreachable` lists the rooms it never asks for, with why (scrRoomGenYeti's copy of the
snake pit: a yeti lair never has a room 8 or 9).

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
object whose code it is (a jar's destroy event, a hanging spider's step that makes the spider).

**Damage** is every statement that matches a `[[damage]]` pattern (`global.plife -= ?` hurts the player,
`hp -= ?` the instance running it, `?.hp -= ?` the one the `?` names; `hp = 0` kills outright) in any
object's own events or any script. Who's hurt and what hurts are read from where the code is: in a
collision event, `self` is the object and `other` the collision's other object; in a `with (obj)`, the
target; and a variable is the object it was found by (`obj = instance_nearest(x, y, oEnemy)`). What hurts
also takes in the objects the conditions test touching (`collision_point(x, y+6, oLava, 0, 0)`), including
through a variable set from such a test (the player's `colSpikes`).

### Output

| Field | Contents |
| --- | --- |
| `format`, `notice`, `license` | As stats.json's |
| `areas`, `levels` | The specs' areas (`slug`, `name`, `bind`) and special levels (`slug`, `name`, `flag`, `rooms`) |
| `unreachable` | The spec's unreachable rooms (`script`, `where`, `why`) |
| `slugs` | Each stats entity's object, to its page's slug |
| `spawns_in` | Per object, each place generation makes it (below) |
| `placed_in` | Per object, the rooms the room editor placed it in (Olmec in rOlmec), and how many |
| `shops` | Per shop: `slug`, `name`, `in`, `bind`, `where`, and `sells`: `object`, `slug`, `price` (its create event's `cost`, stats.json's `objects`), `when`, `at`, `via` |
| `sold_in` | Per object, the shops (by slug) that stock it |
| `drops` | Per object, what it leaves behind or holds: `object` (and `slug`), `by` (the `object:event`), `runs_as` (the objects that run that code: it and the descendants that inherit the event), `trigger` (a collision event's other object), `involves`, `when`, `at`, `via` |
| `damage` | Each statement: `by`, `runs_as`, `trigger`, `find`, `target` (as the code says it: `player`, `self`, `other`, a variable), `victims` and `sources` (objects), `amount` (with `value` or `range`), `when`, `at` |

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
  editor's levels (scrCreateTileObj, from scrLoadLevel) aren't followed.
- **Drops** count only what an object's code makes of monsters, items and treasure. The key is a guess
  from the code's shape (what's touched, or else whose code), and the conditions say the rest.
- **Damage** is the patterns' statements. A kill by `instance_destroy()` (an enemy whose head is in lava)
  isn't one, nor is a stun with no damage. `victims` and `sources` are as far as the code says: an empty
  list is code that hurts whatever a variable holds, set some other way.
