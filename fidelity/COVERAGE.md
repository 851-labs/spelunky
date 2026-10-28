# Coverage

What the committed replays (`fidelity/replays/`) exercised, [all together](#all-committed-replays), and what the
fuzz sweeps did: [goal-directed](#goal-directed-851-3189) runs, which get through levels, plain
[random](#random-input-851-3183) ones, and the [chaos soak](#chaos-soak-851-3299) that keeps adding both. [Deliberate interactions](#deliberate-interactions-851-3137-851-3153-851-3154)
lists which replays exercise the player's actions, items, bombs and ropes, and
[Game flow](#game-flow-851-3159) the transitions, deaths, pause, HUD and deactivation.

## All committed replays

Every replay in `fidelity/replays/` matched the oracle with `--fields` when it was added. `replay-coverage` plays
them on the port (no oracle) and reports which objects they created, by gameplay ticket, in a few minutes:

```sh
scripts/cargo run --release -p harness --bin replay-coverage                # all of fidelity/replays/
scripts/cargo run --release -p harness --bin replay-coverage -- --by-object a.txt b.txt  # which replay made what, visited which rooms, died
```

216 replays, 435,425 frames:

Objects by ticket. *Active* objects were in or near the view on some frame, so their events ran; *inactive* ones only existed deactivated (generated, but never near the player); *never* ones weren't created at all.

| Ticket | Area | Active | Inactive only | Never |
|---|---|---|---|---|
| 851-3147 | enemies | 39 of 40: oAlien oAlienBoss oAlienEject oBarrier oBarrierEmitter oBat oCaveman oCavemanWorship oDeadFish oEnemySight oFireFrog oFrog oGhost oGiantSpider oGiantSpiderHang oHawkman oHawkmanWorship oJaws oMagma oMagmaMan oMagmaTrail oManTrap oMonkey oOlmec oOlmecDebris oOlmecSlam oPiranha oScarab oSkeleton oSnake oSpider oSpiderHang oTombLord oUFO oUFOCrash oVampire oYeti oYetiKing oZombie |  | oEntranceCheck |
| 851-3152 | items | 73 of 77: oAnkh oArrow oBall oBall2 oBomb oBombBag oBombBox oBones oBow oCape oCapePickup oChain oChain2 oChest oCompass oCrate oCrown oCrystalSkull oDiamond oDice oEmerald oEmeraldBig oFakeBones oFireFrogBomb oFishBone oFlare oFlareCrate oGloves oGoldBar oGoldBars oGoldChunk oGoldIdol oGoldNugget oJar oJetpack oKapala oKey oLampItem oLampRedItem oLockedChest oMachete oMachetePre oMattock oMattockHead oMattockHit oMattockPre oMitt oParaPickup oParaUsed oParachute oPaste oPistol oRock oRope oRopePile oRopeThrow oRopeTop oRuby oRubyBig oSapphire oSapphireBig oSceptre oShotgun oSkull oSlash oSpectacles oSpikeShoes oSpringShoes oTeleporter oUdjatEye oWebCannon oWhip oWhipPre |  | oBasketball oFireFrogArmed oJordans oRopeBag |
| 851-3157 | traps | 20 of 22: oArrowRepeaterL oArrowRepeaterR oArrowTrapLeft oArrowTrapLeftLit oArrowTrapRight oArrowTrapRightLit oArrowTrapTest oBoulder oCeilingTrap oDoor oGiantTikiHead oSmashTrap oSmashTrapLit oSpearTrapBottom oSpearTrapLit oSpearTrapTop oSpearsLeft oSpikes oSpringTrap oTrapBlock |  | oTemp oThwompTrap |
| 851-3158 | effects | 40 of 41: oBigCollect oBlood oBloodSpark oBloodTrail oBone oBubble oBullet oBurn oDrip oExplosion oFlame oFlameTrail oFlareSpark oFly oHeart oItemsGet oLaser oLaserExplode oLaserTrail oLavaDrip oLeaf oPoof oPsychicCreate oPsychicCreate2 oPsychicCreateP oPsychicWave oPsychicWaveP oRopeBurn oRubble oRubbleDark oRubbleDarkSmall oRubbleSmall oShotgunBlastLeft oShotgunBlastRight oSmokePuff oVolcanoFlame oWeb oWebBall oYellowBall oYellowTrail |  | oSmallCollect |
| 851-3146 | terrain | 54 of 58: oAlienShip oAltarLeft oAltarRight oBlackFadeUp oBlock oBrick oBrickSmooth oCaveBG2 oCaveBGEntrance oDark oDarkFall oEntrance oExit oFrozenCaveman oGoldDoor oGrave oHardBlock oIce oIceBlock oIceBottom oKaliHead oLadderOrange oLadderTop oLamp oLampRed oLava oLavaSolid oLeaves oLeavesTile oLush oMoai oMoai2 oMoai3 oMoaiInside oMsgSign oPushBlock oSacAltarLeft oSacAltarRight oSign oSplash oTemple oTempleFake oThinIce oTikiTorch oTree oTreeBranch oTreeBranchTile oTreeTile oVine oVineTop oWaterSwim oXGold oXMarket oXocBlock |  | oAlienShipFloor oCaveBG oGold oGoldBig |
| 851-3151 | characters | 7 of 8: oDamsel oDamselKiss oPlayer1 oShopkeeper oShopkeeper2 oTunnelMan oYellHelp |  | oCharacter |
| 851-3159 | game flow | 47 of 77: oBlack oBlackBG oCredits1 oCredits2 oCursObj oEnd oEnd2 oEnd3 oEndCustom oFinalBoss oGame oGamepad oGlobals oHighscores oHintHand oIntro oLevel oLevelEditor oLoadLevel oMenuBottom oMenuLL oMenuLR oMenuLeft oMenuRight oMenuSel oMenuTop oMenuUL oMenuUR oMoonRoom oMusic oPDummy oPDummy2 oPDummy3 oPDummy4 oPDummy5 oPDummy6 oPageDown oPageUp oRoom oScreen oSolid oSprite oStarsRoom oSunRoom oTitle oTransition oWalDamsel |  | oBackdrop oBricks oCaveUp oCheckBox oCheckBox2 oCheckBox3 oCheckBox4 oDetritus oDrawnSprite oEnemy oForeground oFrill oItem oJoyConfig oJoyConfigButton oKeyConfig oKeyConfigButton oLadder oMenu oMoveableSolid oMovingSolid oOKButton oPlatform oRadioBox oRubblePiece oSlider1 oSlider2 oSliderBG oTreasure oWater |

Why the terrain (851-3146) and effects (851-3158) objects that no replay creates are missing:

- Never created by 1.1, so unreachable: `oGold` and `oGoldBig` (only `oLaser` destroys them), `oAlienShipFloor`,
  `oCaveBG` (`scrInitLevel`'s `instance_create` is commented out) and `oSmallCollect` (`oPlayer1`'s is commented
  out). In generated levels `oVineTop` only comes from a `P` tile, which no room or obstacle string has.
- `oTreeTile`, `oLeavesTile` and `oTreeBranchTile` are the level editor's own tiles (F2 on the title,
  `rLevelEditor`, `scrCreateTile`), and `oVineTop` comes from its `t` tile; `editor-every-tile` builds every tile
  the editor offers, in the editor and then in play (851-3135). A custom level (`scrLoadLevel`) builds the
  ordinary `oTree`/`oVine...` instead.
- `oTikiTorch` (dark Jungle levels, 1 in 20 per spot, `scrEntityGen`): the goal-directed sweep's
  `goal-jungle-s66-f1`, cut at frame 1700, is committed as `fuzz-goal-jungle-seed-66-tiki-torch` (851-3291); one
  is active near the player from 1592.
- `oRopeBurn`, which `oRope`'s step makes for a rope segment with lava at `x+12` (`collision_point(x+12, y, oLava,
  0, 0)`): a segment's `x` is its column's left edge, so that's the lava of its own column. `terrain-rope-burn-lava`
  (851-3377) drops a rope into a Temple lava pool, and the burn climbs it to the player and drops him (943).
- `oBloodSpark` (blood soaked up with the kapala) is replayed by `item-seed-11-kapala` (851-3236).
- `oLavaDrip` (lava a bomb lets flow, `oLava`'s destroy) lost its replay when 851-3322 re-made
  `city-of-gold-seed-9`; `terrain-bomb-lava` (851-3375) throws a bomb into a Temple lava pool, whose blast lets
  eleven `oLava` flow away beside the player.

Why the rarer items (851-3152) that no replay creates are missing, with their sources in
[quirks](../docs/quirks.md) §1 (851-3228, 851-3234, 851-3236).

- Unreachable: `oJordans`, `oRopeBag`, `oBasketball` and `oFireFrogArmed` (nothing creates them and no room places
  them).
- The crown is created in every Ice Caves Moai, sealed; `item-seed-9-ankh.txt` revives with the ankh, but at the
  Black Market's entrance.
- Replayed (851-3236): `oKapala` (favor 16 from a damsel and two stunned yetis on two levels' altars, then the
  blood of spiders Kali sends when a UFO's laser defiles the altar, filling it into a heart, `item-seed-11-kapala`).
- Replayed (851-3234): `oBall`/`oChain` (the second defiled altar) and `oBall2`/`oChain2` (the transition after it,
  `item-seed-38-ball-and-chain`), `oDiamond` (a ghost through a big sapphire in a block, `item-seed-2-diamond`),
  `oLampItem` and `oLampRedItem` (a bomb lit in the hand and thrown up at the lamp's block, `item-seed-38-lamp`,
  `item-seed-22-red-lamp`).

Where the last rarities reachable in normal play come from, and what replays them (851-3253):

- `oSmashTrapLit`: `scrEntityGen` makes a smash trap lit on a dark level, and a dark Temple level needs the level
  before it to take more than 20 seconds past the minute (quirks §1). In `trap-dark-smash` the player waits at level 13's exit, and
  one of dark level 14's six slides at him in the dark and crushes him (1836).
- The Lady Xoc statue's gem blocks: `oXocBlock`'s destroy drops two of a block's `treasure` in the City of Gold
  (`scrRoomGen4`'s `X` tile puts a diamond, two sapphires, two emeralds and a ruby in its 30 blocks).
  `city-of-gold-seed-9-to-olmec` blows the diamond's, the right sapphire's and the right emerald's;
  `city-of-gold-seed-9-xoc-gems` blows the other three, the left sapphire's, the ruby's and the left emerald's.
- `oShopkeeper2` is created only by `oStarsRoom`'s alarm 0, the shotgun shopkeepers that drop into the Stars bonus
  room every 100-200 frames while the player lives; `bonus-stars-shotgun` (851-3162) already has them drop in,
  shoot, and one killed.
- `rTransition2x` and `rTransition3x` were already visited by `city-of-gold-seed-9`, with no Tunnel Man; the
  replays with him in them are in [Game flow](#game-flow-851-3159).

The rest of the *never* column can't be reached in 1.1, or isn't an object the game instantiates (851-3367). The
replays visit every room the game can reach: all but `rKeyConfig`, `rJoyConfig` and the SuperSound demo's
`room0` (`replay-coverage --by-object` lists them, except the boot room `rScreenInit`, which every run passes
through). So whatever a reachable room places is created, and seen unless it destroys itself in the same step:

- `oBricks` is: 15 rooms place one (`rTitle`, the transition rooms, the bonus rooms, ...), and its create event
  runs `scrSetupWalls(224)` and destroys it, so `replay-coverage`, which looks between steps, never sees it.

None of the others is placed in a reachable room or created by any script or event:

- Parents, never instantiated themselves (their children count under their own names): `oCharacter`
  (`oPlayer1`'s), `oDrawnSprite`, `oDetritus`, `oEnemy`, `oItem`, `oTreasure`, `oLadder`, `oPlatform`, `oWater`,
  `oRubblePiece`, `oMenu`, `oMoveableSolid` and `oMovingSolid`.
- The settings program's leftovers ([quirks](../docs/quirks.md) §1, `crates/game/tests/config_screens.rs`):
  `oRadioBox`, `oCheckBox`-`oCheckBox4`, `oOKButton`, `oKeyConfigButton`, `oJoyConfigButton`, `oSlider1`,
  `oSlider2` and `oSliderBG` are in no room, and `oKeyConfig` and `oJoyConfig` only in `rKeyConfig` and
  `rJoyConfig`, which only those buttons go to. The settings program's own copies are replayed in
  `fidelity/settings/`.
- Unused: `oEntranceCheck`, `oBackdrop`, `oForeground`, `oCaveUp`, `oFrill`, and `oTemp`, whose
  `sound_play` names a sound that doesn't exist ([source inventory](../docs/source-inventory.md) §9).
- `oThwompTrap`: only `scrRoomGen5`'s `t` tile makes one, and none of the final level's room or obstacle strings
  has a `t`.

Kali's punishments: her first, six spiders from `oKaliHead` (`weapon-seed-38-web-cannon`, `item-seed-11-kapala`),
hurts the player in `item-seed-11-kapala` (1839 and 1894), and her second is the ball and chain
(`item-seed-38-ball-and-chain`). Her third, `oSacAltarLeft`'s destroy once `global.kaliPunish` is 2 or more, makes six
more spiders if the level is already dark with a ghost, and otherwise makes it dark and sends an `oGhost` from the
view's edge, unless one exists already: `item-seed-38-kali-third-punishment` (851-3377) defiles a third altar, on
level 5, which goes dark and gets a ghost from the view's left edge (3420). Its destroy also destroys every other
altar, so the six more spiders need a later level that is dark with a ghost: `item-seed-38-kali-six-more-spiders`
(851-3380) goes on to a level 6 that rolled dark with an altar, waits there until the level's clock sends the ghost
(8968), then defiles a fourth altar (8980), whose destroy has `oKaliHead` release six more spiders (8981).
A sacrifice to an angry Kali, with the favor below 0 but above -8: `item-seed-11-kali-angry-sacrifice`
(851-3549) defiles level 9's altar at a favor of 12 (-4, 1455) and puts a dead yeti on level 10's (1968), whose
`global.favor < 0` branch adds its 2 as the other branch would (-2, quirks §1).

## Level editor and custom levels (851-3135, 851-3245)

| Area | Replays |
|---|---|
| Editor tiles | placing with the left button and dragging, the wheel and A/S changing the tile, 1-4 changing the kind, right-click erase (`editor-build-test-save`, `mouse-level-editor`); every tile it offers (`editor-every-tile`) |
| Editor info and files | the info prompt's doubled Enter skipping the author, lives and ropes, and the name, music, bombs and next level edited (`editor-build-test-save`); a saved level opened with its exit and sign names, which E edits (`editor-doors-signs-new`); saving with Escape and Y and with New and Y, leaving with Escape and N, each `.lvl` compared byte for byte (`editor-build-test-save`, `editor-doors-signs-new`, `editor-sign-no-exit`) |
| Test | `levels/test.tmp` written and played, back to the editor by the exit (`editor-build-test-save`, `editor-test-hud-caps`) and by dying (`editor-doors-signs-new`); a test with 1111 bombs typed in, which `scrTestLevel` doesn't clamp, and 35 rope piles and a bomb box dragged in, taking bombs and ropes past `oPlayer1`'s 99 cap (`editor-test-hud-caps`, 851-3293) |
| Load menu | twelve levels on two pages, page down and up, rows chosen with the mouse, the Load button and Enter (`mouse-load-level`, `custom-chain-end`) |
| Custom levels | a chain through three levels by next-level and named exits, the entrance named after the level left, signs, lives, bombs and ropes kept between levels, and `rEndCustom` to `rHighscores` (`custom-chain-end`); a level saved without exits losing its sign message (`editor-sign-no-exit`) |
| Live input | a session recorded in the native app with held keys and the mouse (`tools/pid-input`): a level named, built, saved, then loaded from the menu and played to its exit (`editor-live-native`, 851-3254) |

The shipped `levels/` in the 1.1 zip is empty, so there are no shipped levels to replay.

## Deliberate interactions (851-3137, 851-3153, 851-3154)

Every replay below is identical to the oracle with `--fields`. The existing replays were checked on the port, frame by
frame, for what happens (which instance dies or breaks next to an explosion, the whip or a thrown item, what the
player holds, what hurts him); the frame is roughly when.

| Interaction | Replays |
|---|---|
| Bomb kills | the shopkeeper (`shop-seed-11-bomb`, 1383; `shop-seed-3-bomb-rope-piles`, 1291; out of his shop, hunting a murderer, `item-seed-38-kali-six-more-spiders`, 5337), the damsel (`damsel-seed-6-death`, ~928), a stunned caveman (`action-carry-bomb-caveman`, ~1133); enemies that weren't stunned: a skeleton and a snake (`action-skull-bomb-rope-arrow`, 1000 and 1553), a yeti a sticky bomb rode on (`item-seed-53-paste-yeti`, ~1297) and the tomb lord (`city-of-gold-seed-2-sceptre`, ~1057); a magma man split into two blobs with full lives (`enemy-magma-man-bomb-full-life`, 510; quirks §1); a jar (`ice-seed-2`, ~1218) and a fire frog's bomb (`jungle-seed-10`, ~1542) blown up; terrain (`movement-tutorial-exit`, ~1523), and the rock sealing a gold door's pocket, two floors bombed through from above (`city-of-gold-seed-9-sealed-door`, 6882 and 7091), the rock sealing a locked chest and its key, a shaft's foot and its wall (`item-seed-35-udjat-eye-sealed-key`, 2053 and 2221), and a Temple lava pool's wall, which lets the pool's lava flow away (`terrain-bomb-lava`, 975-983); Kali's altar (`weapon-seed-38-web-cannon`, ~1504; `item-seed-38-ball-and-chain`, 2359; `item-seed-38-kali-third-punishment`, 3420, the third; `item-seed-38-kali-six-more-spiders`, 8980, the fourth, not by the blast but by its own step once the blast took its floor) |
| Chain explosions | a bomb setting off another (`action-bomb-chain`, 1523 and 1534) |
| Bomb pushback | the damsel (`damsel-seed-6-death`), a caveman (`fuzz-room-change-ends-event`, ~1038), a bomb bag, the shotgun, the shopkeeper and the player's body (`shop-seed-11-bomb`, ~1383), two rope piles, a parachute, the shopkeeper, his shotgun and the player's body (`shop-seed-3-bomb-rope-piles`, 1290-1292), two parachutes, the player's body and the shopkeeper (`shop-seed-3-free-rope-pile`, 1296-1297), a bomb (`action-bomb-chain`) |
| Bombs placed and thrown | dropped (duck + A: `movement-tutorial-exit`, `shop-seed-11-bomb`, `shop-seed-3-bomb-rope-piles`, `shop-seed-3-free-rope-pile`), a lit bomb picked up and put down, a bomb placed while holding one, a bomb thrown (`action-bomb-chain`), a bomb taken out with the item key, lit in the hand and thrown up into a ceiling (`item-seed-38-lamp`, ~1268-1393; `item-seed-22-red-lamp`, ~1358-1483), a bomb thrown down a shaft and one placed against its wall, opening a pocket of rock that seals a locked chest's key (`item-seed-35-udjat-eye-sealed-key`, 1934-2221); a chest's trap bomb, not armed, carried lit out of the exit and back into the bombs (`item-chest-trap-bomb-unarmed`, 284-296; quirks §1); four bombs dug down the mothership's right edge without touching the Alien Boss or the barrier's emitter, the last blowing the Jetpack out of its rock and through the barrier from the boss's side (`ice-seed-63-barrier-through`, 1420-2094; quirks §1) |
| Whip | a caveman (`temple-seed-1`, ~982), a megamouth, a hit point on each step a swing touches it (`enemy-megamouth-whip-every-step`, 318-324; quirks §1), the Yeti King, hurt only by the wind-up behind the player (`enemy-yeti-king-whip-wind-up`, 372-414; quirks §1), a tomb lord, hurt from its own floor by the wind-up and not the crack (`enemy-tomb-lord-whip-wind-up`, 245-304; quirks §1), a magma man, whose life only the wind-up shortens (`enemy-magma-man-whip-wind-up`, 464-506; quirks §1), a giant spider, hurt by the wind-up inside its wait (`enemy-giant-spider-whip-wind-up`, 246-248; quirks §1), a man trap (`jungle-seed-1`, 2946), zombies (`enemy-skeleton-seed-3`, ~1059; `jungle-seed-10`, 3338), jars (`enemy-mines-seed-1-kills`, ~837; `shop-seed-12-dice`, ~988), a skull (`enemy-skeleton-seed-3`, ~1061), the damsel (`damsel-seed-6-death`), whose hop counts as a throw, so she lies stunned for what her cry-for-help countdown had left (`damsel-whip-hop-thrown`, 282-443; quirks §1), the shopkeeper (`shop-seed-11-whip`) |
| Mattock, Bow and Teleporter | a mattock's wind-up made twice, taking 12 of the Yeti King's hit points in one swing, and its hit breaking the brick under the player (`weapon-mattock-wind-up-twice`, 322-331), and the brick beyond the one ahead, then the one under him (`weapon-mattock-wrong-block`, 331 and 371); a brick's rubble falling where it stood and a dark block's off-centre (`terrain-dark-rubble-off-centre`, 297 and 356); the bomb key at part draw leaving a plain arrow, which falls at 0.6 a step after its first, and at full draw readying a bomb arrow that costs no bomb, which sticks in a wall and blows it up (`weapon-bow-bomb-arrow-free`, 277-446); the teleporter used inside a shotgun's wait, taking the player back to its last spot (`weapon-teleporter-old-target`, 405); the teleporter used up twice in a row, the second time from near the view's top, drawn level with the player for a frame (`weapon-teleporter-held-y`, 297); all quirks §1 |
| Thrown items hitting enemies | a rock killing a snake (`action-throw-rock-snake`, 1102), a jar and a skull killing a snake and breaking (`action-throw-jar-snake`, 730; `action-skull-bomb-rope-arrow`, 792); trap arrows killing snakes (`mines-seed-7`, ~948; `shop-seed-22-kissing-theft`, ~1134) and hitting a caveman (`temple-seed-1`, ~877); a rope's head killing a bat (`movement-tutorial-exit`, ~1020; `ending-seed-1`, 5398); a ware, a bomb bag, thrown at the shopkeeper (`shop-seed-11-throw`, 1303); a rock destroying a UFO, which drops an `oAlienEject` (`enemy-ufo-alien-seed-14`, 1957); a stunned yeti a UFO's laser knocks up, falling back onto the UFO and destroying it, untallied (`enemy-ufo-stunned-yeti-seed-14`, 1896; quirks §1); a rock touching a hanging spider and killing the bat beside it, the enemy nearest the rock (`action-throw-rock-nearest-enemy`, 309; quirks §1); a rock stunning a caveman and pushing it at 0.3 of its speed, and the stunned caveman thrown into a yeti, which takes all of its speed (`action-throw-monster-full-push`, 273 and 440; quirks §1) |
| Items breaking | thrown jars (`shop-seed-11-whip`, ~723; `ice-seed-2`, ~1604; `jungle-seed-1`, ~886) and skulls (`enemy-skeleton-seed-3`, ~860), one from a skeleton the whip breaks (`shop-seed-12-dice`, ~1916), a jar the whip breaks, rolling twice for its contents (`item-jar-whip-rolls-twice`, 266; quirks §1), a jar that bounces off a ceiling whole and a skull that breaks on it, more slowly (`item-skull-breaks-on-ceiling`, 282 and 359; quirks §1), a jar a shopkeeper's shot breaks in the player's hands, which the item key gives back after a bomb and a rope (`item-held-jar-shot`, 318-353; quirks §1) |
| Items in water, lava and on spikes | a rock and a jar in water (`jungle-seed-9-market`, ~1616), a crate (`jungle-seed-1`, ~1394), a skull (`enemy-skeleton-seed-3`, ~840); a jar in lava (`temple-seed-43`, ~1825), arrows (`temple-seed-1`, ~1574), a tomb lord that walks into lava and the sceptre it drops there, which sinks and never burns up (`temple-seed-113`, 786), a bomb that lies in lava until it goes off (`terrain-bomb-lava`, ~866-975); an arrow on spikes (`bonus-moon-archery`, ~958) |
| Ropes | thrown up into a ceiling, climbed and jumped off the top (`movement-tutorial-exit`, ~1015-1129), as the held item, thrown down a ledge (`movement-tutorial-exit`, ~1180) and put down (`movement-items-rope`, ~1395); none thrown under a ceiling, and dropped with duck + S (`action-rope-tunnel`, ~1140-1162); duck + S throwing none, against a wall and on flat floor, whose rectangles beside the rope's column both reach into the floor (`action-skull-bomb-rope-arrow`, 1188 and 1626); dropped with duck + S into a lava pool, which burns it from below, up to the player climbing it, dropping him (`terrain-rope-burn-lava`, 856-943) |
| Carrying | a stunned caveman picked up, thrown, picked up again and thrown up (`action-carry-bomb-caveman`, ~897-992; `fuzz-room-change-ends-event`, ~1137-1181), the damsel (`damsel-seed-6-*`, `bonus-sun-damsel`), a rock (`movement-items-rope`), a dead caveman's body carried and thrown (`action-carry-dead-caveman`, 1248-1300), a key carried crawling to its locked chest (`item-seed-35-udjat-eye-sealed-key`, 2308-2349), a stunned caveman lifted from beside the stunned yeti the player crouches over, the enemy nearest him (`action-pickup-nearest-enemy`, 445; quirks §1), a rock thrown with the pitcher's mitt caught on the rebound, every rock held with it and all three thrown together (`item-mitt-catch-every-rock`, 335-371; quirks §1), a rock and a jar thrown level with the mitt, the rock's light gravity lasting a step and the jar's for good (`item-mitt-gravity-one-step`, 301-373; quirks §1); the ankh thrown up out of its shop, lying outside at its price with the shopkeeper calm until it's picked up there (`item-seed-9-ankh-thrown-out`, 1459-1585; quirks §1); the crystal skull carried to a shopkeeper and sold, counted as a golden idol (`item-seed-213-crystal-skull-sold`, 1022-1844; quirks §1) |
| Parachutes | opened in a long fall (`item-seed-26-parachute`), crossed by a rock bounced back left off a wall, which it survives, and torn away by one bounced back right (`item-parachute-torn-moving-right`, 332 and 492; quirks §1) |
| Jetpack | thrust 9 px above the floor at the end of a long drop, which sets the fall's count back to 0, so the landing doesn't hurt (`item-jetpack-thrust-breaks-fall`, 341-353) |
| Arrow traps | a left-facing trap's sensor 6 blocks wide and a right-facing one's 5: the player midway between two that face each other across 11 open blocks sets off only the left-facing one (`trap-arrow-sensor-left-wider`, 263 and 304; quirks §1); generation never makes a right-facing trap with only two open blocks in front (`crates/harness/tests/level_gen.rs`; quirks §1) |
| Webs | a jump out of the later of two webs side by side wearing down the other: eleven of fourteen jumps go to the web beside him, which breaks, and three to his own (`trap-web-jump-wrong-web`, 371-475; quirks §1) |
| Sticky bombs | the paste stolen, a bomb thrown that sticks to a wall (`item-seed-53-paste`, ~985-1105), and one that sticks to a yeti, rides on him and kills him (`item-seed-53-paste-yeti`, 1186-1297) |
| Chests | opened into gems (`equip-seed-13-chest-gems`, 664) and pressed twice more, each press counted (`item-seed-13-chest-tally`, 674 and 684; quirks §1); opened into a trap bomb (`item-chest-trap-bomb-unarmed`, 263) |
| Paying | `shop-seed-3-buy-theft`, `shop-seed-12-dice`, `tunnelman-seed-2-donate`; a bomb bag in a Bomb shop on level 2 (`shop-seed-17-bomb-buy`, 1950: "YOU GOT 3 MORE BOMBS!", $2500), and a bag and a bomb box in one on level 4, priced 20% up (4940, $3000; 4954: "YOU GOT 12 MORE BOMBS!", $12000); wares' prices past level 2 for those without a shop name ("A SPECTACLES FOR $10400.", `item-seed-77-spectacles`, 926; "A SPIKE SHOES FOR $5200.", `item-seed-7-spike-shoes`, 1394; quirks §1), and a compass for sale to a player who has one (`shop-seed-11-compass-restocked`, 2028: "A COMPASS FOR $5400."; quirks §1); bets in a Dice House, a 9 winning twice the bet (`shop-seed-12-dice`, 2222) and a 7 its prize, freed from its case and replaced (`shop-seed-12-dice-seven-cheat`, 2437); a kiss bought in a Kissing Parlor on level 5 (`shop-seed-213-kiss-whip`, 2059: "NOW AIN'T SHE SWEET!", $25000 and a heart) |
| Freed wares collected on touch | a rope pile, once a bomb makes the shopkeeper angry (`shop-seed-3-free-rope-pile`, 1178: "YOU GOT 3 MORE ROPES!"); a bomb box in a Weapon shop, once its shopkeeper is shot dead (`weapon-seed-38-web-cannon`, 1119: "YOU GOT 12 MORE BOMBS!") |
| Thieves | the level after a theft: a shopkeeper waiting by the exit, who turns on the player as he comes near, and a shop's shopkeeper walking about rather than greeting him, its wares freed and counted as stolen as they come on screen (`shop-seed-3-thief-next-level`, 1657-1946); a bomb in a Black Market shop, which adds to `thiefLevel` again on every step for each shopkeeper active off screen (`shop-seed-9-market-bomb`, 1187: 14, then 9 more a step); a whipped shopkeeper freeing the wares, the player no thief (`shop-seed-11-whip`, 1298), and a shot one, whose freed bomb bag the player collects (`weapon-seed-39-pistol`, 1598); a thief luring the shopkeeper into a hole with spikes in it, whose death there counts as the player's kill and makes him a murderer (`shop-seed-35-spikes-murderer`, 1928); on his next level, a Dice House's prize, priced though not for sale, counted as stolen as it comes on screen (`shop-seed-35-murderer-dice-prize`, 2899); a die from the Black Market's Dice House carried through the next shop, angering no one there, then out of it, angering that shop's shopkeeper rather than the Dice House's (`shop-seed-9-market-dice-carried-out`, 1383); the Kissing Parlor's damsel, carried out with the player first, counted bought, not stolen (`shop-seed-22-kissing-theft`, 1309); all quirks §1. And a die thrown again before its bet is settled (`shop-seed-12-dice-seven-cheat`, 2650: "COME BACK HERE, THIEF!") and the Kissing Parlor's damsel whipped while she kisses (`shop-seed-213-kiss-whip`, 2069: "HEY, ONLY I CAN DO THAT!") |
| Damage | fall damage and stuns (`movement-ladder-crawl`, `movement-*`, ~889), a bat (`movement-bat-death`), a snake (`shop-seed-3-buy-theft`, 984), a spider (`enemy-mines-seed-1-kills`, ~1584), a caveman (`title-shortcut5`, ~892), a yeti (`ice-seed-2`, ~1583) and a yeti's throw (`enemy-ufo-alien-seed-14`, 2364), a zombie (`jungle-seed-10`, 3492), a vampire in bat form, healed by the blood of its bites (`enemy-vampire-seed-163-death`, 830-920), a Dead Fish (`enemy-dead-fish-seed-89`, 1152), piranhas and a megamouth (`enemy-megamouth-seed-1-death`, 1310-1400), the Yeti King's touch, 2 hearts in a Yeti lair (`enemy-yeti-king-swat-seed-47`, 1799) and nothing in a custom level (`enemy-yeti-king-custom-level-touch`, 305 and 345; quirks §1), a piranha biting a yeti (`enemy-yeti-piranha-charge`, 348), a tomb lord's flies knocking a caveman back in full (`enemy-tomb-lord-fly-knockback`, 272 and 279; quirks §1), a magma man's touch (`enemy-magma-man-bomb-full-life`, 472; `enemy-magma-man-whip-wind-up`, 472 and 513), a blob of magma's (`enemy-magma-blob-seed-19-death`, 1095 and 1125), a UFO (`fuzz-room-change-ends-event`, ~1052), a lit arrow trap (`trap-dark-arrow`, 2164), a shotgun (`bonus-stars-shotgun`), an arrow trap's arrow (`item-seed-38-ball-and-chain`, 2395), Kali's spiders (`item-seed-11-kapala`, 1839 and 1894) |
| Deaths | a bat (`movement-bat-death`), a megamouth, untallied (`enemy-megamouth-seed-1-death`, 1400; quirks §1), a zombie, a vampire and a Dead Fish on graveyard levels, untallied (`enemy-zombie-seed-177-death`, 960; `enemy-vampire-seed-163-death`, 920; `enemy-dead-fish-seed-89-death`, 1249; quirks §1), a blob of magma on a Temple level, untallied (`enemy-magma-blob-seed-19-death`, 1125; quirks §1), the player's own bomb on a real level, untallied (`action-bomb-death-mines-seed-1`, 721; quirks §1), an arrow trap's arrow (`action-skull-bomb-rope-arrow`, 1708), the boulder, tallied as crushed, never in the boulder row (`trap-boulder-crush`, 813; quirks §1), the Temple idol's ceiling trap (`trap-ceiling-crush`, 1362), the lava at the foot of the Sacrifice Pit, once its trap blocks drop the player (`trap-sacrifice-pit`, 936), a smash trap, whose hit throws the player toward it (`trap-smash-crush`, 1747; quirks §1), one he brushes before it has ever slid, which throws him down by its random `dir` (`trap-smash-random-throw`, 269; quirks §1), and a lit one in the dark, which only crushes (`trap-dark-smash`, 1836; quirks §1), spikes (`fuzz-room-change-ends-event`, `enemy-yeti-king-seed-14`), an explosion (`bonus-sun-damsel`, ~1222), shotguns (`shop-seed-11-whip`, `shop-seed-11-bomb`, `shop-seed-11-throw`, `shop-seed-3-bomb-rope-piles`, `shop-seed-3-free-rope-pile`, `shop-seed-3-buy-theft`, `shop-seed-22-kissing-theft`, `shop-seed-9-market-bomb`), the teleporter (`weapon-seed-40-teleporter`), the ghost (`enemy-ghost-seed-2`), Olmec (`olmec-death-seed-1`), a UFO's laser (`enemy-ufo-alien-seed-14`, 2523; `fuzz-goal-ice-seed-63-alien-boss`, 2138, on level 11), a spider Kali released (`item-seed-38-kali-six-more-spiders`, 9009); crushed by a pushed block, in `rHighscores`, where it hides the player without being a death (`flow-highscores-reset-crush`, 845); crushed in Olmec's lair by the block `oFinalBoss` puts under the exit door when Olmec dies, tallied as Olmec's (`olmec-door-block-crush-seed-1`, 5431; quirks §1); eaten by a mantrap in a custom level, the jetpack lying behind him destroying itself on the next step (`item-jetpack-player-invisible`, 298-299; quirks §1); a damsel thrown onto spikes, counted in `damselsKilledTotal` but not in `damselsKilled` or `kills` (`damsel-spikes-not-a-kill`, 307; quirks §1) |
| Stomps | a caveman stomped in spike shoes (`item-seed-7-spike-stomp`, 1521), a yeti stomped on the way up, stunned with all its hit points (`enemy-yeti-stomp-rising`, 311; quirks §1), and a yeti and a caveman each stomped on the player's 16th step of falling, the yeti losing 1 hit point and the caveman 2 (`enemy-yeti-stomp-sixteen-steps`, 285 and 353; quirks §1) |
| Walking and sight | a caveman's sight check setting off a hawkman walled in under the player, while the caveman itself is too far from him (`enemy-caveman-sight-alerts-hawkman`, 293; quirks §1), a caveman sliding off the ledge it stops at, beside a hawkman that stops dead at its own (`enemy-caveman-ledge-slide`, 287; quirks §1), a skeleton from a custom level's tile `K` walking right, away from the player (`enemy-skeleton-tile-faces-right`, 260; quirks §1), a yeti's sight check setting off a caveman walled in under the player, while the yeti never charges (`enemy-yeti-sight-never-charges`, 293; quirks §1), and a yeti a piranha's bite sets charging, the only way one does (`enemy-yeti-piranha-charge`, 348; quirks §1), Olmec drifting left over the player without stopping to slam, and drifting right stopping at once (`ending-seed-1`, 2924-2932 and 3041; quirks §1), Olmec at the left wall sitting in the lava without drowning, until a hop drifts him right (`ending-seed-1`, 4906-4980; `olmec-door-block-crush-seed-1`, 4906-5244, through two hops with the player on a rope over him; quirks §1), a patrolling shopkeeper's walk ending after one step, ten times (`enemy-shopkeeper-patrol-one-step`, 240-531; quirks §1) |

The jar kills that `area-route --steps '+oJar,/oSnake'` first found on mines seeds 3, 9 and 13, and `'+oJar,/oFrog'`
on jungle seed 15 (851-3291), were stomps that the search's throw test let through: it measured the player's distance
from 8 px past the target's middle, so a stomp, 16 px above it, passed for a throw, and the jar outlived its target each
time. The test now plays the frame on which the target is hurt again with the thrown item stopped dead, and only
counts the throw if the target is then unhurt (`route::thrown_hit`, 851-3296). The same four searches then find real
jar hits, the jar breaking on its target (frames 730, 831, 844 and 970), and the first is `action-throw-jar-snake`.
The skull and rock kills above pass the new test too.

## Game flow (851-3159)

Every replay below is identical to the oracle with `--fields`; the `flow-*` ones were added for this ticket.
`replay-coverage --by-object` lists the rooms each replay visits and its deaths as well as its objects.

| Behavior | Replays |
|---|---|
| Area transitions | `rTransition1` (mines, 40 replays: `mines-seed-*`, `shop-*`, ...), `rTransition1x` with the Tunnel Man (`tunnelman-seed-2-donate`), `rTransition2` (`jungle-*`, `flow-lake-jungle-seed-1`), `rTransition3` (`ice-*`, `fuzz-goal-ice-*`), `rTransition4` (`temple-*`, `ending-seed-1`); `rTransition2x` and `rTransition3x`, where `oPlayer1`'s exit sends the player out of levels 8 and 12 (`global.currLevel` 9 and 13), with every shortcut paid (`city-of-gold-seed-9*`, entered on 4179 and 5576) and with the Tunnel Man: a donation towards the level 9 shortcut (`tunnelman-seed-9-jungle-donate`, $4100 given on 4286), it paid off, which makes `global.tunnel1` `tunnel3Max + 1` (`tunnelman-seed-9-jungle-paid`, paid on 4286), and the level 13 shortcut paid off (`tunnelman-seed-9-ice-paid`, paid on 5683; 851-3253, re-made by 851-3326); and each one with its tunnel above the Tunnel Man's maximum, where `oTransition`'s create only takes 1 off it and creates no Tunnel Man: a fresh table's 100001 in `rTransition1x` (`tunnelman-seed-2-mines-over-max`, entered on 4231), a fresh table's 200001 in `rTransition2x` once the first shortcut is paid (`tunnelman-seed-9-jungle-over-max`, entered on 4179), and `tunnel3Max + 1` in `rTransition3x`, in the game that paid off the level 9 shortcut (`tunnelman-seed-9-jungle-paid-ice-over-max`, entered on 5682; 851-3293); the tutorial's exit to `rTransition1`, whose `scrClearGlobals` starts a real game (`movement-tutorial-exit`) |
| Rooms | `rLevel2` (`ice-*`; its mothership levels, `oAlienShip` and the alien boss, in `fuzz-goal-ice-*` and `ice-seed-63-barrier-through`), the lake `rLevel3` with `oJaws` (`flow-lake-jungle-seed-1`), `rOlmec` and the ending `rEnd`-`rEnd3`, `rCredits2` (`ending-seed-1`, `city-of-gold-seed-9-to-olmec`), `rCredits1` (`title-locked`), the bonus rooms (`bonus-*`) |
| Level generation | the path walk's `else`s, moving toward a room the path has taken: the other way (`mines-seed-1`, level 1; `city-of-gold-seed-9`, level 5) and down, beside the sacrificial pit (`temple-seed-43`, level 14); a level 2 with no shop, its side rooms beside the path all having it on their left (`mines-seed-5`, 1468); after the Black Market, no shop for the rest of the game, no lake roll for the Jungle's levels (`city-of-gold-seed-9`, levels 7 and 8; `jungle-seed-9-market-dark-roll`, 2419 and 3112), and the market's dark roll of 1 using up the area's dark level (`jungle-seed-9-market-dark-roll`, 1673); a Black Market with two General Stores and no Rare Shop, its fifth shop made General after finding the kind left (`shop-seed-9-market-two-general-stores`, 1171); the Ice Caves' row of blocks under the room, with treasure on it (`ice-seed-2`, 760); a Temple level's stone blocks breaking without rubble (`city-of-gold-seed-9`, 5763) and the City of Gold's without gold (`city-of-gold-seed-9-sacrifice-pit`, 7845), and the City's odd earth blocks (`city-of-gold-seed-9`, 6745); a City of Gold that's a sacrifice pit, showing its message twice (`city-of-gold-seed-9-sacrifice-pit`, 7280-7698), and one whose Lady Xoc statue took the pit's third room, so the idol's fall lands on the statue (`city-of-gold-seed-9-xoc-pit`, 7568-7591); a Black Market on a graveyard level, with graves and Zombies and no graveyard message (`jungle-seed-9-market-graveyard`, 1145), and a dark Temple level after the Hedjet, level 12 having taken more than 20 "seconds" past the minute (`city-of-gold-seed-9-hedjet-dark`, 6122); all quirks §1. And a snake pit down the leftmost column, its bottom room's two big rubies taken and its Mattock left buried in its brick (`mines-seed-2-snake-pit`, 3641-4015). A Jungle level that's dark, a graveyard and flooded at once, whose third message is the lake's as "YOU HEAR RUSHING WATER..." (`jungle-seed-10-dark-graveyard-lake`, 4211-4639); quirks §1; a Yeti lair whose only box side room is in the bottom row (`ice-seed-13`, 2030, quirks §1) |
| Keys in a transition room | jump, bomb, rope, item, Escape, F1, F10, run and the directions do nothing there: no `oPlayer1` exists (`flow-transition-keys`) |
| Deaths | 45 replays, 46 deaths (`replay-coverage --by-object` lists them), among them the Deaths row above and the pause screen's F1 and bomb key (`flow-pause-game-over-new-game`, each counted in its level's row only as attack leaves the game over screen, 1000 and 1360; quirks §1) |
| Game over → `rHighscores` | `bonus-*`, `fuzz-room-change-ends-event`, `olmec-death-seed-1`; then the title and a new game, so `scrClearGlobals` is checked (`flow-pause-game-over-new-game`) |
| `rHighscores`' reset button | the block pushed onto `oButtonHighscore`: the click, the trophies and the four bonus doors destroyed at once (809), then `scrResetHighscores`, shake 60 and the thump 18 frames later (827: the counter starts at 20 and counts down to 1), and the player crushed walking into the stopped block (845, quirks §1), which outside a level hides him but isn't a death (`dead` stays false) (`flow-highscores-reset-crush`, 851-3276) |
| `rHighscores`' trophies and panel | one win at 11:40: the silver trophy, no `oXMoon`, and the Moon Room's goal on the panel all the same, climbing the ledges beside the trophies (`flow-highscores-moon-room-slow-win`, 725, quirks §1); one at 16:00 and no trophy (`flow-highscores-time-960-no-trophy`, quirks §1); 8 saves and the Changing Room's door, which makes the player the Damsel (`title-damsel-rope`, 671) |
| The title rope | climbed to the top as the Damsel: `oPDummy5` climbs out as the Spelunker (954, quirks §1), then `rCredits1` (`title-damsel-rope`) |
| The flare key | pressed in `rTutorial` with the keys the settings program wrote: nothing happens, and the oracle's trace is the same as without it (`config-keys-tutorial`, 1070, quirks §1) |
| Restart | a death in the tutorial, a game over like any other, which X takes to `rHighscores` (`movement-bat-death`); the tutorial's title door back to `rTitle` (titleStart 3, `scrClearGlobals`) and a new game (`flow-tutorial-title`) |
| Pause | in a level, mid-fall, with keys pressed while paused, then F1 and the bomb key killing (`flow-pause-game-over-new-game`); in the tutorial and on the title (`flow-tutorial-title`); in `rHighscores` and quitting with F10 (`flow-pause-game-over-new-game`); on the step after the Megamouth's last bite, before `oPlayer1`'s step can kill him, and quitting with F10, the death counted nowhere (`flow-pause-dying-quit-seed-1`, 1401-1430; quirks §1); on the title and quitting with the rope key (`ending-seed-1`, `city-of-gold-seed-9-to-olmec`). While paused only `oScreen` and `oGamepad` are active; the trace compares the rest as deactivated |
| Timers across a pause | `global.time`, `global.xtime`, the "chill" message and the ghost, 700 frames late after two pauses (`flow-pause-ghost-timer`); the ghost unpaused (`enemy-ghost-seed-2`); the dark-level clock (`trap-dark-*`) |
| HUD (`scrDrawHUD`, every level frame, its locals compared) | life changes (every damage replay above), life clamped to 0 after F1's -99 (`flow-pause-game-over-new-game`), the held item for each `pickupItemType` the item replays hold, the compass arrows (`item-seed-3-compass`), and the right arrow drawn above the screen in `rOlmec`, the view lower than its exit (`item-seed-1-compass-olmec-lair`, from 4783; quirks §1), the sticky bomb icon (`item-seed-53-paste`), the equipment row (`item-*`, `equip-*`), the arrows beside the bow (`bonus-moon-archery`), `global.collect` (every money pickup); money in the tens of thousands ($66,825, `city-of-gold-seed-9-xoc-gems`, 8017; $58,275 in `rOlmec`, `city-of-gold-seed-9-to-olmec`, ~11563); bombs and ropes past `oPlayer1`'s 99 cap: the editor's test starts with 1111 bombs (drawn over the rope icon on its first frame), capped to 99 by the next step, then rope piles make 100 and 102 and a bomb box 111, each capped again (`editor-test-hud-caps`, 548-636, 851-3293); messages counted down twice a step on a lit level, by `oScreen` and `oLevel`'s draw, and once on a dark one (`mines-seed-2`, 1730-1930 and 3651-3751; quirks §1) |
| `oLevel` deactivation | every level frame of every replay compares the active and deactivated sets. Instances that moved out of the 96 px window, froze and came back: bones and a skull (`enemy-mines-seed-1-kills`, 1201-1342), a piranha and a bubble (`jungle-seed-10`, 1938-2192); `oEnemySight` flickering at the window's edge (`jungle-seed-10`). A rock thrown out of the view on purpose and followed back: it stops in mid-air once past the view's 16 px margin (`oItem`'s step moves an item only within 16 px of the view), `oLevel` deactivates it once the player walks away and reactivates it when he comes back, and it flies on, bounces off the wall and is picked up again (`item-seed-49-rock-off-view`, 978-1263, 851-3293) |
| The `instance_activate_region` bug | objects right of and below the player stay active outside the window, inside the oversized region: a bat for 63 frames (`action-throw-rock-snake`, from 870), gems and gold (`movement-bat-death`, `enemy-mines-seed-1-kills`); the Black Market's hidden entrance, switched back on by it with the view still about 160 px short, which lets oGame set the Udjat Eye's blink timer again (`item-seed-35-udjat-eye-market`, 5549) |

Not covered yet: dying in a transition room is impossible (no `oPlayer1`; `oPDummy*` can't be hurt), and so is
whipping or bombing there. Life past the 99 cap isn't practical (851-3293). Only the editor's test sets a start value
without clamping it (`scrTestLevel` 93-95, then `scrLoadLevel(-1)`, which skips the clamp: `if (argument0 != -1)`, 138), and its info
prompt never stops on the lives or the ropes: each Enter runs the any-key event twice and moves two fields on
(quirks §1), so only the bombs can be typed above 99. A custom level's start values are floored and clamped to 99
(`scrLoadLevel` 140-148; quirks §1). In play, life comes 1 at a time (the damsel's kiss, `oDamselKiss`, one however many were saved: two on a custom level give one, `damsel-two-saved-one-heart`, 711, quirks §1; the kapala's ninth
blood, `oPlayer1`'s `collision_oBlood` 7-10; a bought kiss, `oPlayer1`'s step 1443-1444, at
`10000 + 5000 * (global.currLevel - 2)` dollars each, `getKissValue`) or 4 to 8 at a time from Kali
(`scrGetFavorMsg` 29-33 and 47-49), each gift needing 16 more favor than the last. Ropes reach the cap from rope
piles instead (`editor-test-hud-caps`).

## Chaos soak (851-3299)

The nightly chaos soak (`soak.yml`, [docs/ci.md](../docs/ci.md#the-chaos-soak)) plays 80 new seeds a night with
`fuzz-sweep --fields` against the oracle, from seed 1451 on, and files what differs as GitHub issues. Before it,
one big run played seeds 101-1450, three in four goal-directed and the rest random input, as the soak does: 881
runs (786 goal-directed, 95 random), 3,280,236 frames, **every one identical to the oracle** with `--fields`.

| Seeds | Where | Port | Runs (directed + random) | Frames | Time |
| --- | --- | --- | --- | --- | --- |
| 101-300, all | 10 `ubuntu-24.04` machines, 4 runs at a time each | 1fe154e | 200 (150 + 50) | 744,390 | 49 minutes |
| 301-1450, part | 20 machines, until it was cancelled for taking all of the organization's | 1fe154e | 609 (601 + 8) | 2,267,780 | 63 minutes |
| 301-1450, part | this Mac, 5 runs at a time, until stopped for the load | f801e9e | 72 (35 + 37) | 268,066 | 81 minutes |

A run took 5-6 minutes on average (up to 14), with the oracle's trace and the port's compared on every frame. The
469 seeds of 301-1450 that no run reached are no likelier to diverge than any other; the nightly runs go on from
1451 instead. Any of the runs plays again with the command `fuzz-sweep` puts in its `results.jsonl`, such as:

```sh
scripts/cargo run --release -p harness --bin fuzz-sweep -- --directed --starts ice --seeds 739 --fuzz 1 \
  --frames 3000 --fields
```

## Goal-directed (851-3189)

`fuzz-sweep --directed` alternates bursts of random play (60-240 frames) with routes that `area-route`'s
search (`harness::route`, 600 states per goal) finds toward the level's exit, or sometimes a shop's ware (then
pay), the damsel (then out the exit with her), the dice or a shopkeeper. It skips transitions with the attack
key. All 100 runs, game seeds 1-100, are identical to the oracle with `--fields` as of 58a90b2 (851-3277): every
frame, every instance's variables and the globals. The comparison took 113 minutes with 4 jobs, at load
averages of 8-79 on 10 cores:

```sh
scripts/cargo run --release -p harness --bin fuzz-sweep -- --directed --seeds 1-100 --fields --jobs 4
```

Two of the runs were committed, with fingerprints, for what no other replay covered: `goal-ice-s55-f1` as
`fuzz-goal-ice-seed-55-ufo-aliens` (an `oAlienEject` from a destroyed UFO, landing as an active `oAlien`) and
`goal-ice-s63-f1` as `fuzz-goal-ice-seed-63-alien-boss` (the alien boss's `oPsychicCreate` and `oPsychicWave`).
GM8's (de)activation order (851-3311) changed how the seed 55 run plays, and it no longer reached the aliens, so
`enemy-ufo-alien-seed-14` (a route made on purpose, 851-3323) replaced it.
Before this, 851-3189 (at 7f08271) compared 97 runs (all seeds but 95, 98 and 100) without `--fields`, and all
97 matched frame for frame.

The coverage below is from the same 100 runs (851-3260), played on the port alone:

```sh
scripts/cargo run --release -p harness --bin fuzz-sweep -- --directed --seeds 1-100 --port-only --jobs 4
```

That's 100 runs of 3,000 frames after the prelude each (372,550 frames), starting in turn in the first mines
level and levels 5, 9 and 13. The search's budget counts states, not seconds, so the replays and this table don't
depend on how busy the machine is. Two sweeps at load averages of 7-94 on 10 cores wrote the same 100 replays,
byte for byte. Only the time changes: from 18 s to 6 minutes a run, and 38-43 minutes in all. Both the port and
the route search have changed since 851-3189, so some of its replays differ now. Over the same 97 seeds, one run
that ended in level 2 now reaches level 3, and there are 122 deaths, not 124.

### Coverage

Deepest level per run (level: runs): 2: 13, 3: 4, 4: 8, 5: 10, 6: 6, 7: 3, 8: 6, 9: 6, 10: 10, 11: 3, 12: 4, 13: 20, 14: 4, 15: 3  
Rooms (room: runs): rHighscores: 85, rIntro: 100, rLevel: 100, rLevel2: 25, rLevel3: 4, rTitle: 100, rTransition1: 74, rTransition2: 15, rTransition3: 19, rTransition3x: 2, rTransition4: 7  
Deaths: 127  
Held by the player: oBomb, oBombBag, oBow, oDamsel, oGloves, oHawkman, oJar, oParaPickup, oPaste, oRock, oRopeThrow, oSpikeShoes, oSpringShoes, oYeti

Objects by ticket. *Active* objects were in or near the view on some frame, so their events ran; *inactive* ones only existed deactivated (generated, but never near the player); *never* ones weren't created at all.

| Ticket | Area | Active | Inactive only | Never |
|---|---|---|---|---|
| 851-3147 | enemies | 32 of 40: oAlien oAlienBoss oAlienEject oBarrier oBarrierEmitter oBat oCaveman oDeadFish oEnemySight oFireFrog oFrog oGiantSpider oGiantSpiderHang oHawkman oJaws oMagma oMagmaMan oMagmaTrail oManTrap oMonkey oPiranha oScarab oSkeleton oSnake oSpider oSpiderHang oTombLord oUFO oUFOCrash oYeti oYetiKing oZombie | oVampire | oCavemanWorship oEntranceCheck oGhost oHawkmanWorship oOlmec oOlmecDebris oOlmecSlam |
| 851-3152 | items | 52 of 77: oArrow oBomb oBombBag oBombBox oBones oBow oCapePickup oChest oCompass oCrate oCrown oCrystalSkull oDice oEmeraldBig oFakeBones oFireFrogBomb oFishBone oFlare oFlareCrate oGloves oGoldBar oGoldBars oGoldChunk oGoldIdol oGoldNugget oJar oJetpack oKey oLockedChest oMachete oMattock oMitt oParaPickup oPaste oPistol oRock oRope oRopePile oRopeThrow oRopeTop oRubyBig oSapphireBig oSceptre oShotgun oSkull oSpectacles oSpikeShoes oSpringShoes oTeleporter oWebCannon oWhip oWhipPre |  | oAnkh oBall oBall2 oBasketball oCape oChain oChain2 oDiamond oEmerald oFireFrogArmed oJordans oKapala oLampItem oLampRedItem oMachetePre oMattockHead oMattockHit oMattockPre oParaUsed oParachute oRopeBag oRuby oSapphire oSlash oUdjatEye |
| 851-3157 | traps | 16 of 22: oArrowTrapLeft oArrowTrapLeftLit oArrowTrapRight oArrowTrapRightLit oArrowTrapTest oCeilingTrap oDoor oGiantTikiHead oSmashTrap oSpearTrapBottom oSpearTrapLit oSpearTrapTop oSpearsLeft oSpikes oSpringTrap oTrapBlock |  | oArrowRepeaterL oArrowRepeaterR oBoulder oSmashTrapLit oTemp oThwompTrap |
| 851-3158 | effects | 29 of 41: oBlood oBloodTrail oBone oBubble oBullet oBurn oDrip oExplosion oFlame oFlameTrail oFlareSpark oFly oItemsGet oLaser oLaserExplode oLaserTrail oLeaf oPoof oPsychicCreate oPsychicWave oRubble oRubbleDark oRubbleDarkSmall oRubbleSmall oShotgunBlastLeft oShotgunBlastRight oSmokePuff oWeb oWebBall |  | oBigCollect oBloodSpark oHeart oLavaDrip oPsychicCreate2 oPsychicCreateP oPsychicWaveP oRopeBurn oSmallCollect oVolcanoFlame oYellowBall oYellowTrail |
| 851-3146 | terrain | 46 of 58: oAlienShip oAltarLeft oAltarRight oBlackFadeUp oBlock oBrick oBrickSmooth oCaveBG2 oCaveBGEntrance oDark oDarkFall oEntrance oExit oFrozenCaveman oGoldDoor oGrave oHardBlock oIce oIceBlock oIceBottom oKaliHead oLadderOrange oLadderTop oLamp oLampRed oLava oLeaves oLush oMoai oMoai2 oMoai3 oMoaiInside oPushBlock oSacAltarLeft oSacAltarRight oSign oSplash oTemple oTempleFake oThinIce oTikiTorch oTree oTreeBranch oVine oWaterSwim oXMarket |  | oAlienShipFloor oCaveBG oGold oGoldBig oLavaSolid oLeavesTile oMsgSign oTreeBranchTile oTreeTile oVineTop oXGold oXocBlock |
| 851-3151 | characters | 6 of 8: oDamsel oDamselKiss oPlayer1 oShopkeeper oTunnelMan oYellHelp |  | oCharacter oShopkeeper2 |
| 851-3159 | game flow | 26 of 77: oBlack oBlackBG oGame oGamepad oGlobals oHighscores oIntro oLevel oMenuBottom oMenuLL oMenuLR oMenuLeft oMenuRight oMenuTop oMenuUL oMenuUR oMusic oPDummy oPDummy3 oPDummy4 oRoom oScreen oSolid oSprite oTitle oTransition |  | oBackdrop oBricks oCaveUp oCheckBox oCheckBox2 oCheckBox3 oCheckBox4 oCredits1 oCredits2 oCursObj oDetritus oDrawnSprite oEnd oEnd2 oEnd3 oEndCustom oEnemy oFinalBoss oForeground oFrill oHintHand oItem oJoyConfig oJoyConfigButton oKeyConfig oKeyConfigButton oLadder oLevelEditor oLoadLevel oMenu oMenuSel oMoonRoom oMoveableSolid oMovingSolid oOKButton oPDummy2 oPDummy5 oPDummy6 oPageDown oPageUp oPlatform oRadioBox oRubblePiece oSlider1 oSlider2 oSliderBG oStarsRoom oSunRoom oTreasure oWalDamsel oWater |

Under 851-3258's rule, which doesn't count a level's first frame, before `oLevel` first deactivates what's out of
view, only one object went from active to inactive only: `oVampire`. Two runs (`goal-jungle-s34-f1` and
`goal-jungle-s58-f1`) generate a vampire, and neither gets near it. The old rule counted it because it was active
on its level's first frame. No other object changed column. Two committed replays do make a vampire active:
`jungle-seed-10.txt` and `item-seed-29-crystal-skull.txt`.

The other enemies neither sweep makes active have replays of their own (`fidelity/replays/enemy-*.txt`, 851-3147),
all identical to the oracle with `--fields`: the ghost, which comes for a player who stays 5000 frames in a level
(`enemy-ghost-seed-2.txt`); the worshipping cavemen and hawkman, which only `rOlmec` places and Olmec's alarms 1 and
6 turn into a hawkman and cavemen, and Olmec with his slam and debris (`ending-seed-1.txt`,
`olmec-death-seed-1.txt`). `oEntranceCheck` is never created: no script, event or room makes one. The Yeti King
swatting the player in its lair (`enemy-yeti-king-swat-seed-47.txt`), a skeleton rising from fake bones and stomped
(`enemy-skeleton-seed-3.txt`) and deliberate kills of the common enemies (`enemy-*-kills.txt`) go further than the
sweeps.

## Random input (851-3183)

What the random-input sweep exercised. All 100 runs match the oracle frame for frame (after #33):

```sh
scripts/cargo run --release -p harness --bin fuzz-sweep -- --seeds 1-100 --jobs 3   # verdicts; about 40 s a run
scripts/cargo run --release -p harness --bin fuzz-sweep -- --seeds 1-100 --port-only # this report, in seconds
```

That's 100 runs of 3,000 random frames each, after the prelude (372,550 frames in all), with game seeds 1-100. The starts go
in turn: the first mines level (through START), and levels 5, 9 and 13 (through the title's shortcut
doors).

### Coverage

Deepest level per run (level: runs): 1: 25, 5: 25, 9: 25, 13: 25  
Rooms (room: runs): rHighscores: 99, rIntro: 100, rLevel: 99, rLevel2: 25, rTitle: 100, rTransition1: 1, rTutorial: 1  
Deaths: 228  
Held by the player: oBomb, oCaveman, oDamsel, oJar, oRock, oRopeThrow, oSkull, oSpringShoes, oYeti

Objects by ticket. *Active* objects were in or near the view on some frame, so their events ran; *inactive* ones only existed deactivated (generated, but never near the player); *never* ones weren't created at all.

| Ticket | Area | Active | Inactive only | Never |
|---|---|---|---|---|
| 851-3147 | enemies | 23 of 40: oBat oCaveman oDeadFish oEnemySight oFireFrog oFrog oGiantSpider oGiantSpiderHang oHawkman oMagma oMagmaMan oMagmaTrail oManTrap oMonkey oPiranha oSnake oSpider oSpiderHang oTombLord oUFO oUFOCrash oYeti oZombie |  | oAlien oAlienBoss oAlienEject oBarrier oBarrierEmitter oCavemanWorship oEntranceCheck oGhost oHawkmanWorship oJaws oOlmec oOlmecDebris oOlmecSlam oScarab oSkeleton oVampire oYetiKing |
| 851-3152 | items | 47 of 77: oArrow oBomb oBombBag oBombBox oBones oBow oCapePickup oChest oCompass oCrate oCrown oDice oEmeraldBig oFakeBones oFireFrogBomb oFlare oGloves oGoldBar oGoldBars oGoldChunk oGoldIdol oGoldNugget oJar oJetpack oMachete oMattock oMitt oParaPickup oPaste oPistol oRock oRope oRopePile oRopeThrow oRopeTop oRubyBig oSapphireBig oSceptre oShotgun oSkull oSpectacles oSpikeShoes oSpringShoes oTeleporter oWebCannon oWhip oWhipPre |  | oAnkh oBall oBall2 oBasketball oCape oChain oChain2 oCrystalSkull oDiamond oEmerald oFireFrogArmed oFishBone oFlareCrate oJordans oKapala oKey oLampItem oLampRedItem oLockedChest oMachetePre oMattockHead oMattockHit oMattockPre oParaUsed oParachute oRopeBag oRuby oSapphire oSlash oUdjatEye |
| 851-3157 | traps | 13 of 22: oArrowTrapLeft oArrowTrapRight oArrowTrapTest oCeilingTrap oDoor oGiantTikiHead oSmashTrap oSpearTrapBottom oSpearTrapTop oSpearsLeft oSpikes oSpringTrap oTrapBlock |  | oArrowRepeaterL oArrowRepeaterR oArrowTrapLeftLit oArrowTrapRightLit oBoulder oSmashTrapLit oSpearTrapLit oTemp oThwompTrap |
| 851-3158 | effects | 25 of 41: oBlood oBloodTrail oBone oBubble oBullet oBurn oDrip oExplosion oFlame oFlameTrail oFlareSpark oFly oLaser oLaserExplode oLaserTrail oLavaDrip oLeaf oPoof oRubble oRubbleDark oRubbleDarkSmall oRubbleSmall oShotgunBlastRight oSmokePuff oWeb |  | oBigCollect oBloodSpark oHeart oItemsGet oPsychicCreate oPsychicCreate2 oPsychicCreateP oPsychicWave oPsychicWaveP oRopeBurn oShotgunBlastLeft oSmallCollect oVolcanoFlame oWebBall oYellowBall oYellowTrail |
| 851-3146 | terrain | 43 of 58: oAltarLeft oAltarRight oBlackFadeUp oBlock oBrick oBrickSmooth oCaveBG2 oCaveBGEntrance oDark oDarkFall oEntrance oExit oFrozenCaveman oGrave oHardBlock oIce oIceBottom oKaliHead oLadderOrange oLadderTop oLamp oLampRed oLava oLeaves oLush oMoai oMoai2 oMoai3 oMoaiInside oMsgSign oPushBlock oSacAltarLeft oSacAltarRight oSign oSplash oTemple oTempleFake oThinIce oTree oTreeBranch oVine oWaterSwim oXMarket |  | oAlienShip oAlienShipFloor oCaveBG oGold oGoldBig oGoldDoor oIceBlock oLavaSolid oLeavesTile oTikiTorch oTreeBranchTile oTreeTile oVineTop oXGold oXocBlock |
| 851-3151 | characters | 5 of 8: oDamsel oPlayer1 oShopkeeper oTunnelMan oYellHelp |  | oCharacter oDamselKiss oShopkeeper2 |
| 851-3159 | game flow | 26 of 77: oBlack oBlackBG oGame oGamepad oGlobals oHighscores oIntro oLevel oMenuBottom oMenuLL oMenuLR oMenuLeft oMenuRight oMenuTop oMenuUL oMenuUR oMusic oPDummy oPDummy3 oPDummy4 oRoom oScreen oSolid oSprite oTitle oTransition |  | oBackdrop oBricks oCaveUp oCheckBox oCheckBox2 oCheckBox3 oCheckBox4 oCredits1 oCredits2 oCursObj oDetritus oDrawnSprite oEnd oEnd2 oEnd3 oEndCustom oEnemy oFinalBoss oForeground oFrill oHintHand oItem oJoyConfig oJoyConfigButton oKeyConfig oKeyConfigButton oLadder oLevelEditor oLoadLevel oMenu oMenuSel oMoonRoom oMoveableSolid oMovingSolid oOKButton oPDummy2 oPDummy5 oPDummy6 oPageDown oPageUp oPlatform oRadioBox oRubblePiece oSlider1 oSlider2 oSliderBG oStarsRoom oSunRoom oTreasure oWalDamsel oWater |
