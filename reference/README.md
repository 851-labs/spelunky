# reference

The original Spelunky Classic 1.1 files, committed unmodified. Everything the port imports is generated
from these. Nothing is copied by hand.

| File | What it is |
|---|---|
| `spelunky_1_1_src.zip` | The GameMaker 7/8 source Derek Yu released on 2009-12-25: `spelunky.gmk`, `config.gmk`, `COPYING.txt`, `readme.txt` |
| `spelunky_1_1.zip` | The released 1.1 game: `Spelunky.exe`, `supersound.dll`, `sound/`, `levels/`, default `.cfg` files. It's the reference oracle's input and the source of the sound files. |
| `SHA256SUMS` | Checksums of both zips (`shasum -a 256 -c SHA256SUMS`) |

**The source matches the release.** `Spelunky.exe` and `config.exe` from `spelunky_1_1.zip` were decompiled
with OpenGMK's decompiler (`gm8decompiler`, <https://github.com/OpenGMK/OpenGMK>) and the results compared with
`spelunky.gmk` and `config.gmk` from `spelunky_1_1_src.zip`, resource by resource. Every script, object, event,
action, room, tile, view and game setting is identical; only the GameMaker editor's own metadata differs.
So the port, transpiled from the `.gmk` files, and the oracle, running `Spelunky.exe`, run the same GML. Checked
2026-10-30 (audit 1b, 851-3771; recorded by 851-3800); to check again, decompile both executables the same way
and compare them with the `.gmk` files in the zip.

Source: <https://spelunkyworld.com/original.html> (downloaded 2026-10-22 from
`http://www.derekyu.com/games/spelunky_1_1_src.zip` and `http://www.derekyu.com/games/spelunky_1_1.zip`).

Copyright (c) 2008, 2009 Derek Yu and Mossmouth, LLC. Distributed under the Spelunky User License v1.1b
(`../LICENSE`, also inside both zips as `COPYING.txt`).
