# pid-input

Drives the native app (macOS) with real input from a script: a key goes down, stays down across frames and
comes up, the way a person types. The desktop automation tools only send instant taps, and GM8 drops a key
that's pressed and released within one frame: the any-key events see `keyboard_key` already 0, so the level
editor types nothing (docs/quirks.md §5). Every event goes to the app's process alone (`CGEventPostToPid`), so
it can run while someone else uses the machine: it never types into the app in front, and the real pointer
doesn't move.

```sh
swiftc -O -o /tmp/pid-input tools/pid-input/pid-input.swift
SPELUNKY_BACKGROUND=1 target/release/app --scale 3 --mute --record /tmp/session.txt --trace /tmp/live.jsonl &
/tmp/pid-input --pid $! --screen 960x720 tools/pid-input/examples/level-editor.txt
kill -INT <pid>   # closes the window, ending the recording
```

`SPELUNKY_BACKGROUND=1` makes the app open its window without activating, so it doesn't take the keyboard
from the app in front. Keys still reach it: AppKit hands a key posted to the process to its key window, active
or not.

## Scripts

One command per line; `#` starts a comment; words are separated by spaces or tabs. Times are milliseconds. The
first error stops the script, naming its line.

| Command | |
| --- | --- |
| `hold KEY [MS]` | key down, wait (default 100), key up |
| `down KEY`, `up KEY` | one half of that |
| `type WORD [MS]` | each character held MS (default 100), with an MS gap (`hold space` for a space) |
| `wait MS` | |
| `move X Y` | move the mouse (a drag while a button is down) |
| `press BUTTON X Y`, `release BUTTON X Y` | `left`, `right` or `middle` |
| `click BUTTON X Y [MS]` | press, hold MS (default 100), release |
| `wheel up`, `wheel down` | one notch, where the mouse last was |
| `shot PATH` | the window's picture (`screencapture -l`, which needs Screen Recording permission), covered or not |
| `front`, `pointer`, `echo TEXT` | print the app in front, the real pointer, or the text |

Keys are named as on a US keyboard: letters, digits, `enter`, `escape`, `backspace`, `space`, `tab`, arrows
(`left`), `f1`-`f12`, `shift`/`control`/`alt`. With `--screen WxH` (the game's back buffer: 960x720 at
`--scale 3` with the default 3x screen), mouse coordinates are GM window pixels, the ones a replay's `mouse`
lines use, mapped the way the app shows the buffer (the largest whole scale that fits, centred); without it,
points in the window's content.

## Mouse events in a background window

A mouse event posted to a process has no window: AppKit routes a click to the window the event names, and
drops one that names none. The tool sets the event's window in the undocumented field 51 and its position in
that window with the private `CGEventSetWindowLocation` (the public `kCGMouseEventWindowUnderMousePointer`
fields aren't what AppKit reads). Moves, drags, clicks and the wheel then arrive as they would under the real
pointer. Both are undocumented and may change with macOS; the tool fails loudly if the function is missing.

## Timing

The game reads input once a tick (30 a second). Hold a key for at least 100 ms so it spans a tick boundary. On
a loaded machine the app can fall behind and run late ticks back to back, and everything that arrived in the
gap lands in the first of them: a short hold then becomes a tap within one tick (which the editor ignores),
and a `move` followed quickly by a click lands in the same tick as the click (the editor then places the tile
at the cursor object's previous cell, as GM8 would). Generous holds and waits keep a script reliable; the
recording shows what the game actually got, tick by tick.
