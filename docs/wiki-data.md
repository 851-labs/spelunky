# Wiki data

The spelunky.ai wiki's numbers (a monster's health and damage, an item's price, a gem's value) come from
the game's own code, not from memory or another wiki, and each carries the file and line it came from, so a
page can cite the code and a reviewer can check it. Like the [media](wiki-media.md), the output isn't
committed: it's built from the extracted assets.

```sh
scripts/extract     # once: assets/
scripts/wiki-data   # target/wiki-data/stats.json
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
