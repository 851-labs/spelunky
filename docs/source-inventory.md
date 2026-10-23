# Spelunky Classic 1.1: GameMaker source inventory

Source: `spelunky.gmk` from `reference/spelunky_1_1_src.zip` (749,475 bytes, magic 1234321, GMK version 800, game id 834524).
The parser is a throwaway Python script (`tools/inventory-py/gmk_parse.py`) that follows LateralGM's
`GmFileReader.java`. Everything in this file comes from parsed data or from tokenizing the dumped GML. Where a
statement describes GameMaker *runner* behaviour (not the game's source), it is labelled "runner behaviour" and
should be checked against OpenGMK before it is relied on.

## Parse status

- **Fully parsed.** All 749,475 bytes were consumed: settings, triggers, constants, sounds, sprites, backgrounds,
  paths, scripts, fonts, timelines, objects, rooms, last instance/tile ids, include files, extension packages,
  game information, library creation code, room order and the resource tree. No warnings: every zlib block
  decompressed and was consumed exactly, and every boolean field was 0 or 1. Every per-resource version
  number matched the GM 8.0 values (sprite 800, background 710, path 530, script 800, font 800,
  timeline 500, object 430, room 541, action 440).
- Pixel data is skipped but checked. All 2,299 sprite frames and 29 backgrounds are stored as raw BGRA with
  `length == w*h*4`, so they can be decoded. There is no sound data because every sound slot is deleted.
- `config.gmk` (the separate `config.exe` settings tool) also parses cleanly. It is dumped to `config_tool/`.
- Unknown values: one object event, `core` mouse sub-event **12**, has no name in the GM 8.0 event table
  I used (0-11, 50-58, 60-61). It is in the leftover supersound demo object and doesn't affect gameplay.
  The file is `objects/core/mouse_12.gml`.

## Layout of this directory

| Path | Contents |
|---|---|
| `scripts/<name>.gml`, `scripts.json` | 139 scripts (code verbatim, with CRLF converted to LF), plus a folder/line index |
| `objects/<name>/<event>.gml` | Code for each event. Events with more than one action (or with DnD actions) get a `// ==== action N ... ====` header per action. Non-code DnD actions are written as one header line that includes lib/id/kind/function/applies_to/relative/not/args. |
| `objects/<name>/object.json` | Properties (parent, sprite, mask, solid, visible, persistent, depth, tree folder) and the full action list for every event. DnD action arguments are resolved to resource names. |
| `objects.json` | One-line index of all objects |
| `rooms/<name>.json`, `rooms.json` | Size, speed, persistence, colours, backgrounds, views, every instance (object/x/y/id/creation code) and every tile. No room has creation code, so there are no `.creation.gml` files. |
| `sprites.json`, `backgrounds.json`, `sounds.json`, `fonts.json`, `paths.json`, `timelines.json` | Metadata. The last four are `[]` because those resources don't exist. |
| `game_settings.json`, `game_information.rtf` | Global game settings, room order, library creation code and help text |
| `constants.json`, `triggers.json`, `includes_and_extensions.json` | All empty |
| `resource_tree.json` | The author's resource-tree folders. The object groups below come from this tree, not from name guessing. |
| `parse_report.json` | Byte offsets per section, slot counts and warnings |
| `config_tool/` | The same dump for `config.gmk` |

## 1. Counts

| Resource | Defined | ID slots (incl. deleted) | Notes |
|---|---:|---:|---|
| sprites | 804 | 1436 | 2299 frames total; all pixel data decodable (BGRA, len == w*h*4) |
| sounds | 0 | 54 | every slot deleted; all audio is external files via supersound.dll |
| backgrounds | 29 | 56 | all pixel data decodable |
| paths | 0 | 0 |  |
| scripts | 139 | 315 |  |
| fonts | 0 | 1 | 1 deleted slot; fonts are built at runtime with font_add_sprite |
| timelines | 0 | 2 | 2 deleted slots |
| objects | 453 | 944 | 1001 events, 969 code actions, 350 other DnD actions |
| rooms | 30 | 124 | 6330 placed instances, 9 tiles |
| triggers | 0 | - | |
| constants | 0 | - | |
| include files | 0 | - | |
| extension packages | 0 | - | |

GML lines (physical / non-blank / code lines excluding comments and blanks):

| Source | Units | Physical | Non-blank | Code |
|---|---:|---:|---:|---:|
| Scripts | 139 | 12500 | 11375 | 9684 |
| Object event code actions | 969 | 22548 | 20919 | 20007 |
| Room creation code | 0 | 0 | 0 | 0 |
| Instance creation code | 0 | 0 | 0 | 0 |
| **Total** | 1108 | 35048 | 32294 | 29691 |

Game settings that matter for a port (`game_settings.json`):

- **Window:** 320x240 views scaled through surfaces. GM "scaling" is fixed at 100%, and interpolation is off.
- **Keys:** F4 toggles fullscreen and F9 takes a screenshot. Esc does **not** end the game, and F1/F5/F6 are off.
- **Focus:** the game freezes when the window loses focus.
- **Errors:** "display errors" is on and "treat uninitialized variables as 0" is **off**, so reading an
  unset variable is a runtime error in the original.
- **Loading:** a custom loading image is shown.
- **Libraries:** the library creation code list has 3 entries, all empty.

There are no constants, triggers, include files or extension packages. The game information text is the
license (RTF).

## 2. Objects

453 objects in total: 345 have a parent and the maximum inheritance depth is 3. The parent roots are:

- `oDrawnSprite`: 332 descendants.
- `oMenu`: 12 descendants.
- `oLamp`: 1 descendant.
- 108 objects have no parent and no children.

The groups below are the author's own resource-tree folders. "(top level)" holds the controllers and room
drivers (`oPlayer1`, `oGame`, `oLevel`, `oRoom`, `oScreen`, `oMusic`, `oGlobals`, `oGamepad`, `oTitle`,
`oLevelEditor`, ...). The shared parents (`oDrawnSprite`, `oCharacter`, `oEnemy`, `oItem`, `oSolid`, ...) are
in "Basis".

Event totals across all objects:

- create 305, step 196, begin_step 1, end_step 3
- alarm 159 (alarm indices 0-11 are all used)
- collision 123, other_animation_end 51, destroy 46
- mouse 38, draw 36, keypress 22
- other_outside 11, other_room_end 6, other_game_end 3, other_close_button 1

Notable absences: no continuous keyboard events, no room_start, no user events.

Only 36 objects have a draw event. The other 417 rely on GM's **default draw**, so the port needs default
sprite drawing with `image_index`, `image_xscale`/`yscale`, `image_angle`, `image_blend` and `image_alpha`.

Column key: Sol = solid, Pers = persistent, Vis = visible ("N" when invisible). LOC is counted over the
object's own code actions only, as physical lines / code lines.


#### Characters (6 objects, 1207 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oDamsel | oItem | sDamselLeft | - |  |  |  | 100 | create, destroy, step, coll:oWhip, coll:oWhipPre, animation_end, draw | 462/430 |
| oYellHelp | oDrawnSprite | sYellHelp | - |  |  |  | 1 | create, alarm0, step | 4/4 |
| oDamselKiss | oDrawnSprite | sDamselLeft | - | Y |  |  | 100 | create, step, room_end, animation_end | 24/21 |
| oShopkeeper | oEnemy | sShopLeft | - |  |  |  | 60 | create, alarm0, step, coll:oCharacter, coll:oWhip, coll:oWhipPre, coll:oShotgun, animation_end, draw | 734/657 |
| oTunnelMan | - | sTunnelManLeft | - |  |  |  | 100 | create, step | 91/83 |
| oShopkeeper2 | oShopkeeper | sShopLeft | - |  |  |  | 60 | create, step | 13/12 |

#### Basis (17 objects, 935 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oSolid | oDrawnSprite | - | - |  |  |  | 100 | create, destroy | 40/38 |
| oMovingSolid | oSolid | - | - |  |  |  | 0 | create | 1/1 |
| oPlatform | oDrawnSprite | - | - |  |  |  | 0 | - | 0/0 |
| oLadder | oDrawnSprite | - | - |  |  |  | 0 | - | 0/0 |
| oMoveableSolid | oSolid | - | - |  |  |  | 0 | create | 3/3 |
| oCharacter | oDrawnSprite | - | - |  | Y |  | 0 | - | 0/0 |
| oWater | oDrawnSprite | - | - |  |  |  | 0 | create | 2/2 |
| oBackdrop | - | - | - |  |  |  | 0 | create | 1/1 |
| oForeground | - | - | - |  |  |  | 0 | create | 1/1 |
| oEnemy | oDrawnSprite | - | - |  | Y |  | 0 | create, destroy, step, coll:oCharacter, coll:oWhip, coll:oWhipPre, outside, draw | 403/356 |
| oItem | oDrawnSprite | - | - | Y |  |  | 0 | create, destroy, alarm2, step, draw | 451/387 |
| oDetritus | oDrawnSprite | - | - |  |  |  | 0 | create, step | 49/40 |
| oFrill | oDrawnSprite | - | - |  |  |  | 0 | - | 0/0 |
| oTreasure | oDrawnSprite | - | - |  |  |  | 101 | create, step | 95/73 |
| oDrawnSprite | - | - | - |  |  |  | 0 | create | 2/2 |
| oRubblePiece | oDrawnSprite | - | - |  |  |  | 0 | create, step | 34/31 |
| oMenu | - | - | - |  |  |  | 0 | - | 0/0 |

#### Enemies (40 objects, 5263 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oCaveman | oEnemy | sCavemanLeft | - |  |  |  | 60 | create, step, coll:oCharacter, coll:oWhip, coll:oWhipPre | 319/291 |
| oBat | oEnemy | sBatHang | - |  |  |  | 40 | create, step | 118/104 |
| oSpider | oEnemy | sSpiderFlip | - |  |  |  | 40 | create, alarm0, alarm1, step, animation_end | 132/107 |
| oSpiderHang | oEnemy | sSpiderHang | - |  |  |  | 40 | create, step | 45/39 |
| oPiranha | oEnemy | sPiranhaLeft | - |  |  |  | 40 | create, alarm0, step | 174/156 |
| oUFO | oEnemy | sUFO | - |  |  |  | 40 | create, step, animation_end | 164/145 |
| oManTrap | oEnemy | sManTrapLeft | - |  |  |  | 60 | create, step, coll:oCharacter, coll:oCaveman, coll:oDamsel, coll:oWhip, coll:oWhipPre, coll:oShopkeeper, animation_end | 376/334 |
| oSnake | oEnemy | sSnakeLeft | - |  |  |  | 60 | create, step | 122/95 |
| oFrog | oEnemy | sFrogLeft | - |  |  |  | 40 | create, step | 125/102 |
| oEnemySight | - | sSight | - |  |  | N | 0 | create, coll:oSolid, coll:oCharacter | 18/17 |
| oOlmec | oMovingSolid | sOlmecStart1 | - |  |  |  | 40 | create, alarm1, alarm2, alarm3, alarm4, alarm5, alarm6, step | 299/274 |
| oOlmecSlam | - | sOlmecSlam | - |  |  | N | 0 | create, alarm0, coll:oSolid | 9/9 |
| oYeti | oEnemy | sYetiLeft | - |  |  |  | 60 | create, alarm0, step, coll:oCharacter, coll:oWhip, coll:oWhipPre, animation_end | 395/360 |
| oHawkman | oEnemy | sHawkLeft | - |  |  |  | 60 | create, alarm0, step, coll:oCharacter, coll:oWhip, coll:oWhipPre, animation_end | 370/335 |
| oEntranceCheck | - | sSight | - |  |  | N | 0 | step, coll:oSolid | 6/6 |
| oAlien | oEnemy | sAlien | - |  |  |  | 60 | create, step | 103/89 |
| oAlienEject | oDrawnSprite | sAlienEject | - |  |  |  | 60 | create, step, coll:oPlayer1, coll:oWeb, outside, animation_end | 97/87 |
| oOlmecDebris | oDrawnSprite | sOlmecDebris | - |  |  |  | 1 | create, step | 37/30 |
| oMagmaMan | oEnemy | sMagmaManLeft | - |  |  |  | 60 | create, alarm0, step, coll:oCharacter, coll:oEnemy, coll:oBomb, coll:oWhip, animation_end | 196/171 |
| oMagma | oDetritus | sMagma | - |  |  |  | 1 | create, alarm0, step, coll:oCharacter, coll:oWater, coll:oEnemy, animation_end | 61/58 |
| oMagmaTrail | oDrawnSprite | sFlameTrail | - |  |  |  | 2 | create, animation_end | 0/0 |
| oSkeleton | oEnemy | sSkeletonCreateL | - |  |  |  | 60 | create, step | 126/88 |
| oAlienBoss | oEnemy | sAlienBoss | - |  |  |  | 60 | create, step, coll:oCharacter, animation_end | 151/131 |
| oGhost | oDrawnSprite | sGhostRight | - |  |  |  | 0 | create, step, coll:oCharacter, coll:oWhip, animation_end | 103/87 |
| oZombie | oEnemy | sZombieLeft | - |  |  |  | 40 | create, step | 148/127 |
| oDeadFish | oEnemy | sDeadFishLeft | - |  |  |  | 40 | create, step | 103/91 |
| oGiantSpiderHang | oEnemy | sGiantSpiderHang | - |  |  |  | 40 | create, step | 66/59 |
| oGiantSpider | oEnemy | sGiantSpiderFlip | - |  |  |  | 40 | create, alarm0, alarm1, step, coll:oCharacter, coll:oWhip, animation_end | 234/199 |
| oBarrierEmitter | - | sBarrierEmitter | - |  |  |  | 100 | create, destroy, step, coll:oItem, coll:oBullet | 26/26 |
| oBarrier | - | sBarrier | - |  |  |  | 108 | coll:oCharacter, coll:oItem, coll:oBullet, coll:oWebBall | 41/37 |
| oCavemanWorship | - | sCavemanWorshipR | - |  |  |  | 60 | create | 1/1 |
| oHawkmanWorship | - | sHawkLeft | - |  |  |  | 0 | - | 0/0 |
| oScarab | oEnemy | sScarab | - |  |  |  | 40 | create, destroy, step, coll:oCharacter | 100/87 |
| oTombLord | oEnemy | sTombLordLeft | - |  |  |  | 60 | create, step, coll:oCharacter, coll:oWhip, animation_end, draw | 220/200 |
| oMonkey | oEnemy | sMonkeyLeft | - |  |  |  | 40 | create, step, coll:oCharacter, coll:oItem | 400/350 |
| oUFOCrash | - | sUFOCrash | - |  |  |  | 0 | create, alarm0, step, coll:oSolid, coll:oEnemy | 15/15 |
| oYetiKing | oEnemy | sYetiKingLeft | - |  |  |  | 60 | create, step, coll:oCharacter, coll:oWhip, animation_end, draw | 247/227 |
| oFireFrog | oEnemy | sFireFrogLeft | - |  |  |  | 40 | create, step | 125/102 |
| oJaws | oEnemy | sJawsLeft | - |  |  |  | 40 | create, step, animation_end, draw | 215/190 |
| oVampire | oEnemy | sVampireLeft | - |  |  |  | 60 | create, alarm0, step, coll:oCharacter, coll:oBlood, coll:oWhip, coll:oWhipPre | 479/437 |

#### Items (37 objects, 1316 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oRock | oItem | sRock | - |  |  |  | 100 | create | 3/3 |
| oBomb | oItem | sBomb | - |  |  |  | 99 | create, destroy, alarm0, alarm1, step, end_step | 45/42 |
| oArrow | oItem | sArrowRight | - |  |  |  | 100 | create, alarm1, alarm2, step, draw | 52/48 |
| oRopeThrow | oItem | sRopeEnd | - |  |  |  | 100 | create, step | 42/41 |
| oRope | oLadder | sRope | - |  |  |  | 200 | create, step | 21/20 |
| oRopeTop | oDrawnSprite | sRopeTop | - |  |  |  | 200 | create | 3/3 |
| oFishBone | oItem | sFishBone | - |  |  |  | 100 | create, alarm2, step | 29/25 |
| oGoldIdol | oItem | sGoldIdol | - |  |  |  | 100 | create, step | 38/37 |
| oWhip | oDrawnSprite | sWhipRight | - |  |  |  | 1 | create, step | 18/17 |
| oWhipPre | oDrawnSprite | sWhipPreR | - |  |  |  | 1 | create, alarm0, step | 19/18 |
| oFlare | oItem | sFlare | - |  |  |  | 30 | create, alarm0, step | 31/29 |
| oJar | oItem | sJar | - |  |  |  | 100 | create, destroy, step, coll:oWhip, coll:oBullet | 239/214 |
| oSkull | oItem | sSkull | - |  |  |  | 100 | create, destroy, step, coll:oBullet | 206/185 |
| oShotgun | oItem | sShotgunRight | - |  |  |  | 101 | create, step | 10/10 |
| oParachute | - | sParaOpen | - |  |  |  | 900 | step, coll:oItem, animation_end | 11/11 |
| oMachete | oItem | sMacheteRight | - |  |  |  | 101 | create, step | 10/10 |
| oSlash | oWhip | sSlashRight | - |  |  |  | 1 | create, step, animation_end | 20/19 |
| oMachetePre | oWhipPre | sMachetePreR | - |  |  |  | 1 | create, alarm0, step | 18/17 |
| oPistol | oItem | sPistolRight | - |  |  |  | 101 | create, step | 10/10 |
| oBasketball | oItem | sBasketball | - | Y |  |  | 100 | create, step | 65/61 |
| oMattock | oItem | sMattockRight | - |  |  |  | 101 | create, step | 10/10 |
| oMattockPre | oWhipPre | sMattockPreR | - |  |  |  | 1 | create, alarm0, step | 18/17 |
| oMattockHit | oWhip | sMattockHitR | - |  |  |  | 1 | create, step, animation_end | 78/70 |
| oMattockHead | oItem | sMattockHead | - |  |  |  | 100 | create | 3/3 |
| oWebCannon | oItem | sWebCannonR | - |  |  |  | 101 | create, step | 10/10 |
| oBow | oItem | sBowRight | - |  |  |  | 101 | create, step | 25/20 |
| oDice | oItem | sDice1 | - |  |  |  | 100 | create, step, draw | 244/218 |
| oSceptre | oItem | sSceptreRight | - |  |  |  | 101 | create, step | 10/10 |
| oTeleporter | oItem | sTeleporter | - |  |  |  | 101 | create | 5/5 |
| oCrystalSkull | oGoldIdol | sCrystalSkull | - |  |  |  | 100 | create | 2/2 |
| oFireFrogBomb | oItem | sFireFrogArmedL | - |  |  |  | 99 | create, alarm1, step | 45/43 |
| oBall | oItem | sBall | - |  |  |  | 99 | create, step | 43/41 |
| oChain | oDrawnSprite | sChain | - |  |  |  | 1 | create, step | 11/11 |
| oBall2 | oItem | sBall | - |  |  |  | 99 | create, step | 12/12 |
| oChain2 | oDrawnSprite | sChain | - |  |  |  | 1 | create, step | 12/12 |
| oLampItem | oItem | sLampItem | - |  |  |  | 100 | create | 6/6 |
| oLampRedItem | oItem | sLampRedItem | - |  |  |  | 100 | create | 6/6 |

#### Collectibles (40 objects, 356 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oGoldNugget | oTreasure | sGoldNugget | - | Y |  |  | 100 | create, alarm0 | 6/6 |
| oGoldChunk | oTreasure | sGoldChunk | - | Y |  |  | 100 | create | 6/6 |
| oChest | oItem | sChest | - |  |  |  | 900 | create, step | 8/6 |
| oRubyBig | oTreasure | sRubyBig | - | Y |  |  | 101 | create, alarm0, coll:oGhost | 9/9 |
| oSapphireBig | oTreasure | sSapphireBig | - | Y |  |  | 101 | create, alarm0, coll:oGhost | 9/9 |
| oEmeraldBig | oTreasure | sEmeraldBig | - | Y |  |  | 101 | create, alarm0, coll:oGhost | 9/9 |
| oRuby | oTreasure | sRuby | - | Y |  |  | 101 | create, alarm0 | 7/7 |
| oSapphire | oTreasure | sSapphire | - | Y |  |  | 101 | create, alarm0 | 7/7 |
| oEmerald | oTreasure | sEmerald | - | Y |  |  | 101 | create, alarm0 | 7/7 |
| oBombBag | oItem | sBombBag | - |  |  |  | 100 | create | 6/6 |
| oGoldBar | oTreasure | sGoldBar | - | Y |  |  | 100 | create | 5/5 |
| oGoldBars | oTreasure | sGoldBars | - | Y |  |  | 100 | create | 6/6 |
| oRopeBag | oItem | sRopeEnd | - |  |  |  | 100 | create | 3/3 |
| oCrate | oItem | sCrate | - |  |  |  | 900 | create, step | 8/6 |
| oFakeBones | oDrawnSprite | sFakeBonesLeft | - |  |  |  | 900 | create, step, animation_end | 26/24 |
| oBones | oDrawnSprite | sBonesLeft | - |  |  |  | 900 | create, step | 12/11 |
| oRopePile | oItem | sRopePile | - |  |  |  | 100 | create | 6/5 |
| oParaUsed | oDrawnSprite | sParaUsed | - |  |  |  | 900 | create, step | 12/11 |
| oBombBox | oItem | sBombBox | - |  |  |  | 100 | create | 7/7 |
| oPaste | oItem | sPaste | - |  |  |  | 100 | create | 7/6 |
| oParaPickup | oItem | sParaPickup | - |  |  |  | 100 | create | 6/5 |
| oSpringShoes | oItem | sSpringShoes | - |  |  |  | 100 | create | 6/6 |
| oJordans | oItem | sJordans | - |  |  |  | 100 | create | 6/5 |
| oCompass | oItem | sCompass | - |  |  |  | 100 | create | 6/5 |
| oSpectacles | oItem | sSpectacles | - |  |  |  | 100 | create | 6/5 |
| oJetpack | oItem | sJetpackRight | - |  |  |  | 100 | create, step | 9/8 |
| oCapePickup | oItem | sCape | - |  |  |  | 100 | create | 6/5 |
| oCape | oDrawnSprite | sCapeRight | - |  |  |  | 100 | create, step | 51/50 |
| oGloves | oItem | sGlove | - |  |  |  | 100 | create | 7/6 |
| oLockedChest | oItem | sLockedChest | - |  |  |  | 900 | create, step, coll:oKey | 29/26 |
| oKey | oItem | sKeyRight | - |  |  |  | 101 | create, step | 9/9 |
| oUdjatEye | oItem | sUdjatEye | - |  |  |  | 100 | create | 6/5 |
| oAnkh | oItem | sAnkh | - |  |  |  | 101 | create | 6/5 |
| oCrown | oItem | sCrown | - |  |  |  | 100 | create | 6/5 |
| oMitt | oItem | sMitt | - |  |  |  | 100 | create | 7/6 |
| oDiamond | oTreasure | sDiamond | - | Y |  |  | 101 | create, alarm0 | 7/7 |
| oSpikeShoes | oItem | sSpikeShoes | - |  |  |  | 100 | create | 6/5 |
| oFireFrogArmed | oItem | sFireFrogArmedL | - |  |  |  | 900 | create, step | 8/6 |
| oFlareCrate | oItem | sFlareCrate | - |  |  |  | 900 | create, alarm0, step | 28/26 |
| oKapala | oItem | sKapala | - |  |  |  | 100 | create | 6/5 |

#### Traps (22 objects, 1177 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oSpikes | oDrawnSprite | sSpikes | - |  |  |  | 120 | create | 1/1 |
| oArrowTrapRight | oSolid | sArrowTrapRight | - |  |  |  | 110 | create, destroy, alarm0, alarm1, step | 80/38 |
| oTemp | oSolid | sArrowTrapLeft | - | Y |  |  | 110 | create, alarm0, step | 49/45 |
| oBoulder | oMovingSolid | sBoulder | - |  |  |  | 200 | create, step, coll:oSolid | 127/101 |
| oGiantTikiHead | oDrawnSprite | sGiantTikiHead | - |  |  |  | 997 | alarm0 | 3/3 |
| oSpearTrapTop | oSolid | sSpearTrap1 | - |  |  |  | 100 | create, destroy, alarm0, step | 105/91 |
| oSpearsLeft | oDrawnSprite | sSpearsLeft | - |  |  |  | 999 | step, animation_end | 9/9 |
| oSpearTrapBottom | oSolid | sSpearTrap2 | - |  |  |  | 100 | create, destroy, alarm0, step | 106/92 |
| oThwompTrap | oMovingSolid | sThwompTrap | - |  |  |  | 60 | create, step | 53/50 |
| oSmashTrap | oMovingSolid | sSmashTrap | - |  |  |  | 60 | create, destroy, step | 173/163 |
| oCeilingTrap | oMovingSolid | sBlock | - |  |  |  | 60 | create, destroy, step, animation_end | 48/44 |
| oDoor | oMovingSolid | sDoor | - |  |  |  | 9000 | create, destroy, step | 41/37 |
| oTrapBlock | oSolid | sSkullBlock | - |  |  |  | 110 | create, destroy, step | 21/21 |
| oArrowTrapLeft | oSolid | sArrowTrapLeft | - |  |  |  | 110 | create, destroy, alarm0, alarm1, step | 76/35 |
| oArrowRepeaterR | oSolid | sArrowTrapRight | - |  |  |  | 110 | create, destroy, alarm0, alarm1, step | 33/31 |
| oArrowRepeaterL | oSolid | sArrowTrapLeft | - |  |  |  | 110 | create, destroy, alarm0, alarm1, step | 33/31 |
| oSpringTrap | - | sSpringTrap | - |  |  |  | 100 | create, step, coll:oPlayer1, coll:oEnemy, coll:oItem, animation_end | 63/62 |
| oSpearTrapLit | oSpearTrapTop | sSpearTrapLit | - |  |  |  | 100 | - | 0/0 |
| oArrowTrapRightLit | oArrowTrapRight | sArrowTrapRightLit | - |  |  |  | 110 | - | 0/0 |
| oArrowTrapLeftLit | oArrowTrapLeft | sArrowTrapLeftLit | - |  |  |  | 110 | - | 0/0 |
| oSmashTrapLit | oMovingSolid | sSmashTrapLit | - |  |  |  | 60 | create, destroy, step | 173/163 |
| oArrowTrapTest | - | sRed | - |  |  | N | 0 | create, coll:oMoveableSolid, coll:oCharacter, coll:oEnemy, coll:oItem, coll:oBoulder, coll:oTreasure | 160/160 |

#### Blocks (58 objects, 794 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oBrick | oSolid | sBrick | - |  |  |  | 100 | create, destroy | 42/39 |
| oLadderOrange | oLadder | sLadder | - |  |  |  | 1000 | - | 0/0 |
| oHardBlock | oSolid | sBrick | - |  |  |  | 100 | create | 1/1 |
| oBlock | oSolid | sBlock | - |  |  |  | 110 | create, destroy | 27/26 |
| oLadderTop | oPlatform | sLadderTop | - |  |  |  | 1000 | - | 0/0 |
| oEntrance | oDrawnSprite | sEntrance | - |  |  |  | 9000 | create, draw | 9/9 |
| oExit | oDrawnSprite | sExit | - |  |  |  | 9000 | create, draw | 10/9 |
| oGold | oDrawnSprite | sGold | - | Y |  |  | 100 | destroy | 6/6 |
| oPushBlock | oMoveableSolid | sBlock | - |  |  |  | 110 | create, destroy, step, outside | 36/36 |
| oGoldBig | oDrawnSprite | sGoldBig | - | Y |  |  | 100 | destroy | 10/9 |
| oLush | oSolid | sLush | - |  |  |  | 100 | create, destroy | 45/41 |
| oWaterSwim | oWater | sWater | - |  |  |  | 9003 | destroy | 4/4 |
| oSplash | oDrawnSprite | sSplash | - |  |  |  | 1 | create, animation_end | 2/2 |
| oVine | oLadder | sVine | - |  |  |  | 1000 | - | 0/0 |
| oDark | oSolid | sDark | - |  |  |  | 100 | create, destroy | 43/40 |
| oIce | oSolid | sIce | - |  |  |  | 110 | create, destroy | 17/15 |
| oVineTop | oLadder | sVineTop | - |  |  |  | 1000 | - | 0/0 |
| oCaveBG | - | sCaveBG1 | - |  |  |  | 99999999 | create | 9/8 |
| oDarkFall | oMovingSolid | sDarkFall | - |  |  |  | 100 | create, step, coll:oSolid | 27/23 |
| oFrozenCaveman | oDrawnSprite | sFrozenCaveman | - |  |  |  | 100 | destroy, step | 8/8 |
| oTempleFake | - | sTemple | - |  |  |  | 60 | step | 5/5 |
| oLava | oWater | sLava | - |  |  |  | 20 | create, destroy, step | 29/29 |
| oLavaSolid | oSolid | sLava | - | Y |  |  | 110 | create | 1/1 |
| oTree | oSolid | sTreeTrunk | - |  |  |  | 2 | create, destroy, step | 16/16 |
| oLeaves | oPlatform | sLeaves | - |  |  |  | 1 | create, destroy, step | 49/46 |
| oTreeBranch | oPlatform | sTreeBranchRight | - |  |  |  | 2 | create, destroy, step | 13/13 |
| oTemple | oSolid | sTemple | - |  |  |  | 100 | create, destroy | 50/46 |
| oCaveBG2 | - | sCaveBG | - |  |  |  | 998 | - | 0/0 |
| oCaveBGEntrance | - | sCaveBGEntrance | - |  |  |  | 998 | - | 0/0 |
| oBlackFadeUp | oMenu | sBlackFadeUp | - |  |  |  | 999999 | - | 0/0 |
| oAltarLeft | oSolid | sAltarLeft | - |  |  |  | 110 | destroy | 9/9 |
| oAltarRight | oSolid | sAltarRight | - |  |  |  | 110 | destroy | 9/9 |
| oAlienShip | oSolid | sAlienTop | - |  |  |  | 100 | create, destroy | 4/3 |
| oAlienShipFloor | oSolid | sAlienFloor | - |  |  |  | 100 | create, destroy | 4/3 |
| oBrickSmooth | oSolid | sCaveSmooth | - |  |  |  | 110 | destroy | 9/9 |
| oThinIce | oSolid | sThinIce1 | - |  |  |  | 110 | create, step | 14/13 |
| oSign | oSolid | sSignGeneral | - |  |  |  | 110 | destroy | 9/9 |
| oIceBottom | oDrawnSprite | sIceBottom2 | - |  |  |  | 110 | create, alarm0, step | 4/4 |
| oGrave | oSolid | sGrave | - |  |  |  | 110 | create, destroy, step | 20/20 |
| oLamp | - | sLamp | - |  |  |  | 201 | create | 1/1 |
| oMsgSign | oDrawnSprite | sMsgSign | - |  |  |  | 998 | create, coll:oCharacter, draw | 130/112 |
| oXMarket | oExit | sExit | - |  |  |  | 9000 | create | 1/1 |
| oMoai | oSolid | sMoai | - |  |  |  | 100 | create | 1/1 |
| oMoaiInside | oSolid | sMoaiInside | - |  |  |  | 100 | create | 1/1 |
| oMoai2 | oSolid | sMoai2 | - |  |  |  | 100 | create | 1/1 |
| oMoai3 | oSolid | sMoai3 | - |  |  |  | 100 | create | 1/1 |
| oGoldDoor | oDrawnSprite | sGoldDoor | - |  |  |  | 10000 | coll:oSceptre | 23/22 |
| oXGold | oExit | sExit | - |  |  |  | 9000 | - | 0/0 |
| oLampRed | oLamp | sLampRed | - |  |  |  | 201 | create | 1/1 |
| oIceBlock | oMoveableSolid | sIceBlock | - |  |  |  | 110 | create, destroy, step | 18/18 |
| oTikiTorch | - | sTikiTorch | - |  |  |  | 8999 | create | 1/1 |
| oXocBlock | oSolid | sGoldBlock | - |  |  |  | 110 | create, destroy | 34/32 |
| oSacAltarLeft | oSolid | sSacAltarLeft | - |  |  |  | 110 | create, destroy, step | 74/69 |
| oSacAltarRight | oSacAltarLeft | sSacAltarRight | - |  |  |  | 110 | - | 0/0 |
| oKaliHead | oDrawnSprite | sKaliHead1 | - |  |  |  | 997 | create, alarm0 | 14/14 |
| oTreeTile | oSolid | sTreeTrunk | - |  |  |  | 2 | create, step | 3/2 |
| oLeavesTile | oPlatform | sLeaves | - |  |  |  | 1 | create, step | 15/5 |
| oTreeBranchTile | oPlatform | sTreeBranchLeft | - |  |  |  | 2 | create, step | 11/1 |

#### Effects (41 objects, 652 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oBlood | oDetritus | sBlood | - |  |  |  | 1 | create, alarm0, alarm1, alarm2, step | 24/22 |
| oExplosion | oDrawnSprite | sExplosion | - |  |  |  | 1 | create, coll:oSolid, coll:oEnemy, coll:oItem, coll:oBoulder, coll:oDamsel, coll:oWeb, coll:oBarrierEmitter, animation_end | 122/114 |
| oBloodTrail | oDrawnSprite | sBloodTrail | - |  |  |  | 1 | create, animation_end | 0/0 |
| oFlame | oDetritus | sFlame | - |  |  |  | 1 | create, destroy, alarm0, step, coll:oWater | 18/15 |
| oFlameTrail | oDrawnSprite | sFlameTrail | - |  |  |  | 1 | create, animation_end | 0/0 |
| oDrip | oRubblePiece | sDrip | - |  |  |  | 1 | create | 1/1 |
| oSmokePuff | oDrawnSprite | sSmokePuff | - |  |  |  | 1 | create, step, animation_end | 12/5 |
| oSmallCollect | oDrawnSprite | sSmallCollect | - |  |  |  | 1 | create, alarm0, step | 5/5 |
| oBubble | oDrawnSprite | sBubble | - |  |  |  | 1 | create, step, animation_end | 10/9 |
| oBigCollect | oDrawnSprite | sBigCollect | - |  |  |  | 1 | create, alarm0, step | 2/2 |
| oRubble | oRubblePiece | sRubble | - |  |  |  | 1 | - | 0/0 |
| oRubbleSmall | oRubblePiece | sRubbleSmall | - |  |  |  | 1 | - | 0/0 |
| oLaser | oDrawnSprite | sLaser | - |  |  |  | 1 | create, alarm0, step, coll:oSolid, coll:oEnemy, coll:oDamsel, outside | 61/48 |
| oLaserTrail | oDrawnSprite | sLaserTrail | - |  |  |  | 1 | create, animation_end | 0/0 |
| oLaserExplode | oDrawnSprite | sLaserExplode | - |  |  |  | 1 | create, animation_end | 1/1 |
| oRubbleDark | oDrawnSprite | sRubbleDark | - |  |  |  | 1 | create, step, outside | 10/9 |
| oRubbleDarkSmall | oRubblePiece | sRubbleDarkSmall | - |  |  |  | 1 | outside | 0/0 |
| oLavaDrip | oRubblePiece | sLavaDrip | - |  |  |  | 1 | create, animation_end | 1/1 |
| oYellowBall | oDrawnSprite | sYellowBall | - |  |  |  | 1 | create, alarm0, alarm1, step, coll:oSolid, outside | 54/27 |
| oBurn | oDrawnSprite | sBurn | - |  |  |  | 999 | create, step, animation_end | 10/9 |
| oItemsGet | oDrawnSprite | sItemGet | - |  |  |  | 1 | create, alarm0, step | 8/8 |
| oHeart | oDrawnSprite | sSmoochHeart | - |  |  |  | 1 | create, alarm0, step | 2/2 |
| oFlareSpark | oDrawnSprite | sFlareSpark | - |  |  |  | 150 | create, step, animation_end | 10/9 |
| oRopeBurn | oDrawnSprite | sBurn | - |  |  |  | 99 | create, step | 17/15 |
| oBone | oDetritus | sBone | - |  |  |  | 1 | create, step, animation_end | 12/11 |
| oPsychicWave | oDrawnSprite | sPsychicWave | - |  |  |  | 1 | create, step, coll:oEnemy, coll:oDamsel, outside, animation_end | 27/22 |
| oPsychicCreate | oDrawnSprite | sPsychicCreate | - |  |  |  | 1 | create, step, animation_end | 7/7 |
| oShotgunBlastLeft | - | sShotgunBlastLeft | - |  |  |  | 0 | create, animation_end | 1/1 |
| oShotgunBlastRight | - | sShotgunBlastRight | - |  |  |  | 0 | create, animation_end | 1/1 |
| oBullet | - | sBullet | - |  |  |  | 0 | create, alarm0, step, coll:oSolid, coll:oCharacter, coll:oEnemy, coll:oDamsel | 91/83 |
| oVolcanoFlame | - | sVolcanoFlame | - |  |  |  | 1 | create, alarm0, step, outside | 14/9 |
| oPoof | oDrawnSprite | sPoof | - |  |  |  | 1 | create, step, animation_end | 6/6 |
| oPsychicCreate2 | oDrawnSprite | sPsychicCreate | - |  |  |  | 1 | create, step, animation_end | 6/6 |
| oLeaf | oRubblePiece | sLeaf | - |  |  |  | 1 | create | 4/4 |
| oYellowTrail | oDrawnSprite | sYellowTrail | - |  |  |  | 1 | create, animation_end | 0/0 |
| oWebBall | oDrawnSprite | sWebBall | - |  |  |  | 1 | create, alarm0, alarm1, step, coll:oSolid, coll:oWater, coll:oEnemy, coll:oItem, coll:oWeb, outside, animation_end | 52/40 |
| oWeb | - | sWeb | - |  |  |  | 200 | create, step, coll:oWater, coll:oEnemy, coll:oItem, coll:oLaser, coll:oTreasure, coll:oRubblePiece, coll:oSlash | 32/30 |
| oFly | - | sFlyLeft | - |  |  |  | 0 | create, step, coll:oSolid, coll:oCharacter, coll:oEnemy, coll:oDamsel | 87/81 |
| oPsychicWaveP | oDrawnSprite | sPsychicWaveP | - |  |  |  | 1 | create, step, coll:oEnemy, coll:oDamsel, outside, animation_end | 43/38 |
| oPsychicCreateP | oDrawnSprite | sPsychicCreate | - |  |  |  | 1 | create, step, coll:oSolid, animation_end | 7/7 |
| oBloodSpark | oDrawnSprite | sBloodSpark | - |  |  |  | 1 | create, step, animation_end | 4/4 |

#### Init (11 objects, 169 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oRadioBox | - | sBox | - |  |  |  | 0 | create, mouse:left_press | 39/35 |
| oCheckBox | - | sBoxChecked | - |  |  |  | 0 | create, mouse:left_press | 18/18 |
| oOKButton | - | sOKButton | - |  |  |  | 0 | mouse:left_press, mouse:left_release, mouse:leave | 8/8 |
| oCheckBox2 | - | sBoxChecked | - |  |  |  | 0 | create, mouse:left_press | 18/18 |
| oCheckBox3 | - | sBoxChecked | - |  |  |  | 0 | create, mouse:left_press | 18/18 |
| oCheckBox4 | - | sBoxChecked | - |  |  |  | 0 | create, mouse:left_press | 18/18 |
| oKeyConfigButton | - | sKeyConfigButton | - |  |  |  | 0 | mouse:left_press, mouse:left_release, mouse:leave | 4/4 |
| oJoyConfigButton | - | sJoyConfigButton | - |  |  |  | 0 | mouse:left_press, mouse:left_release, mouse:leave | 4/4 |
| oSliderBG | - | sSliderBG | - |  |  |  | 0 | - | 0/0 |
| oSlider1 | - | sSlider | - |  |  |  | 0 | create, step, mouse:left_press | 21/21 |
| oSlider2 | - | sSlider | - |  |  |  | 0 | create, alarm0, step, mouse:left_press | 25/25 |

#### (top level) (29 objects, 7158 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oGame | - | - | - |  |  |  | 0 | create, alarm0, alarm1, alarm2, step, draw, kpress:M | 366/320 |
| oPlayer1 | oCharacter | sStandLeft | - |  |  |  | 50 | create, alarm0, alarm1, alarm2, alarm3, alarm4, alarm10, alarm11, step, end_step, coll:oBlood, animation_end, draw, kpress:bksp, kpress:enter | 2620/2287 |
| oRoom | - | sRoom | - |  |  | N | 0 | - | 0/0 |
| oGlobals | - | - | - |  | Y | N | 0 | create | 71/48 |
| oHighscores | - | - | - |  |  |  | 0 | create, step, draw, kpress:Q | 307/270 |
| oLevel | - | - | - |  |  |  | 0 | create, step, room_end, draw | 210/133 |
| oTransition | - | - | - |  |  |  | 0 | create, alarm0, alarm1, step, draw | 575/535 |
| oSprite | oDrawnSprite | - | - |  |  |  | 1 | create | 1/1 |
| oBricks | - | - | - |  |  |  | 0 | create | 2/2 |
| oFinalBoss | - | - | - |  |  |  | 0 | create, step | 21/20 |
| oEnd | - | - | - |  |  |  | 0 | create, step, room_end | 53/43 |
| oEnd2 | - | - | - |  |  |  | 0 | create, alarm0, alarm1, alarm2, step | 21/17 |
| oEnd3 | - | - | - |  |  |  | 0 | create, alarm0, alarm1, alarm2, alarm3, alarm4, alarm5, alarm11, step, draw | 144/135 |
| oIntro | - | - | - |  |  |  | 0 | create, alarm8, alarm9, alarm10, alarm11, step, draw | 132/127 |
| oTitle | - | - | - |  |  |  | 0 | create, alarm0, alarm1, alarm2, alarm3, step, draw, kpress:F2, kpress:F3 | 203/177 |
| oScreen | - | - | - |  | Y | N | 0 | create, begin_step, end_step, game_end, room_end, close_button, kpress:F1, kpress:F10 | 379/358 |
| oCredits1 | - | - | - |  |  |  | 0 | create, alarm0, alarm1, alarm2, alarm3, alarm4, alarm5, alarm6, alarm7, alarm8, alarm11, step, draw | 193/180 |
| oStarsRoom | - | - | - |  |  |  | 0 | create, alarm0, alarm1, alarm2, alarm3, alarm10, alarm11, draw | 69/66 |
| oMoonRoom | - | - | - |  |  |  | 0 | create, alarm0, alarm1, alarm2, alarm3, alarm9, alarm10, alarm11, draw | 109/79 |
| oSunRoom | - | - | - |  |  |  | 0 | create, alarm0, alarm1, alarm2, alarm3, alarm10, alarm11, step, draw | 111/108 |
| oMusic | - | - | - |  | Y | N | 0 | create, game_end | 65/63 |
| oCredits2 | - | - | - |  |  |  | 0 | create, alarm0, alarm1, alarm2, alarm3, alarm4, alarm5, alarm6, alarm7, alarm8, alarm11, step, draw | 200/188 |
| oGamepad | - | - | - |  | Y |  | 0 | create, step | 493/397 |
| oKeyConfig | - | - | - |  |  |  | 0 | create, room_end, draw, kpress:any | 252/247 |
| oJoyConfig | - | - | - |  |  |  | 0 | create, step, room_end, draw | 214/207 |
| oLevelEditor | - | - | - |  |  |  | 1 | create, step, mouse:wheel_up, mouse:wheel_down, draw, kpress:any, kpress:esc, kpress:1, kpress:2, kpress:3, kpress:4, kpress:A, kpress:N, kpress:S, kpress:Y | 936/862 |
| oCursObj | - | sEntrance | - |  |  | N | 0 | create, step | 12/12 |
| oLoadLevel | - | - | - |  |  |  | 0 | create, step, draw, kpress:any | 162/135 |
| oEndCustom | - | - | - |  |  |  | 0 | create, alarm0, alarm1, alarm2, alarm3, alarm4, alarm5, alarm11, step, draw | 150/141 |

#### Other (22 objects, 425 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oCaveUp | oDrawnSprite | sCaveUp | - |  |  |  | 0 | - | 0/0 |
| oPDummy | oDrawnSprite | sRunLeft | - |  |  |  | 50 | create, alarm0, alarm1, alarm2, alarm3, alarm4, alarm5, step, animation_end, draw | 162/140 |
| oBlack | oMenu | sBlack | - |  |  |  | 1 | - | 0/0 |
| oMenuTop | oMenu | sMenuTop | - |  |  |  | 1 | - | 0/0 |
| oMenuBottom | oMenu | sMenuBottom | - |  |  |  | 1 | - | 0/0 |
| oMenuLeft | oMenu | sMenuLeft | - |  |  |  | 1 | - | 0/0 |
| oMenuRight | oMenu | sMenuRight | - |  |  |  | 1 | - | 0/0 |
| oMenuUL | oMenu | sMenuUL | - |  |  |  | 1 | - | 0/0 |
| oMenuUR | oMenu | sMenuUR | - |  |  |  | 1 | - | 0/0 |
| oMenuLL | oMenu | sMenuLL | - |  |  |  | 1 | - | 0/0 |
| oMenuLR | oMenu | sMenuLR | - |  |  |  | 1 | - | 0/0 |
| oPDummy2 | oDrawnSprite | sRunLeft | - |  |  |  | 997 | create, alarm0, alarm1, alarm2, step, draw | 140/129 |
| oPDummy3 | oDrawnSprite | sRunLeft | - |  |  |  | 40 | create, alarm0, alarm2, step, draw | 65/58 |
| oPDummy4 | oDrawnSprite | sRunLeft | - |  |  |  | 50 | create, alarm2, step | 43/38 |
| oPDummy5 | oDrawnSprite | sRunLeft | - |  |  |  | 50 | create, step | 18/13 |
| oBlackBG | oMenu | sBlack | - |  |  |  | 9999999 | - | 0/0 |
| oHintHand | - | sHintPoint | - |  |  |  | 0 | create, draw | 15/14 |
| oPDummy6 | oDrawnSprite | sStandLeft | - |  |  |  | 50 | create, draw | 6/4 |
| oPageUp | - | sPageUp | - |  |  |  | 0 | mouse:left_press, mouse:left_release, mouse:leave, kpress:up | 10/8 |
| oPageDown | - | sPageDown | - |  |  |  | 0 | mouse:left_press, mouse:left_release, mouse:leave, kpress:down | 10/8 |
| oMenuSel | oMenu | sMenuSelOff | - |  |  |  | 1 | mouse:left_release | 7/7 |
| oWalDamsel | - | sWalDamsel | - |  |  |  | 100 | create, step | 6/6 |

#### Other/Intro (17 objects, 115 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oDesert | oSolid | sDesert | - | Y |  |  | 100 | create | 2/2 |
| oDesertTop | - | sDesertTop | - |  |  |  | 10 | create | 2/2 |
| oIntroBG | - | sIntroBG | - |  |  |  | 9999 | create, step | 10/10 |
| oMoon | - | sMoon | - |  |  |  | 99999 | create, step | 10/10 |
| oPalmTreeDark | - | sPalmTreeDark | - |  |  |  | 999 | - | 0/0 |
| oShrubDark | - | sShrubDark | - |  |  |  | 998 | - | 0/0 |
| oShrub | - | sShrub | - |  |  |  | 998 | - | 0/0 |
| oPalmTree | - | sPalmTree | - |  |  |  | 999 | - | 0/0 |
| oDesertScroll | oDrawnSprite | sDesert | - | Y |  |  | 100 | create, step | 6/4 |
| oDesertTopScroll | - | sDesertTop | - |  |  |  | 10 | create, step | 6/5 |
| oShrubScroll | oDrawnSprite | sShrub | - |  |  |  | 998 | create, step | 6/5 |
| oPalmTreeScroll | oDrawnSprite | sPalmTree | - |  |  |  | 999 | create, step | 6/5 |
| oCamel | - | sCamel | - |  |  |  | 200 | create, step | 23/22 |
| oCaravan | - | sCaravan1 | - |  |  |  | 200 | create, step | 28/27 |
| oBatIntro | - | sBatLeft | - |  |  |  | 0 | create, step | 20/17 |
| oDesert2 | oSolid | sDesert2 | - | Y |  |  | 100 | create | 2/2 |
| oDesertScroll2 | oDrawnSprite | sDesert2 | - | Y |  |  | 100 | create, step | 6/4 |

#### Other/Title (33 objects, 169 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oTitleLogo | oSolid | sTitle | - |  |  |  | 1 | - | 0/0 |
| oTitleBG | oDrawnSprite | sTitleBG | - |  |  |  | 999999 | - | 0/0 |
| oXStart | oDrawnSprite | sExit | - |  |  |  | 999999 | - | 0/0 |
| oXScores | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oXTitle | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oXEnd | oXStart | sExit | - |  |  |  | 9000 | - | 0/0 |
| oQuitSign | oSolid | sQuitSign | - |  |  |  | 999 | - | 0/0 |
| oResetSign | oSolid | sResetSign | - |  |  |  | 999 | - | 0/0 |
| oButtonHighscore | - | sButton | - |  |  |  | 999 | create, step | 34/32 |
| oStartSign | - | sStartSign | - |  |  |  | 999 | - | 0/0 |
| oScoresSign | - | sScoresSign | - |  |  |  | 999 | - | 0/0 |
| oLevel5Sign | - | sLevel5Sign | - |  |  |  | 999 | - | 0/0 |
| oLevel9Sign | - | sLevel9Sign | - |  |  |  | 999 | - | 0/0 |
| oXShortcut5 | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oXShortcut9 | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oXSun | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oXMoon | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oXStars | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oTrophy | - | sBronzeTrophy | - |  |  |  | 900 | - | 0/0 |
| oBasket | - | sBasket | - |  |  |  | 99 | animation_end | 1/1 |
| oRim | - | sRim | - |  |  |  | 99 | - | 0/0 |
| oRimDeflect | - | sRimDeflect | - |  |  |  | 99 | - | 0/0 |
| oXChange | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oNew | - | sNew | - |  |  |  | 0 | - | 0/0 |
| oTarget | - | sTarget | - |  |  |  | 0 | create, step, coll:oArrow | 71/68 |
| oTutorialSign | oSolid | sTutorialSign | - |  |  |  | 999 | - | 0/0 |
| oXTutorial | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oLevel13Sign | - | sLevel13Sign | - |  |  |  | 999 | - | 0/0 |
| oXShortcut13 | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oXChange2 | oXStart | sExit | - |  |  |  | 999999 | - | 0/0 |
| oMultiTrophy | - | sMultiGold | - |  |  |  | 900 | - | 0/0 |
| oBatTarget | - | sBatLeft | - |  |  |  | 0 | create, step, coll:oCharacter, coll:oArrow | 77/68 |
| oCopy | - | sCopy | - |  |  |  | 0 | - | 0/0 |

#### Other/End (9 objects, 108 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oEndPlat | oSolid | sEndPlat | - | Y |  |  | 999 | create, step | 3/2 |
| oBigChest | oDrawnSprite | sBigChest | - |  |  |  | 0 | - | 0/0 |
| oBigTreasure | - | sBigTreasure | - | Y |  |  | 997 | create, step | 56/47 |
| oLavaSpray | - | sLavaSpray | - |  |  |  | 999 | create, alarm0, step | 24/21 |
| oEnd2BG | - | sEnd2BG | - |  |  |  | 9999 | - | 0/0 |
| oPlayerSil | oDrawnSprite | sPlayerSil | - |  |  |  | 1 | create, step | 15/13 |
| oTreasureSil | oDrawnSprite | sTreasureSil | - |  |  |  | 1 | create, step | 18/15 |
| oBGEnd3 | - | sBGEnd3 | - |  |  |  | 9999 | create, step | 11/10 |
| oEndWall | - | sEndWall | - |  |  |  | 9999 | - | 0/0 |

#### Level Editor (68 objects, 73 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| oBatTile | oDrawnSprite | sBatHang | - |  |  |  | 100 | - | 0/0 |
| oSnakeTile | oDrawnSprite | sSnakeLeft | - |  |  |  | 100 | - | 0/0 |
| oSpiderTile | oDrawnSprite | sSpiderHang | - |  |  |  | 100 | - | 0/0 |
| oGoldBarTile | oDrawnSprite | sGoldBarTile | - |  |  |  | 100 | - | 0/0 |
| oGoldBarsTile | oDrawnSprite | sGoldBarsTile | - |  |  |  | 100 | - | 0/0 |
| oEmeraldBigTile | oDrawnSprite | sEmeraldBigTile | - |  |  |  | 100 | - | 0/0 |
| oSapphireBigTile | oDrawnSprite | sSapphireBigTile | - |  |  |  | 100 | - | 0/0 |
| oRubyBigTile | oDrawnSprite | sRubyBigTile | - |  |  |  | 100 | - | 0/0 |
| oChestTile | oDrawnSprite | sChestTile | - |  |  |  | 100 | - | 0/0 |
| oCrateTile | oDrawnSprite | sCrateTile | - |  |  |  | 100 | - | 0/0 |
| oDamselTile | oDrawnSprite | sDamselLeftIco | - |  |  |  | 100 | - | 0/0 |
| oRockTile | oDrawnSprite | sRockTile | - |  |  |  | 100 | - | 0/0 |
| oJarTile | oDrawnSprite | sJarTile | - |  |  |  | 100 | - | 0/0 |
| oGoldIdolTile | oDrawnSprite | sGoldIdolTile | - |  |  |  | 100 | - | 0/0 |
| oDiamondTile | oDrawnSprite | sDiamondTile | - |  |  |  | 100 | - | 0/0 |
| oGiantSpiderTile | oDrawnSprite | sGiantSpiderHang | - |  |  |  | 200 | create | 1/1 |
| oCavemanTile | oDrawnSprite | sCavemanLeft | - |  |  |  | 100 | - | 0/0 |
| oYetiTile | oDrawnSprite | sYetiLeft | - |  |  |  | 100 | - | 0/0 |
| oHawkmanTile | oDrawnSprite | sHawkLeft | - |  |  |  | 100 | - | 0/0 |
| oManTrapTile | oDrawnSprite | sManTrapLeft | - |  |  |  | 100 | create | 1/1 |
| oSkeletonTile | oDrawnSprite | sSkeletonLeft | - |  |  |  | 100 | - | 0/0 |
| oTombLordTile | oDrawnSprite | sTombLordLeft | - |  |  |  | 200 | create | 2/2 |
| oMonkeyTile | oDrawnSprite | sMonkeyLeft | - |  |  |  | 100 | - | 0/0 |
| oFrogTile | oDrawnSprite | sFrogLeft | - |  |  |  | 100 | create | 1/1 |
| oZombieTile | oDrawnSprite | sZombieLeft | - |  |  |  | 100 | create | 1/1 |
| oShopkeeperTile | oDrawnSprite | sShopLeftIco | - |  |  |  | 100 | - | 0/0 |
| oAlienTile | oDrawnSprite | sAlien | - |  |  |  | 100 | create | 1/1 |
| oPiranhaTile | oDrawnSprite | sPiranhaTile | - |  |  |  | 100 | - | 0/0 |
| oWaterTile | oDrawnSprite | sWater | - |  |  |  | 999 | - | 0/0 |
| oLavaTile | oDrawnSprite | sLava | - |  |  |  | 100 | - | 0/0 |
| oEditButton | - | sEditButton | - |  |  |  | 0 | step, mouse:left_press, mouse:left_release, mouse:leave | 14/13 |
| oBonesTile | oDrawnSprite | sFakeBonesLeft | - |  |  |  | 100 | - | 0/0 |
| oUFOTile | oDrawnSprite | sUFO | - |  |  |  | 100 | create | 1/1 |
| oSmashTrapTile | oDrawnSprite | sSmashTrap | - |  |  |  | 100 | - | 0/0 |
| oSpearTrapTileTop | oDrawnSprite | sSpearTrap1 | - |  |  |  | 100 | - | 0/0 |
| oSpearTrapTileBot | oDrawnSprite | sSpearTrap2 | - |  |  |  | 100 | - | 0/0 |
| oBombBagTile | oDrawnSprite | sBombBagTile | - |  |  |  | 100 | - | 0/0 |
| oBombBoxTile | oDrawnSprite | sBombBoxTile | - |  |  |  | 100 | - | 0/0 |
| oBombPasteTile | oDrawnSprite | sBombPasteTile | - |  |  |  | 100 | - | 0/0 |
| oRopePileTile | oDrawnSprite | sRopePileTile | - |  |  |  | 100 | - | 0/0 |
| oSpectaclesTile | oDrawnSprite | sSpectaclesTile | - |  |  |  | 100 | - | 0/0 |
| oGlovesTile | oDrawnSprite | sGlovesTile | - |  |  |  | 100 | - | 0/0 |
| oCapeTile | oDrawnSprite | sCapeTile | - |  |  |  | 100 | - | 0/0 |
| oSpringShoesTile | oDrawnSprite | sSpringShoesTile | - |  |  |  | 100 | - | 0/0 |
| oParachuteTile | oDrawnSprite | sParachuteTile | - |  |  |  | 100 | - | 0/0 |
| oMacheteTile | oDrawnSprite | sMacheteTile | - |  |  |  | 100 | - | 0/0 |
| oMittTile | oDrawnSprite | sMittTile | - |  |  |  | 100 | - | 0/0 |
| oMattockTile | oDrawnSprite | sMattockTile | - |  |  |  | 100 | - | 0/0 |
| oPistolTile | oDrawnSprite | sPistolTile | - |  |  |  | 100 | - | 0/0 |
| oShotgunTile | oDrawnSprite | sShotgunTile | - |  |  |  | 100 | - | 0/0 |
| oWebCannonTile | oDrawnSprite | sWebCannonTile | - |  |  |  | 100 | - | 0/0 |
| oTeleporterTile | oDrawnSprite | sTeleporterTile | - |  |  |  | 100 | - | 0/0 |
| oBowTile | oDrawnSprite | sBowTile | - |  |  |  | 100 | - | 0/0 |
| oCompassTile | oDrawnSprite | sCompassTile | - |  |  |  | 100 | - | 0/0 |
| oJetpackTile | oDrawnSprite | sJetpackTile | - |  |  |  | 100 | - | 0/0 |
| oSpikeShoesTile | oDrawnSprite | sSpikeShoesTile | - |  |  |  | 100 | - | 0/0 |
| oCrystalSkullTile | oDrawnSprite | sCrystalSkullTile | - |  |  |  | 100 | - | 0/0 |
| oYetiKingTile | oDrawnSprite | sYetiKingLeft | - |  |  |  | 200 | create | 1/1 |
| oMegaMouthTile | oDrawnSprite | sMegaMouthTile | - |  |  |  | 200 | create | 1/1 |
| oFireFrogTile | oDrawnSprite | sFireFrogLeft | - |  |  |  | 100 | create | 1/1 |
| oSpringTrapTile | oDrawnSprite | sSpringTrap | - |  |  |  | 100 | - | 0/0 |
| oIceTile | oDrawnSprite | sIce | - |  |  |  | 100 | - | 0/0 |
| oWebTile | oDrawnSprite | sWebTile | - |  |  |  | 100 | - | 0/0 |
| oLoadButton | - | sLoadButton | - |  |  |  | 1 | create, step, mouse:left_press, mouse:left_release, mouse:leave | 20/20 |
| oTestButton | - | sTestButton | - |  |  |  | 0 | step, mouse:left_press, mouse:left_release, mouse:leave | 14/14 |
| oNewButton | - | sNewButton | - |  |  |  | 0 | step, mouse:left_press, mouse:left_release, mouse:leave | 13/13 |
| oVampireTile | oDrawnSprite | sVampireLeft | - |  |  |  | 100 | create | 1/1 |
| oAlienBossTile | oDrawnSprite | sAlienBossTile | - |  |  |  | 200 | create | 1/1 |

#### supersound (3 objects, 90 code lines)

| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |
|---|---|---|---|:-:|:-:|:-:|---:|---|---:|
| core | - | - | - |  |  |  | 0 | create, mouse:12, game_end, draw | 126/89 |
| mascara | - | s_0 | - |  |  |  | 0 | create | 5/1 |
| mascara2 | - | s_1 | - |  |  |  | 0 | - | 0/0 |

## 3. Scripts

Call sites count GML calls plus DnD `action_execute_script`. Scripts that are never called:
`SS_FreeSound`, `SS_IsHandleValid`, `checkFlarePressed`, `getIdCollisionCharacter`, `getIdCollisionCharacterTop`, `isCollisionCharacter`, `isCollisionMovingSolidBottom`.

| Folder | Script | Lines (phys/code) | Call sites |
|---|---|---:|---:|
| (top level) | rand | 2/2 | 1363 |
| (top level) | scrClearGlobals | 169/123 | 9 |
| (top level) | scrUpdateHighscores | 119/76 | 4 |
| (top level) | scrResetHighscores | 37/13 | 2 |
| (top level) | scrDrawHUD | 185/156 | 1 |
| (top level) | scrGetKey | 61/39 | 18 |
| (top level) | scrGetJoy | 35/13 | 18 |
| (top level) | scrGetChar | 65/43 | 11 |
| (top level) | scrCreateBlood | 33/8 | 94 |
| (top level) | scrCreateFlame | 31/7 | 5 |
| (top level) | scrGetFavorMsg | 112/88 | 2 |
| (top level) | scrCheckCollisions | 54/24 | 6 |
| (top level) | scrInit | 212/174 | 1 |
| Audio | playMusic | 7/7 | 25 |
| Audio | stopAllMusic | 9/8 | 16 |
| Audio | playSound | 1/1 | 321 |
| Audio | startMusic | 81/79 | 4 |
| Audio | scrMusicFade | 13/12 | 1 |
| Character | scrHoldItem | 66/41 | 9 |
| Character | scrStealItem | 325/292 | 4 |
| Character | scrUseItem | 685/619 | 1 |
| Character | scrPlayerIsDucking | 34/10 | 17 |
| Character | scrFireBow | 74/47 | 6 |
| Character | scrDropItem | 45/17 | 6 |
| Collision | setCollisionBounds | 11/4 | 117 |
| Collision | isCollisionSolid | 8/2 | 8 |
| Collision | isCollisionLadder | 10/4 | 2 |
| Collision | isCollisionPlatform | 8/2 | 2 |
| Collision | isCollisionCharacter | 12/8 | 0 |
| Collision | getIdCollisionCharacter | 11/7 | 0 |
| Collision | canPushMoveableSolids | 6/1 | 2 |
| Collision | canLandOnPlatforms | 6/1 | 1 |
| Collision/hidden | approximatelyZero | 7/4 | 12 |
| Collision/hidden | calculateCollisionBounds | 11/4 | 22 |
| Collision/hidden | isCollisionRectangle | 24/11 | 1 |
| Collision/horizontal | isCollisionLeft | 11/6 | 60 |
| Collision/horizontal | isCollisionRight | 11/6 | 60 |
| Collision/horizontal | isCollisionCharacterLeft | 18/11 | 1 |
| Collision/horizontal | isCollisionCharacterRight | 18/11 | 1 |
| Collision/horizontal | getIdCollisionLeft | 8/2 | 1 |
| Collision/horizontal | getIdCollisionRight | 8/2 | 1 |
| Collision/horizontal | isCollisionMoveableSolidLeft | 11/6 | 1 |
| Collision/horizontal | isCollisionMoveableSolidRight | 11/6 | 1 |
| Collision/vertical | isCollisionTop | 9/4 | 51 |
| Collision/vertical | isCollisionBottom | 9/4 | 90 |
| Collision/vertical | isCollisionCharacterTop | 20/11 | 5 |
| Collision/vertical | isCollisionCharacterBottom | 19/11 | 2 |
| Collision/vertical | isCollisionPlatformBottom | 11/6 | 3 |
| Collision/vertical | isCollisionMovingSolidBottom | 9/4 | 0 |
| Collision/vertical | isCollisionWaterTop | 9/4 | 1 |
| Collision/vertical | getIdCollisionCharacterTop | 16/7 | 0 |
| Control | checkJumpPressed | 4/4 | 1 |
| Control | checkLeft | 4/4 | 1 |
| Control | checkRight | 4/4 | 1 |
| Control | checkLeftReleased | 4/4 | 1 |
| Control | checkLeftPressed | 4/4 | 1 |
| Control | checkRightPressed | 4/4 | 1 |
| Control | checkRightReleased | 4/4 | 1 |
| Control | checkUp | 4/4 | 2 |
| Control | checkDown | 4/4 | 2 |
| Control | checkJump | 4/4 | 1 |
| Control | checkJumpReleased | 4/4 | 1 |
| Control | checkAttackPressed | 4/4 | 12 |
| Control | checkItemPressed | 4/4 | 1 |
| Control | checkBombPressed | 4/4 | 2 |
| Control | checkRopePressed | 4/4 | 2 |
| Control | checkRun | 4/4 | 1 |
| Control | checkAttack | 4/4 | 4 |
| Control | checkUpPressed | 4/4 | 1 |
| Control | checkDownPressed | 4/4 | 1 |
| Control | checkStartPressed | 4/4 | 10 |
| Control | checkAttackReleased | 4/4 | 1 |
| Control | checkPayPressed | 4/4 | 1 |
| Control | checkFlarePressed | 4/4 | 0 |
| Control | checkJoyButton | 23/23 | 1 |
| In Level | scrShake | 11/4 | 9 |
| In Level | isRoom | 9/2 | 99 |
| In Level | isInShop | 13/5 | 17 |
| In Level | isLevel | 8/2 | 27 |
| In Level | scrShopkeeperAnger | 25/18 | 13 |
| In Level | getKissValue | 1/1 | 6 |
| In Level | isRealLevel | 8/2 | 70 |
| Level Editor | scrCreateTile | 393/367 | 6 |
| Level Editor | scrCreateTileObj | 417/393 | 1 |
| Level Editor | scrSetCursorTile | 103/96 | 4 |
| Level Editor | scrLoadLevel | 195/158 | 4 |
| Level Editor | scrTestLevel | 96/69 | 2 |
| Level Generation | scrRoomGen | 484/409 | 1 |
| Level Generation | scrLevelGen | 298/239 | 1 |
| Level Generation | scrGetRoomX | 32/6 | 48 |
| Level Generation | scrGetRoomY | 33/7 | 54 |
| Level Generation | scrEntityGen | 548/473 | 1 |
| Level Generation | scrRoomGen2 | 544/471 | 1 |
| Level Generation | scrRoomGen3 | 525/456 | 1 |
| Level Generation | scrRoomGen4 | 562/491 | 1 |
| Level Generation | scrInitLevel | 297/205 | 1 |
| Level Generation | scrRoomGen5 | 215/160 | 1 |
| Level Generation | scrTreasureGen | 134/103 | 5 |
| Level Generation | scrSetupWalls | 515/392 | 4 |
| Level Generation | scrRoomGenYeti | 453/376 | 1 |
| Level Generation | scrGetName | 61/36 | 7 |
| Level Generation | scrShopItemsGen | 192/167 | 6 |
| Level Generation | scrRoomGenMarket | 427/358 | 1 |
| Level Generation | scrCheckWaterTop | 51/26 | 3 |
| Level Generation | scrGenerateItem | 90/67 | 11 |
| Platform Engine | makeActive | 8/4 | 100 |
| Platform Engine | characterCreateEvent | 114/81 | 1 |
| Platform Engine | moveTo | 110/85 | 44 |
| Platform Engine | platformCharacterIs | 22/7 | 41 |
| Platform Engine | characterStepEvent | 1032/804 | 1 |
| Platform Engine | characterDrawEvent | 62/43 | 1 |
| Platform Engine | gameStepEvent | 273/214 | 1 |
| Platform Engine | characterSprite | 195/141 | 1 |
| Platform Engine | scrMoveableSolidRecurseDrop | 11/10 | 2 |
| Stat Tracking | scrWriteStats | 234/196 | 1 |
| Stat Tracking | scrReadStats | 186/150 | 1 |
| supersound | SS_Init | 199/160 | 2 |
| supersound | SS_LoadSound | 19/2 | 63 |
| supersound | SS_PlaySound | 16/2 | 3 |
| supersound | SS_LoopSound | 16/2 | 3 |
| supersound | SS_StopSound | 16/2 | 12 |
| supersound | SS_SetSoundFreq | 21/6 | 17 |
| supersound | SS_SetSoundPan | 23/6 | 5 |
| supersound | SS_SetSoundVol | 30/12 | 99 |
| supersound | SS_FreeSound | 20/3 | 0 |
| supersound | SS_Unload | 11/1 | 3 |
| supersound/Advanced | SS_IsSoundPlaying | 27/2 | 35 |
| supersound/Advanced | SS_GetSoundPosition | 19/2 | 2 |
| supersound/Advanced | SS_SetSoundPosition | 19/3 | 1 |
| supersound/Advanced | SS_GetSoundLength | 18/2 | 1 |
| supersound/Advanced | SS_IsHandleValid | 20/6 | 0 |
| supersound/Advanced | SS_PauseSound | 26/2 | 1 |
| supersound/Advanced | SS_ResumeSound | 19/2 | 1 |
| supersound/Advanced | SS_IsSoundPaused | 21/2 | 3 |
| supersound/Advanced | SS_GetSoundVol | 17/2 | 2 |
| supersound/Advanced | SS_GetSoundPan | 19/2 | 2 |
| supersound/Advanced | SS_GetSoundFreq | 18/2 | 7 |
| supersound/Advanced | SS_IsSoundLooping | 22/2 | 1 |
| supersound/Advanced | SS_GetSoundBytesPerSecond | 15/2 | 1 |

## 4. Rooms

The room order is shown below; index 0 is the first room. **Room speed is 30 in all 30 rooms.** No room is
persistent and no room has creation code. No instance in any room has creation code either.

Only 9 tiles exist in the whole file: `rHighscores` has 4, and `rTitle`, `rStars`, `rMoon`, `rSun` and `rTutorial` have 1 each.
Level terrain is built at runtime with `instance_create` and `tile_add`. `rLevel` is a 4x4 grid of `oRoom`
placeholders plus `oPlayer1`, `oGame` and `oLevel`.

`oScreen`'s create event rewrites view 0 of 26 rooms at runtime with `room_set_view`. It sets a 320x240 view,
a port of `320*scale` x `240*scale`, and follows `oPlayer1` with a 128x96 border in the level rooms. The
editor-stored views in the table below are therefore overridden at startup.

| # | Room | Size | Speed | Views on | View 0 (view / port, follow) | Instances | Tiles | BG color |
|---:|---|---|---:|:-:|---|---:|---:|---|
| 0 | rScreenInit | 320x240 | 30 | N | - | 2 | 0 | #000000 |
| 1 | rIntro | 960x240 | 30 | Y | 320x240 @0,0 / 640x480, oPDummy3 | 429 | 0 | #C0C0C0 |
| 2 | rCredits1 | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 82 | 0 | #C0C0C0 |
| 3 | rCredits2 | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 82 | 0 | #C0C0C0 |
| 4 | rTitle | 640x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 379 | 1 | #C0C0C0 |
| 5 | rHighscores | 320x256 | 30 | Y | 320x240 @0,0 / 640x480, - | 314 | 4 | #C0C0C0 |
| 6 | rSun | 320x256 | 30 | Y | 320x240 @0,0 / 640x480, - | 200 | 1 | #C0C0C0 |
| 7 | rMoon | 320x256 | 30 | Y | 320x240 @0,0 / 640x480, - | 176 | 1 | #C0C0C0 |
| 8 | rStars | 320x256 | 30 | Y | 320x240 @0,0 / 640x480, - | 172 | 1 | #C0C0C0 |
| 9 | rTutorial | 672x304 | 30 | Y | 320x240 @0,0 / 640x480, oPlayer1 | 404 | 1 | #C0C0C0 |
| 10 | rLevelEditor | 672x544 | 30 | Y | 320x240 @0,0 / 640x480, - | 150 | 0 | #C0C0C0 |
| 11 | rLoadLevel | 672x544 | 30 | Y | 320x240 @0,0 / 640x480, - | 240 | 0 | #C0C0C0 |
| 12 | rLevel | 672x544 | 30 | Y | 320x240 @0,0 / 640x480, oPlayer1 | 19 | 0 | #C0C0C0 |
| 13 | rLevel2 | 672x608 | 30 | Y | 320x240 @0,0 / 640x480, oPlayer1 | 103 | 0 | #C0C0C0 |
| 14 | rLevel3 | 672x672 | 30 | Y | 320x240 @0,0 / 640x480, oPlayer1 | 23 | 0 | #C0C0C0 |
| 15 | rOlmec | 672x880 | 30 | Y | 320x240 @0,0 / 640x480, oPlayer1 | 387 | 0 | #C0C0C0 |
| 16 | rTransition1 | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 303 | 0 | #C0C0C0 |
| 17 | rTransition1x | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 305 | 0 | #C0C0C0 |
| 18 | rTransition2 | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 304 | 0 | #C0C0C0 |
| 19 | rTransition2x | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 304 | 0 | #C0C0C0 |
| 20 | rTransition3 | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 303 | 0 | #C0C0C0 |
| 21 | rTransition3x | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 303 | 0 | #C0C0C0 |
| 22 | rTransition4 | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 303 | 0 | #C0C0C0 |
| 23 | rEnd | 640x256 | 30 | Y | 320x240 @0,0 / 640x480, - | 361 | 0 | #C0C0C0 |
| 24 | rEnd2 | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 2 | 0 | #C0C0C0 |
| 25 | rEnd3 | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 200 | 0 | #C0C0C0 |
| 26 | rEndCustom | 320x240 | 30 | Y | 320x240 @0,0 / 640x480, - | 475 | 0 | #C0C0C0 |
| 27 | rKeyConfig | 160x120 | 30 | Y | 160x120 @0,0 / 320x240, - | 1 | 0 | #000000 |
| 28 | rJoyConfig | 160x120 | 30 | Y | 160x120 @0,0 / 320x240, - | 1 | 0 | #000000 |
| 29 | room0 | 320x240 | 30 | N | - | 3 | 0 | #C0C0C0 |

## 5. The GameMaker runtime subset the game uses

Method: every script, object code action and DnD expression argument was tokenized with a GML tokenizer that
skips `//` and `/* */` comments and `"..."`/`'...'` strings (GM 8 strings have no escapes).

- **Functions:** an identifier followed by `(` that isn't a game script and doesn't follow `.`.
- **Variables and constants:** classified against JoshEdit/LateralGM's GML keyword lists, with GM 8 extras added.
- **Cross-check:** every identifier that isn't a keyword, built-in, constant, resource or script must be
  assigned somewhere in game code (or by a DnD "set variable" action). Only 4 identifiers fail this check, and
  all are genuine dangling references (see Surprises): `rInit` (2), `sndStep1` (1), `sndStep2` (1), `oInit` (1). So no built-in variable was
  silently counted as a game variable.

### 5.1 Built-in functions: **112 distinct, 5789 call sites**

Every function name was checked against the reference function list. The only name missing from that list
(which is GMS-era) is `show_info`, a real GM 8 function used by the supersound demo object. There are no
calls to unknown or extension functions. The 23 SuperSound `SS_*` names are **game scripts** that wrap
`external_call`, so they aren't counted here.

| Category | Function | Calls | Code units using it |
|---|---|---:|---:|
| Instances | `instance_create` | 1388 | 156 |
| Instances | `instance_destroy` | 341 | 144 |
| Instances | `instance_exists` | 140 | 53 |
| Instances | `instance_nearest` | 80 | 24 |
| Instances | `instance_place` | 41 | 19 |
| Instances | `instance_activate_object` | 21 | 6 |
| Instances | `instance_position` | 20 | 6 |
| Instances | `instance_number` | 20 | 12 |
| Instances | `instance_deactivate_object` | 6 | 3 |
| Instances | `instance_activate_region` | 5 | 2 |
| Instances | `object_get_parent` | 4 | 3 |
| Instances | `instance_activate_all` | 3 | 1 |
| Instances | `instance_deactivate_region` | 1 | 1 |
| Instances | `instance_deactivate_all` | 1 | 1 |
| Collision / movement | `collision_point` | 546 | 105 |
| Collision / movement | `collision_rectangle` | 84 | 27 |
| Collision / movement | `distance_to_object` | 58 | 26 |
| Collision / movement | `collision_line` | 16 | 16 |
| Collision / movement | `move_snap` | 13 | 7 |
| Collision / movement | `place_meeting` | 3 | 2 |
| Collision / movement | `distance_to_point` | 1 | 1 |
| Drawing | `draw_text` | 325 | 24 |
| Drawing | `draw_set_color` | 148 | 23 |
| Drawing | `draw_sprite` | 115 | 12 |
| Drawing | `draw_set_font` | 89 | 23 |
| Drawing | `draw_sprite_ext` | 26 | 13 |
| Drawing | `draw_set_alpha` | 17 | 7 |
| Drawing | `draw_circle` | 15 | 1 |
| Drawing | `draw_rectangle` | 11 | 7 |
| Drawing | `draw_clear` | 6 | 1 |
| Drawing | `draw_set_blend_mode_ext` | 4 | 1 |
| Drawing | `draw_set_blend_mode` | 4 | 1 |
| Drawing | `font_add_sprite` | 2 | 1 |
| Drawing | `draw_surface_stretched` | 2 | 1 |
| Drawing | `screen_redraw` | 1 | 1 |
| Drawing | `draw_surface` | 1 | 1 |
| Drawing | `screen_refresh` | 1 | 1 |
| Surfaces | `surface_set_target` | 12 | 1 |
| Surfaces | `surface_create` | 6 | 1 |
| Surfaces | `surface_exists` | 6 | 1 |
| Surfaces | `surface_reset_target` | 4 | 1 |
| Surfaces | `surface_free` | 3 | 1 |
| Tiles | `tile_add` | 65 | 9 |
| Tiles | `tile_layer_find` | 8 | 6 |
| Tiles | `tile_delete` | 8 | 6 |
| Rooms / views | `room_goto` | 53 | 19 |
| Rooms / views | `room_set_view` | 26 | 1 |
| Rooms / views | `room_get_name` | 6 | 6 |
| Rooms / views | `game_end` | 5 | 2 |
| Rooms / views | `room_restart` | 4 | 2 |
| Rooms / views | `room_goto_next` | 2 | 2 |
| Strings | `string` | 185 | 52 |
| Strings | `ord` | 98 | 5 |
| Strings | `string_length` | 70 | 17 |
| Strings | `real` | 69 | 19 |
| Strings | `string_delete` | 62 | 11 |
| Strings | `string_insert` | 29 | 7 |
| Strings | `is_real` | 21 | 21 |
| Strings | `string_char_at` | 19 | 10 |
| Strings | `is_string` | 4 | 4 |
| Strings | `string_upper` | 3 | 3 |
| Strings | `string_lower` | 3 | 2 |
| Math / random | `abs` | 239 | 51 |
| Math / random | `round` | 78 | 19 |
| Math / random | `ceil` | 73 | 28 |
| Math / random | `floor` | 64 | 26 |
| Math / random | `degtorad` | 42 | 11 |
| Math / random | `point_distance` | 39 | 16 |
| Math / random | `random` | 35 | 13 |
| Math / random | `sin` | 23 | 13 |
| Math / random | `cos` | 21 | 11 |
| Math / random | `point_direction` | 12 | 11 |
| Math / random | `radtodeg` | 8 | 2 |
| Math / random | `arctan` | 8 | 2 |
| Math / random | `frac` | 4 | 2 |
| Math / random | `make_color_rgb` | 4 | 2 |
| Math / random | `sqr` | 2 | 1 |
| Math / random | `sqrt` | 1 | 1 |
| Keyboard / mouse / joystick | `joystick_check_button` | 37 | 2 |
| Keyboard / mouse / joystick | `joystick_zpos` | 36 | 2 |
| Keyboard / mouse / joystick | `keyboard_check_pressed` | 29 | 21 |
| Keyboard / mouse / joystick | `keyboard_check` | 17 | 8 |
| Keyboard / mouse / joystick | `joystick_has_pov` | 12 | 1 |
| Keyboard / mouse / joystick | `joystick_pov` | 12 | 1 |
| Keyboard / mouse / joystick | `joystick_direction` | 12 | 1 |
| Keyboard / mouse / joystick | `joystick_exists` | 5 | 3 |
| Keyboard / mouse / joystick | `keyboard_check_released` | 4 | 4 |
| Keyboard / mouse / joystick | `mouse_check_button` | 4 | 3 |
| File I/O | `file_text_write_string` | 158 | 5 |
| File I/O | `file_text_readln` | 107 | 5 |
| File I/O | `file_text_writeln` | 84 | 5 |
| File I/O | `file_text_read_string` | 83 | 5 |
| File I/O | `file_text_close` | 13 | 9 |
| File I/O | `file_text_open_read` | 9 | 5 |
| File I/O | `file_text_eof` | 6 | 2 |
| File I/O | `file_text_open_write` | 5 | 5 |
| File I/O | `file_delete` | 4 | 2 |
| File I/O | `file_exists` | 2 | 1 |
| File I/O | `file_find_next` | 2 | 1 |
| File I/O | `file_find_first` | 1 | 1 |
| Highscore table | `highscore_value` | 44 | 5 |
| Highscore table | `highscore_add` | 29 | 2 |
| Highscore table | `highscore_clear` | 2 | 2 |
| DLL | `external_call` | 27 | 23 |
| DLL | `external_define` | 22 | 1 |
| Sound (GM built-in) | `sound_play` | 4 | 1 |
| Window / dialogs | `window_set_cursor` | 17 | 4 |
| Window / dialogs | `show_message` | 12 | 1 |
| Window / dialogs | `get_integer` | 4 | 1 |
| Window / dialogs | `show_error` | 1 | 1 |
| Window / dialogs | `show_menu` | 1 | 1 |
| Window / dialogs | `show_info` | 1 | 1 |

### 5.2 Drag-and-drop actions (must be emulated too)

All are library 1 (main). `action_inherited` is the only way the game calls `event_inherited`, which never
appears in GML. Set-variable actions are `name = value` (all 15 set `image_speed` to 0.3, 0.4, 0.8 or 1 in create events),
and none are relative. Two `action_if` actions in `oScreen` create choose the fullscreen mode. The third, in
`oGamepad` step, gates the event on `global.gamepadOn`, which affects the actions that follow it in the event.

| lib/id | DnD action | Kind | Count | Example locations |
|---|---|---|---:|---|
| 1/604 | action_inherited | normal | 284 | oSolid:create/0, oMovingSolid:create/0, oMoveableSolid:create/0, oWater:create/0 ... |
| 1/203 | action_kill_object | normal | 31 | oExplosion:other/7, oBloodTrail:other/7, oFlameTrail:other/7, oSmallCollect:alarm/0 ... |
| 1/611 | - | variable | 15 | oBlood:create/0, oExplosion:create/0, oBloodTrail:create/0, oFlame:create/0 ... |
| 1/601 | action_execute_script | normal | 7 | oGame:step/0, oPlayer1:create/0, oPlayer1:draw/0, core:create/0 ... |
| 1/605 | - | normal | 6 | oGame:create/0, oPlayer1:step/0, oPlayer1:step/0, oPlayer1:step/0 ... |
| 1/408 | action_if | normal | 3 | oScreen:create/0, oScreen:create/0, oGamepad:step/0 |
| 1/524 | action_color | normal | 2 | oGame:draw/0, oGame:draw/0 |
| 1/531 | action_fullscreen | normal | 2 | oScreen:create/0, oScreen:create/0 |

### 5.3 Built-in variables: **46 distinct**

Array-style built-ins (`alarm[]`, `view_*[]`, `background_index[]`) are counted once by base name. `argument0`
to `argument7` are used, and the plain `argument[]` array is not used.

| Variable | References |
|---|---:|
| `x` | 2839 |
| `y` | 2733 |
| `sprite_index` | 1168 |
| `argument0` | 350 |
| `argument1` | 273 |
| `alarm` | 223 |
| `view_yview` | 209 |
| `view_xview` | 203 |
| `argument2` | 190 |
| `image_speed` | 148 |
| `image_index` | 63 |
| `view_hview` | 60 |
| `keyboard_key` | 59 |
| `view_wview` | 53 |
| `depth` | 46 |
| `visible` | 36 |
| `sprite_width` | 35 |
| `sprite_height` | 33 |
| `direction` | 28 |
| `image_xscale` | 26 |
| `image_angle` | 22 |
| `image_alpha` | 20 |
| `image_yscale` | 19 |
| `room_width` | 19 |
| `image_blend` | 18 |
| `room_height` | 18 |
| `mouse_x` | 16 |
| `mouse_y` | 14 |
| `id` | 10 |
| `room` | 8 |
| `object_index` | 6 |
| `speed` | 6 |
| `view_vborder` | 5 |
| `argument4` | 4 |
| `argument5` | 4 |
| `argument3` | 2 |
| `sprite_xoffset` | 2 |
| `sprite_yoffset` | 2 |
| `room_speed` | 2 |
| `view_enabled` | 2 |
| `view_hborder` | 2 |
| `view_object` | 2 |
| `argument6` | 1 |
| `argument7` | 1 |
| `background_index` | 1 |
| `mouse_button` | 1 |

Not used at all: `hspeed`, `vspeed`, `gravity`, `friction`, `xprevious`, `yprevious`, `xstart`, `ystart`,
`path_*`, `timeline_*`, `score`, `lives`, `health`, `instance_count`, `fps`, `current_time`, `bbox_*`,
`image_number`, `mask_index`, `solid` (as a variable), `persistent` (as a variable).

### 5.4 Built-in constants: **27 distinct**

`c_white` (84), `c_yellow` (55), `ty_string` (47), `dll_stdcall` (22), `vk_enter` (21), `vk_shift` (13), `cr_default` (13), `vk_escape` (12), `c_black` (12), `vk_backspace` (11), `vk_delete` (11), `cr_none` (4), `bm_normal` (4), `vk_up` (3), `vk_down` (3), `vk_left` (3), `vk_right` (3), `vk_space` (3), `bm_one` (3), `mb_left` (3), `vk_control` (2), `vk_alt` (2), `bm_src_color` (2), `bm_zero` (2), `mb_right` (2), `c_red` (2), `bm_dest_color` (1)

Resource names used as values in code: objects 3,605 times, sprites 1,259, rooms 79, backgrounds 66.

### 5.5 Language features

Keyword counts: `if` 6198, `global` 3345, `else` 2914, `and` 1686, `false` 941, `not` 938, `true` 881, `break` 849, `case` 833, `other` 701, `or` 559, `with` 371, `return` 325, `for` 138, `switch` 107, `repeat` 39, `then` 35, `while` 28, `noone` 23, `begin` 9, `end` 9, `mod` 5, `default` 5, `globalvar` 3, `var` 2, `self` 2, `exit` 1.

Other language features in use:

- 315 distinct `global.*` variables; `globalvar` appears 3 times.
- 2-D arrays (`a[i,j]`): 121 index expressions.
- `mod` 5 times; `div`, `%`, bitwise operators, `<>` and `:=` are all unused.
- `begin`/`end` blocks: 9.
- `exit` once.
- 1,388 `instance_create` calls, 214 alarm assignments, 147 `image_speed` assignments and 45 runtime `depth`
  assignments.

## 6. Randomness

- **Functions:** the only RNG function is `random(x)`, with 35 direct call sites. The game uses no `irandom`,
  `random_range`, `choose`, `random_set_seed`, `random_get_seed` or `randomize` anywhere (checked on tokens,
  outside comments and strings).
- **Wrapper:** nearly everything goes through `scripts/rand.gml`, which has **1,363 call sites**:

  ```
  randNum = argument1 - argument0 + 1;
  return floor(random(randNum)) + argument0;
  ```

  So `rand(a,b)` returns an inclusive integer in [a,b]. Note that `randNum` isn't declared `var`, so every call
  writes an instance variable `randNum` on the calling instance (or on the `with` target). Nothing else reads
  `randNum`.
- **Where it is used:** level generation (`scrLevelGen`, `scrRoomGen*`, `scrEntityGen`, `scrTreasureGen`,
  `scrGenerateItem`, `scrShopItemsGen`) and gameplay or cosmetics (`oPlayer1` step has 45 calls, block destroy
  events have 26-33 each, plus blood, flames, debris, `scrUseItem` and the shopkeeper). The 35 direct `random()`
  calls are particle velocities and similar.
- **Seed:** **never set in game code.** Seeding is runner behaviour: the GM 8 runner randomizes at startup. Level
  generation and moment-to-moment effects share **one global RNG stream**, so a fixed seed would only reproduce
  a level if every earlier RNG call happened in exactly the same order. For seeded or replayable runs the
  port must either reproduce GM 8's generator and call order exactly, or split the streams on purpose. That
  is a design decision for a ticket. If bit-exact behaviour is wanted, check the GM 8 generator against
  OpenGMK.

## 7. External dependencies

### 7.1 supersound.dll (all music and sound effects)

- **Binding:** `scripts/SS_Init.gml` calls `external_define` 22 times on `supersound.dll`. Every binding is
  `dll_stdcall` and every argument and return value is `ty_string`, so sound handles are strings.
- **Exported symbols:**
  - `SS_Init`
  - `SS_LoadSound(file, stream)`
  - `SS_PlaySound(h)`, `SS_LoopSound(h)`, `SS_StopSound(h)`, `SS_PauseSound(h)`, `SS_ResumeSound(h)`,
    `SS_FreeSound(h)`
  - `SS_SetSoundFreq(h, f)`, `SS_SetSoundPan(h, p)`, `SS_SetSoundVol(h, v)`
  - `SS_GetSoundVol(h)`, `SS_GetSoundFreq(h)`, `SS_GetSoundPan(h)`
  - `SS_IsSoundPlaying(h)`, `SS_IsSoundLooping(h)`, `SS_IsSoundPaused(h)`
  - `SS_GetSoundPosition(h)`, `SS_SetSoundPosition(h, p)`, `SS_GetSoundLength(h)`,
    `SS_GetSoundBytesPerSecond(h)`
- **Calls:** there are 27 `external_call` sites across the 23 `SS_*` wrapper scripts. Four wrappers have 2
  calls each.
- **Units:** volume is 0..10000, with music set as `2000 + 8000*(musicVol/18)`. Pan is -10000..10000, and
  frequency is in Hz.
- **Unload bug:** the `SS_Unload` binding points at the DLL's **`SS_Init`** symbol (copy-paste bug), so
  "unload" at game end actually calls init again.
- **Init and loading:** `oMusic` (persistent, placed in `rScreenInit`) runs `SS_Init` via DnD, then loads
  every file with `SS_LoadSound("sound/....", 0|1)`. Music (`.ogg`) is streamed and effects (`.wav`) are fully
  loaded. `playSound(snd)` is just `SS_PlaySound`.
- **Files:** **62 distinct `sound/*` files are referenced and all ship in `game.zip`.** Five shipped files
  are never referenced: `bombready.wav`, `gspiderjump.wav`, `land.wav`, `letsexplore.wav`, `pause.wav`.
- **Built-in sound:** GM's own `sound_play` appears 4 times, all in the unused object `oTemp`. It refers to
  `sndArrowTrap`, which is not a resource.

### 7.2 File I/O (all `file_text_*`, relative to the working directory; no ini, registry or binary files)

| File | Read by | Written by | Format |
|---|---|---|---|
| `settings.cfg` | `scrInit` | `config.exe` (`scrSaveSettings` in config.gmk) | 7 lines: fullscreen, graphicsHigh, downToRun, gamepadOn (0/1), screenScale, musicVol, soundVol (0-17). The shipped file is `0,1,0,1,3,15,15`. |
| `keys.cfg` | `scrInit` | `oKeyConfig` room_end (and config.exe) | 12 virtual-key codes: up, down, left, right, jump, attack, item, run, bomb, rope, flare, pay |
| `gamepad.cfg` | `scrInit` | `oJoyConfig` room_end (and config.exe) | 9 joystick button numbers: jump, attack, item, run, bomb, rope, flare, pay, start |
| `stats.txt` | `scrReadStats` (and a dead `if (false ...)` block in `oGame` draw) | `scrWriteStats` | Human-readable "SPELUNKY EXTENDED STATS" report. The reader parses values with `string_delete(str,1,20)` and `real()`. |
| `levels/*.lvl` | `scrLoadLevel`, `oLevelEditor`; listed by `oLoadLevel` with `file_find_first("levels\*.lvl", 0)`/`file_find_next` | `oLevelEditor` keypress Y | See 7.3 |
| `levels/test.tmp` | `scrLoadLevel` when `global.testLevel != ""`, `oLevelEditor` | `scrTestLevel` | Same as `.lvl`; deleted by the editor |

**Highscore table as persistent storage.** `scrUpdateHighscores` and `scrResetHighscores` call
`highscore_clear` and then `highscore_add` exactly 10 entries:

- MONEY+9e6, TIME+8e6, KILLS+7e6, SAVES+6e6, PLAYS+5e6
- WINS+4e6, DEATHS+3e6, TUNNEL1+2e6, TUNNEL2+1e6
- MINIGAMES, packed as `m1*10000 + m2*100 + m3`

The large offsets keep GM's descending sort in a fixed order, and the values are read back with
`highscore_value(i)` (44 call sites). Where the runner stores this table on disk is runner behaviour and isn't
in the source. A port needs its own persistence for these 10 values, with the same overflow behaviour (for
example, money above 1e6 bleeding into the next slot's range).

### 7.3 Custom levels (`levels/*.lvl`, text)

1. 32 lines of 40 characters each. Each character is a tile code, passed to `scrCreateTileObj(ch, 16+i*16, 16+j*16)`.
2. Then `author`, `music`, `lifeStart`, `bombStart`, `ropeStart` and `nextLevel`, one per line.
3. Optional: `exitNamesNum` followed by that many lines. These are assigned in scan order to each `X` (exit)
   or `@` (entrance) tile's `leadsTo`.
4. Optional: `signNamesNum` followed by that many lines, assigned to `I` sign tiles.

Names are lower-cased before the file is opened. The editor writes the same format. The shipped `levels/`
directory is empty.

### 7.4 Other OS and runner surface

- Joystick: `joystick_exists`, `joystick_check_button`, `joystick_direction`, `joystick_pov`,
  `joystick_has_pov` and `joystick_zpos`, used in `oGamepad` and the joystick config.
- Window: `window_set_cursor` (`cr_none`/`cr_default`), DnD `action_fullscreen`, and `game_end`.
- Dialogs: `show_message`, `show_error`, `show_menu`, `get_integer` and `show_info`. These are almost all in
  the supersound demo object and the level editor.
- `config.exe` calls `execute_program(global.spelunkyEXE, ...)` to launch the game. That call is only in
  config.gmk.

## 8. Hard-to-port features

| Feature | Usage in the game source | Notes |
|---|---|---|
| `with` | 371 occurrences in 143 code units | Iteration order over instances (and over parent-matched children) is visible to gameplay: `instance_nearest`, first-match `instance_place`/`collision_*`, and destruction inside `with`. The port must copy GM 8's instance ordering. |
| `other` | 701 occurrences: 677 in collision events, 22 in step, 2 in destroy, 0 in scripts | Collision `other` plus `other` inside `with` (which means the outer instance). |
| Collision events | 123 events | Parent matching is used heavily (for example `coll:oItem`, `coll:oEnemy`). |
| Solid objects | 24 objects are `solid`, mostly treasure (`oItem`, `oGold*`, gems, `oTemp`, `oLavaSolid`, `oEndPlat`, `oDesert*`, ...) | Runner behaviour: GM 8 moves an instance back to its previous position before a collision event with a solid instance. 12 collision events involve a solid object, for example `oExplosion`, `oParachute`, `oWebBall`, `oWeb`, `oBarrier*`, `oMonkey`, `oSpringTrap` → `oItem`, and big gems → `oGhost`. Check this before relying on it. |
| Instance (de)activation | `instance_deactivate_region` (1), `instance_activate_region` (5), `instance_activate_object` (21), `instance_deactivate_object` (6), `instance_deactivate_all(true)` (1), `instance_activate_all` (3) | `oLevel` step deactivates everything more than 96 px outside the view **every step**, then reactivates a whitelist. The pause screen deactivates everything except `oGamepad`. Deactivated instances run no events and are invisible to collision checks and `with`. Faithful AI and physics depend on this culling. |
| Alarms | 159 alarm events (indices 0-11 all used), 214 `alarm[n] =` assignments | Runner behaviour: alarm countdown runs in the step order between begin step and step. |
| Animation | 147 `image_speed` assignments, 51 `animation_end` events, 63 `image_index` references | The runner advances `image_index` by `image_speed` and fires animation end. |
| Built-in motion | `speed = 10` and `direction` on the "sight" objects of `oCaveman`, `oYeti` and `oHawkman`; `direction` in `scrFireBow`, `oArrow` and `oFishBone` (there it only feeds `image_angle`) | All other movement is custom `xVel`/`yVel` (Martin Piecyk's platform engine: `moveTo`, `isCollision*`, `setCollisionBounds`). The runner still has to apply speed and direction for the sight objects. |
| `place_meeting` / `move_snap` / `collision_*` | `place_meeting` 3 (in `moveTo` and `gameStepEvent`), `move_snap` 13, `collision_point` 546, `collision_rectangle` 84, `collision_line` 16, `instance_place` 41, `instance_position` 20 | **373 of 804 sprites use precise per-pixel masks** with separate masks per frame, and all bboxes are automatic. `collision_point` mostly passes `prec=0` (487 calls). It passes `prec=-1` 58 times, and -1 is false in GM 8
(runner behaviour: values below 0.5 are false), so those checks are not precise. `collision_line` always uses precise checks, and `collision_rectangle` does 6 times. The port needs GM 8's precise mask generation (alpha tolerance) and its bbox rules. |
| Surfaces and blend modes | 3 surfaces in `oScreen` (`pSurf` for pause, `screen` for the frame, `darkSurf` for dark-level lighting) | **`oScreen` begin_step draws the whole frame by hand**: it calls `screen_redraw()` into a surface, composites darkness with `draw_set_blend_mode_ext(bm_dest_color, bm_zero)`, uses the "alpha fix" trick (`bm_src_color, bm_one` with a black rectangle), and blits with `bm_one, bm_zero` via `draw_surface_stretched`, then calls `screen_refresh()`. It also re-creates surfaces when `surface_exists` fails. This is ChevyRay's view-scaling scripts. |
| Runtime tiles | `tile_add` 65, `tile_layer_find` 8, `tile_delete` 8 | Background decorations are added as tiles during level generation. |
| Sprite fonts | `font_add_sprite(sFont / sFontSmall, ord(' '), false, 0)` | Monospaced, not proportional. `draw_text` is called 325 times. |
| Views | 412 direct reads or writes of `view_xview`/`view_yview`, plus `room_set_view` for other rooms | Needs GM's view-follow-with-border algorithm, applied after the step. |
| Persistent objects | `oGlobals`, `oScreen`, `oMusic` and `oGamepad` (real singletons); also `oCharacter` and `oEnemy` | `oCharacter` and `oEnemy` are parent objects and are never instantiated directly. In GM 8 `persistent` is not inherited (runner behaviour), so the flag on those two has no effect. |
| Dynamic typing | `real()`/`string()` everywhere, `is_real`/`is_string`, sound handles as strings | Also `if (file)` and `if (surface)` truthiness (GM 8 treats a value as true if > 0.5), uninitialised-variable errors, and 2-D arrays. |
| Room names as strings | `isRoom("rTitle")` has 99 call sites via `room_get_name` | String comparisons on room names. |
| Not used | | `instance_change`, `event_perform`, `event_user`, `execute_string`, `execute_file`, `variable_local_*`, `variable_global_*`, `script_execute`, timelines, paths, `motion_set`/`motion_add`, `move_contact*`, `move_towards_point`, `move_bounce*`, `move_outside*`, `mp_*`, `sound_add`, ini and registry functions, `keyboard_check_direct`, `sleep`, `irandom`, `choose`. All of these were checked on tokens and none appear. |

## 9. Notable surprises

1. **No sound resources.** All 54 sound slots are deleted, and every sound goes through the SuperSound DLL with
   string handles. The `SS_Unload` binding is bound to `SS_Init`.
2. **Supersound demo left in the game:**
   - Objects `core`, `mascara` and `mascara2` (`core` loads `"anime bowling babes.ogg"`, which isn't shipped).
   - Room `room0`, which is the **last** room in the room order.
   - Two sprites in a `supersound` folder.
3. **Config-tool leftovers in the main game.** `rKeyConfig`/`rJoyConfig` with `oKeyConfig`/`oJoyConfig` call
   `room_goto(rInit)`, and `oOKButton` sets `oInit.startGame`. `rInit` and `oInit` exist only in `config.gmk`,
   so these are dangling references. Reaching that code would raise an "unknown variable" error.
4. **Dead references.** `global.sndStep1`/`sndStep2` are never assigned. They are only read in `oPlayer1`
   alarm 3, which is never set because the only setters are commented out. `oTemp` (solid) calls `sound_play`
   on a sound resource that doesn't exist, and nothing ever creates it.
5. **Region bug.** `oLevel` calls `instance_activate_region(oPlayer1.x-16, oPlayer1.y-16, oPlayer1.x+16,
   oPlayer1.y+16, true)`, passing x2/y2 where the function takes width/height. A faithful port must keep the
   oversized region.
6. **Level layout.** Rooms hold almost no tiles (9 in total), so levels are built entirely by scripts.
   `rLevel` is a 4x4 grid of `oRoom` markers.
7. **The highscore table stores stats** (see 7.2), with values packed by offset.
8. **Room speed is 30 everywhere.** `oGame` step adds `room_speed` to `global.time` and `global.xtime` every
   step. Any timing code in the port has to match how these units are used.
9. **Engine controllers come from rooms.** `rScreenInit` (room order index 0) contains only `oMusic` and
   `oScreen`. `oScreen` create runs `scrReadStats` and `scrInit` (which loads the fonts and reads the config files),
   configures every room's view, then calls `room_goto_next()`. If `surface_create` returned -1, it calls
   `game_end()` instead. `oGlobals` is placed in `rIntro`, and `oGamepad` is created with `instance_create`.
10. **Unknown mouse sub-event 12** on the demo `core` object (see Parse status).
11. **No `room_start` events at all.** Per-room setup happens in create events of placed objects such as
    `oLevel` and `oGame`, and in `room_end`.

## Appendix: config.exe (`config.gmk`)

`config.gmk` holds 43 objects, 53 scripts, 3 rooms (`rInit`, `rKeyConfig`, `rJoyConfig`), 37 sprites,
2 backgrounds and 1 font. It has 953 physical script lines and 1,486 object-code lines. It reads and writes
`settings.cfg`, `keys.cfg` and `gamepad.cfg`, and launches the game with `execute_program`.

Functions it uses that the main game does not: `execute_program`.

## Tooling

The throwaway Python tools that produced this file are in `tools/inventory-py/` (see its README). They are
superseded by the Rust `gmk` crate (851-3122) and kept only so this inventory can be regenerated.
