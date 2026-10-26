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

122 replays, 276,779 frames:

Objects by ticket. *Active* objects were in or near the view on some frame, so their events ran; *inactive* ones only existed deactivated (generated, but never near the player); *never* ones weren't created at all.

| Ticket | Area | Active | Inactive only | Never |
|---|---|---|---|---|
| 851-3147 | enemies | 39 of 40: oAlien oAlienBoss oAlienEject oBarrier oBarrierEmitter oBat oCaveman oCavemanWorship oDeadFish oEnemySight oFireFrog oFrog oGhost oGiantSpider oGiantSpiderHang oHawkman oHawkmanWorship oJaws oMagma oMagmaMan oMagmaTrail oManTrap oMonkey oOlmec oOlmecDebris oOlmecSlam oPiranha oScarab oSkeleton oSnake oSpider oSpiderHang oTombLord oUFO oUFOCrash oVampire oYeti oYetiKing oZombie |  | oEntranceCheck |
| 851-3152 | items | 73 of 77: oAnkh oArrow oBall oBall2 oBomb oBombBag oBombBox oBones oBow oCape oCapePickup oChain oChain2 oChest oCompass oCrate oCrown oCrystalSkull oDiamond oDice oEmerald oEmeraldBig oFakeBones oFireFrogBomb oFishBone oFlare oFlareCrate oGloves oGoldBar oGoldBars oGoldChunk oGoldIdol oGoldNugget oJar oJetpack oKapala oKey oLampItem oLampRedItem oLockedChest oMachete oMachetePre oMattock oMattockHead oMattockHit oMattockPre oMitt oParaPickup oParaUsed oParachute oPaste oPistol oRock oRope oRopePile oRopeThrow oRopeTop oRuby oRubyBig oSapphire oSapphireBig oSceptre oShotgun oSkull oSlash oSpectacles oSpikeShoes oSpringShoes oTeleporter oUdjatEye oWebCannon oWhip oWhipPre |  | oBasketball oFireFrogArmed oJordans oRopeBag |
| 851-3157 | traps | 20 of 22: oArrowRepeaterL oArrowRepeaterR oArrowTrapLeft oArrowTrapLeftLit oArrowTrapRight oArrowTrapRightLit oArrowTrapTest oBoulder oCeilingTrap oDoor oGiantTikiHead oSmashTrap oSmashTrapLit oSpearTrapBottom oSpearTrapLit oSpearTrapTop oSpearsLeft oSpikes oSpringTrap oTrapBlock |  | oTemp oThwompTrap |
| 851-3158 | effects | 39 of 41: oBigCollect oBlood oBloodSpark oBloodTrail oBone oBubble oBullet oBurn oDrip oExplosion oFlame oFlameTrail oFlareSpark oFly oHeart oItemsGet oLaser oLaserExplode oLaserTrail oLavaDrip oLeaf oPoof oPsychicCreate oPsychicCreate2 oPsychicCreateP oPsychicWave oPsychicWaveP oRubble oRubbleDark oRubbleDarkSmall oRubbleSmall oShotgunBlastLeft oShotgunBlastRight oSmokePuff oVolcanoFlame oWeb oWebBall oYellowBall oYellowTrail |  | oRopeBurn oSmallCollect |
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
- Not yet reached: `oRopeBurn` (a rope segment with lava 12 pixels to its right). `oBloodSpark` (blood soaked up
  with the kapala) is replayed by `item-seed-11-kapala` (851-3236).

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
  before it to take more than 20 seconds past the minute (quirks §1). `city-of-gold-seed-9`'s dark level 14 has
  one, far from the player, and `oLevel` deactivates it on the level's first step, before its step event. In `trap-dark-smash` the player waits at level 13's exit, and
  one of dark level 14's four slides at him in the dark and crushes him.
- The Lady Xoc statue's gem blocks: `oXocBlock`'s destroy drops two of a block's `treasure` in the City of Gold
  (`scrRoomGen4`'s `X` tile puts a diamond, two sapphires, two emeralds and a ruby in its 30 blocks).
  `city-of-gold-seed-9-to-olmec` blows the right emerald's; `city-of-gold-seed-9-xoc-gems` blows the other five,
  the last one with a bomb thrown from a rope, after the player baits the statue's smash trap out of its row.
- `oShopkeeper2` is created only by `oStarsRoom`'s alarm 0, the shotgun shopkeepers that drop into the Stars bonus
  room every 100-200 frames while the player lives; `bonus-stars-shotgun` (851-3162) already has them drop in,
  shoot, and one killed.
- `rTransition2x` and `rTransition3x` were already visited by `city-of-gold-seed-9`, with no Tunnel Man; the
  replays with him in them are in [Game flow](#game-flow-851-3159).

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
| Bomb kills | the shopkeeper (`shop-seed-2-bomb`, ~1986), the damsel (`damsel-seed-6-death`, ~928), a stunned caveman (`action-carry-bomb-caveman`, ~1133); enemies that weren't stunned: a skeleton and a snake (`action-skull-bomb-rope-arrow`, ~1000 and ~1128), a yeti a sticky bomb rode on (`item-seed-53-paste-yeti`, ~1297) and the tomb lord (`city-of-gold-seed-2-sceptre`, ~1057); a jar (`ice-seed-2`, ~1218) and a fire frog's bomb (`jungle-seed-10`, ~1542) blown up; terrain (`movement-tutorial-exit`, ~1523); Kali's altar (`weapon-seed-38-web-cannon`, ~1494) |
| Chain explosions | a bomb setting off another (`action-bomb-chain`, ~1523-1531) |
| Bomb pushback | the damsel (`damsel-seed-6-death`), a caveman (`fuzz-room-change-ends-event`, ~1038), a bomb bag, a rope pile, the shotgun and the shopkeeper (`shop-seed-2-bomb`, ~1983), a bomb (`action-bomb-chain`) |
| Bombs placed and thrown | dropped (duck + A: `movement-tutorial-exit`, `shop-seed-2-bomb`), a lit bomb picked up and put down, a bomb placed while holding one, a bomb thrown (`action-bomb-chain`), a bomb taken out with the item key, lit in the hand and thrown up into a ceiling (`item-seed-38-lamp`, ~1268-1393; `item-seed-22-red-lamp`, ~1358-1483) |
| Whip | a caveman (`temple-seed-1`, ~982), a man trap (`jungle-seed-1`, ~2212), zombies (`enemy-skeleton-seed-3`, ~1059; `jungle-seed-10`, ~2716), jars (`enemy-mines-seed-1-kills`, ~837; `shop-seed-12-dice`, ~988), a skull (`enemy-skeleton-seed-3`, ~1061), the damsel (`damsel-seed-6-death`), the shopkeeper (`shop-seed-11-whip`) |
| Thrown items hitting enemies | a rock killing a snake (`action-throw-rock-snake`, 1102), a jar and a skull killing a snake and breaking (`action-throw-jar-snake`, 730; `action-skull-bomb-rope-arrow`, 792); trap arrows killing snakes (`mines-seed-7`, ~948; `shop-seed-22-kissing-theft`, ~1134) and hitting a caveman (`temple-seed-1`, ~877); a rope's head killing a bat (`movement-tutorial-exit`, ~1020) and a spider (`ending-seed-1`, ~5813) |
| Items breaking | thrown jars (`shop-seed-11-whip`, ~723; `ice-seed-2`, ~1604; `jungle-seed-1`, ~886) and skulls (`enemy-skeleton-seed-3`, ~860; `shop-seed-12-dice`, ~1909) |
| Items in water, lava and on spikes | a rock and a jar in water (`jungle-seed-9-market`, ~1616), a crate (`jungle-seed-1`, ~1394), a skull (`enemy-skeleton-seed-3`, ~840); a jar in lava (`temple-seed-43`, ~1825), arrows (`temple-seed-1`, ~1574); an arrow on spikes (`bonus-moon-archery`, ~958) |
| Ropes | thrown up into a ceiling, climbed and jumped off the top (`movement-tutorial-exit`, ~1015-1129), as the held item, thrown down a ledge (`movement-tutorial-exit`, ~1180) and put down (`movement-items-rope`, ~1395); none thrown under a ceiling, and dropped with duck + S (`action-rope-tunnel`, ~1140-1162); duck + S throwing none, against a wall and on flat floor, whose rectangles beside the rope's column both reach into the floor (`action-skull-bomb-rope-arrow`, 1188 and 1428) |
| Carrying | a stunned caveman picked up, thrown, picked up again and thrown up (`action-carry-bomb-caveman`, ~897-992; `fuzz-room-change-ends-event`, ~1137-1181), the damsel (`damsel-seed-6-*`, `bonus-sun-damsel`), a rock (`movement-items-rope`), a dead caveman's body carried and thrown (`action-carry-dead-caveman`, 1248-1300) |
| Sticky bombs | the paste stolen, a bomb thrown that sticks to a wall (`item-seed-53-paste`, ~985-1105), and one that sticks to a yeti, rides on him and kills him (`item-seed-53-paste-yeti`, 1186-1297) |
| Paying | `shop-seed-3-bomb-buy-theft`, `shop-seed-12-dice`, `tunnelman-seed-2-donate` |
| Damage | fall damage and stuns (`movement-ladder-crawl`, `movement-*`, ~889), a bat (`movement-bat-death`), a snake (`shop-seed-3-bomb-buy-theft`, ~984), a spider (`enemy-mines-seed-1-kills`, ~1584), a caveman (`title-shortcut5`, ~892), a yeti (`ice-seed-2`, ~1583), a UFO (`fuzz-room-change-ends-event`, ~1052), a lit arrow trap (`trap-dark-arrow`, ~2016), a shotgun (`bonus-stars-shotgun`), Kali (`weapon-seed-38-web-cannon`) |
| Deaths | a bat (`movement-bat-death`), an arrow trap's arrow (`action-skull-bomb-rope-arrow`, 1477), the boulder (`trap-boulder-crush`), a smash trap (`trap-smash-crush`) and a lit one in the dark (`trap-dark-smash`), spikes (`fuzz-room-change-ends-event`, `enemy-yeti-king-seed-14`), an explosion (`bonus-sun-damsel`, ~1222; `shop-seed-2-bomb`), shotguns (`shop-seed-11-whip`, `shop-seed-3-bomb-buy-theft`, `shop-seed-22-kissing-theft`), the teleporter (`weapon-seed-40-teleporter`), the ghost (`enemy-ghost-seed-2`), the Yeti King (`enemy-yeti-king-seed-14`), Olmec (`olmec-death-seed-1`), a UFO's laser (`fuzz-goal-ice-seed-55-ufo-aliens`, 2501), and the ice caves' level 11 (`fuzz-goal-ice-seed-63-alien-boss`, 2138); crushed by a pushed block, in `rHighscores`, where it hides the player without being a death (`flow-highscores-reset-crush`, 845) |

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
| Area transitions | `rTransition1` (mines, 27 replays: `mines-seed-*`, `shop-*`, ...), `rTransition1x` with the Tunnel Man (`tunnelman-seed-2-donate`), `rTransition2` (`jungle-*`, `flow-lake-jungle-seed-1`), `rTransition3` (`ice-*`, `fuzz-goal-ice-*`), `rTransition4` (`temple-*`, `ending-seed-1`); `rTransition2x` and `rTransition3x`, where `oPlayer1`'s exit sends the player out of levels 8 and 12 (`global.currLevel` 9 and 13), with every shortcut paid (`city-of-gold-seed-9*`) and with the Tunnel Man: a donation towards the level 9 shortcut (`tunnelman-seed-9-jungle-donate`), it paid off, which makes `global.tunnel1` `tunnel3Max + 1` (`tunnelman-seed-9-jungle-paid`), and the level 13 shortcut paid off (`tunnelman-seed-9-ice-paid`, 851-3253); and each one with its tunnel above the Tunnel Man's maximum, where `oTransition`'s create only takes 1 off it and creates no Tunnel Man: a fresh table's 100001 in `rTransition1x` (`tunnelman-seed-2-mines-over-max`), a fresh table's 200001 in `rTransition2x` once the first shortcut is paid (`tunnelman-seed-9-jungle-over-max`), and `tunnel3Max + 1` in `rTransition3x`, in the game that paid off the level 9 shortcut (`tunnelman-seed-9-jungle-paid-ice-over-max`, 851-3293); the tutorial's exit to `rTransition1`, whose `scrClearGlobals` starts a real game (`movement-tutorial-exit`) |
| Rooms | `rLevel2` (`ice-*`; its mothership levels, `oAlienShip` and the alien boss, in `fuzz-goal-ice-*`), the lake `rLevel3` with `oJaws` (`flow-lake-jungle-seed-1`), `rOlmec` and the ending `rEnd`-`rEnd3`, `rCredits2` (`ending-seed-1`), `rCredits1` (`title-locked`), the bonus rooms (`bonus-*`) |
| Keys in a transition room | jump, bomb, rope, item, Escape, F1, F10, run and the directions do nothing there: no `oPlayer1` exists (`flow-transition-keys`) |
| Deaths | 28 replays, 29 deaths (`replay-coverage --by-object`): the Deaths row above, and the pause screen's F1 and bomb key (`flow-pause-game-over-new-game`) |
| Game over → `rHighscores` | `bonus-*`, `fuzz-room-change-ends-event`, `olmec-death-seed-1`; then the title and a new game, so `scrClearGlobals` is checked (`flow-pause-game-over-new-game`) |
| `rHighscores`' reset button | the block pushed onto `oButtonHighscore`: the click, the trophies and the four bonus doors destroyed at once (809), then `scrResetHighscores`, shake 60 and the thump 18 frames later (827: the counter starts at 20 and counts down to 1), and the player crushed walking into the stopped block (845, quirks §1), which outside a level hides him but isn't a death (`dead` stays false) (`flow-highscores-reset-crush`, 851-3276) |
| Restart | the tutorial restarted after a death (`movement-bat-death`); the tutorial's title door back to `rTitle` (titleStart 3, `scrClearGlobals`) and a new game (`flow-tutorial-title`) |
| Pause | in a level, mid-fall, with keys pressed while paused, then F1 and the bomb key killing (`flow-pause-game-over-new-game`); in the tutorial and on the title (`flow-tutorial-title`); in `rHighscores` and quitting with F10 (`flow-pause-game-over-new-game`); on the title and quitting with the rope key (`ending-seed-1`). While paused only `oScreen` and `oGamepad` are active; the trace compares the rest as deactivated |
| Timers across a pause | `global.time`, `global.xtime`, the "chill" message and the ghost, 700 frames late after two pauses (`flow-pause-ghost-timer`); the ghost unpaused (`enemy-ghost-seed-2`); the dark-level clock (`trap-dark-*`) |
| HUD (`scrDrawHUD`, every level frame, its locals compared) | life changes (every damage replay above), life clamped to 0 after F1's -99 (`flow-pause-game-over-new-game`), the held item for each `pickupItemType` the item replays hold, the compass arrows (`item-seed-3-compass`), the sticky bomb icon (`item-seed-53-paste`), the equipment row (`item-*`, `equip-*`), the arrows beside the bow (`bonus-moon-archery`), `global.collect` (every money pickup); money in the tens of thousands ($47,575, `city-of-gold-seed-9-xoc-gems`, ~7991; $48,900 in `rOlmec`, `city-of-gold-seed-9-to-olmec`, ~10497); bombs and ropes past `oPlayer1`'s 99 cap: the editor's test starts with 1111 bombs (drawn over the rope icon on its first frame), capped to 99 by the next step, then rope piles make 100 and 102 and a bomb box 111, each capped again (`editor-test-hud-caps`, 548-636, 851-3293) |
| `oLevel` deactivation | every level frame of every replay compares the active and deactivated sets. Instances that moved out of the 96 px window, froze and came back: bones and a skull (`enemy-mines-seed-1-kills`, 1201-1342), a piranha and a bubble (`jungle-seed-10`, 1938-2192); `oEnemySight` flickering at the window's edge (`jungle-seed-10`). A rock thrown out of the view on purpose and followed back: it stops in mid-air once past the view's 16 px margin (`oItem`'s step moves an item only within 16 px of the view), `oLevel` deactivates it once the player walks away and reactivates it when he comes back, and it flies on, bounces off the wall and is picked up again (`item-seed-49-rock-off-view`, 978-1263, 851-3293) |
| The `instance_activate_region` bug | objects right of and below the player stay active outside the window, inside the oversized region: a bat for 108 frames (`action-throw-rock-snake`, from ~805), gems and gold (`movement-bat-death`, `enemy-mines-seed-1-kills`) |

Not covered yet: dying in a transition room is impossible (no `oPlayer1`; `oPDummy*` can't be hurt), and so is
whipping or bombing there. Life past the 99 cap isn't practical (851-3293). Only the editor's test sets a start value
without clamping it (`scrTestLevel` 93-95, then `scrLoadLevel(-1)`, which skips the clamp: `if (argument0 != -1)`, 138), and its info
prompt never stops on the lives or the ropes: each Enter runs the any-key event twice and moves two fields on
(quirks §1), so only the bombs can be typed above 99. A custom level's start values are floored and clamped to 99
(`scrLoadLevel` 140-148; quirks §1). In play, life comes 1 at a time (the damsel's kiss, `oDamselKiss`; the kapala's ninth
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

Two of the runs are committed, with fingerprints, for what no other replay covers: `goal-ice-s55-f1` as
`fuzz-goal-ice-seed-55-ufo-aliens` (an `oAlienEject` from a destroyed UFO, landing as an active `oAlien`) and
`goal-ice-s63-f1` as `fuzz-goal-ice-seed-63-alien-boss` (the alien boss's `oPsychicCreate` and `oPsychicWave`).
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
killing the player (`enemy-yeti-king-seed-14.txt`), a skeleton rising from fake bones and stomped
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
