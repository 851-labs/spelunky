# Coverage

What the committed replays (`fidelity/replays/`) exercised, [all together](#all-committed-replays), and what the
fuzz sweeps did: [goal-directed](#goal-directed-851-3189) runs, which get through levels, and plain
[random](#random-input-851-3183) ones. [Deliberate interactions](#deliberate-interactions-851-3137-851-3153-851-3154)
lists which replays exercise the player's actions, items, bombs and ropes, and
[Game flow](#game-flow-851-3159) the transitions, deaths, pause, HUD and deactivation.

## All committed replays

Every replay in `fidelity/replays/` matched the oracle with `--fields` when it was added. `replay-coverage` plays
them on the port (no oracle) and reports which objects they created, by gameplay ticket, in a few minutes:

```sh
scripts/cargo run --release -p harness --bin replay-coverage                # all of fidelity/replays/
scripts/cargo run --release -p harness --bin replay-coverage -- --by-object a.txt b.txt  # which replay made what, visited which rooms, died
```

96 replays, 209,091 frames:

Objects by ticket. *Active* objects were in or near the view on some frame, so their events ran; *inactive* ones only existed deactivated (generated, but never near the player); *never* ones weren't created at all.

| Ticket | Area | Active | Inactive only | Never |
|---|---|---|---|---|
| 851-3147 | enemies | 36 of 40: oAlienBoss oBarrier oBarrierEmitter oBat oCaveman oCavemanWorship oDeadFish oEnemySight oFireFrog oFrog oGhost oGiantSpider oGiantSpiderHang oHawkman oHawkmanWorship oJaws oMagma oMagmaTrail oManTrap oMonkey oOlmec oOlmecDebris oOlmecSlam oPiranha oScarab oSkeleton oSnake oSpider oSpiderHang oTombLord oUFO oUFOCrash oVampire oYeti oYetiKing oZombie |  | oAlien oAlienEject oEntranceCheck oMagmaMan |
| 851-3152 | items | 73 of 77: oAnkh oArrow oBall oBall2 oBomb oBombBag oBombBox oBones oBow oCape oCapePickup oChain oChain2 oChest oCompass oCrate oCrown oCrystalSkull oDiamond oDice oEmerald oEmeraldBig oFakeBones oFireFrogBomb oFishBone oFlare oFlareCrate oGloves oGoldBar oGoldBars oGoldChunk oGoldIdol oGoldNugget oJar oJetpack oKapala oKey oLampItem oLampRedItem oLockedChest oMachete oMachetePre oMattock oMattockHead oMattockHit oMattockPre oMitt oParaPickup oParaUsed oParachute oPaste oPistol oRock oRope oRopePile oRopeThrow oRopeTop oRuby oRubyBig oSapphire oSapphireBig oSceptre oShotgun oSkull oSlash oSpectacles oSpikeShoes oSpringShoes oTeleporter oUdjatEye oWebCannon oWhip oWhipPre |  | oBasketball oFireFrogArmed oJordans oRopeBag |
| 851-3157 | traps | 20 of 22: oArrowRepeaterL oArrowRepeaterR oArrowTrapLeft oArrowTrapLeftLit oArrowTrapRight oArrowTrapRightLit oArrowTrapTest oBoulder oCeilingTrap oDoor oGiantTikiHead oSmashTrap oSmashTrapLit oSpearTrapBottom oSpearTrapLit oSpearTrapTop oSpearsLeft oSpikes oSpringTrap oTrapBlock |  | oTemp oThwompTrap |
| 851-3158 | effects | 37 of 41: oBigCollect oBlood oBloodSpark oBloodTrail oBone oBubble oBullet oBurn oDrip oExplosion oFlame oFlameTrail oFlareSpark oFly oHeart oItemsGet oLaser oLaserExplode oLaserTrail oLavaDrip oLeaf oPoof oPsychicCreate2 oPsychicCreateP oPsychicWaveP oRubble oRubbleDark oRubbleDarkSmall oRubbleSmall oShotgunBlastLeft oShotgunBlastRight oSmokePuff oVolcanoFlame oWeb oWebBall oYellowBall oYellowTrail |  | oPsychicCreate oPsychicWave oRopeBurn oSmallCollect |
| 851-3146 | terrain | 49 of 58: oAlienShip oAltarLeft oAltarRight oBlackFadeUp oBlock oBrick oBrickSmooth oCaveBG2 oCaveBGEntrance oDark oDarkFall oEntrance oExit oFrozenCaveman oGoldDoor oGrave oHardBlock oIce oIceBlock oIceBottom oKaliHead oLadderOrange oLadderTop oLamp oLampRed oLava oLavaSolid oLeaves oLush oMoai oMoai2 oMoai3 oMoaiInside oMsgSign oPushBlock oSacAltarLeft oSacAltarRight oSign oSplash oTemple oTempleFake oThinIce oTree oTreeBranch oVine oWaterSwim oXGold oXMarket oXocBlock |  | oAlienShipFloor oCaveBG oGold oGoldBig oLeavesTile oTikiTorch oTreeBranchTile oTreeTile oVineTop |
| 851-3151 | characters | 7 of 8: oDamsel oDamselKiss oPlayer1 oShopkeeper oShopkeeper2 oTunnelMan oYellHelp |  | oCharacter |
| 851-3159 | game flow | 37 of 77: oBlack oBlackBG oCredits1 oCredits2 oEnd oEnd2 oEnd3 oFinalBoss oGame oGamepad oGlobals oHighscores oIntro oLevel oMenuBottom oMenuLL oMenuLR oMenuLeft oMenuRight oMenuTop oMenuUL oMenuUR oMoonRoom oMusic oPDummy oPDummy2 oPDummy3 oPDummy4 oPDummy5 oRoom oScreen oSolid oSprite oStarsRoom oSunRoom oTitle oTransition |  | oBackdrop oBricks oCaveUp oCheckBox oCheckBox2 oCheckBox3 oCheckBox4 oCursObj oDetritus oDrawnSprite oEndCustom oEnemy oForeground oFrill oHintHand oItem oJoyConfig oJoyConfigButton oKeyConfig oKeyConfigButton oLadder oLevelEditor oLoadLevel oMenu oMenuSel oMoveableSolid oMovingSolid oOKButton oPDummy6 oPageDown oPageUp oPlatform oRadioBox oRubblePiece oSlider1 oSlider2 oSliderBG oTreasure oWalDamsel oWater |

Why the terrain (851-3146) and effects (851-3158) objects that no replay creates are missing:

- Never created by 1.1, so unreachable: `oGold` and `oGoldBig` (only `oLaser` destroys them), `oAlienShipFloor`,
  `oCaveBG` (`scrInitLevel`'s `instance_create` is commented out) and `oSmallCollect` (`oPlayer1`'s is commented
  out). `oVineTop` is generated only for a `P` tile, which no room or obstacle string has.
- The level editor's tiles (F2 on the title, `rLevelEditor`, `scrCreateTile`): `oTreeTile`, `oLeavesTile`,
  `oTreeBranchTile` and `oVineTop`. A custom level (`scrLoadLevel`) builds the ordinary `oTree`/`oVine...` instead.
- Covered by the fuzz sweeps below but by no committed replay: `oTikiTorch` (dark Jungle levels, 1 in 20 per
  spot), `oPsychicCreate` and `oPsychicWave` (the alien boss's attack).
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

## Deliberate interactions (851-3137, 851-3153, 851-3154)

Every replay below is identical to the oracle with `--fields`. The existing replays were checked on the port, frame by
frame, for what happens (which instance dies or breaks next to an explosion, the whip or a thrown item, what the
player holds, what hurts him); the frame is roughly when.

| Interaction | Replays |
|---|---|
| Bomb kills | the shopkeeper (`shop-seed-2-bomb`, ~1986), the damsel (`damsel-seed-6-death`, ~928), a stunned caveman (`action-carry-bomb-caveman`, ~1133); a jar (`ice-seed-2`, ~1218) and a fire frog's bomb (`jungle-seed-10`, ~1542) blown up; terrain (`movement-tutorial-exit`, ~1523); Kali's altar (`weapon-seed-38-web-cannon`, ~1494) |
| Chain explosions | a bomb setting off another (`action-bomb-chain`, ~1523-1531) |
| Bomb pushback | the damsel (`damsel-seed-6-death`), a caveman (`fuzz-room-change-ends-event`, ~1038), a bomb bag, a rope pile, the shotgun and the shopkeeper (`shop-seed-2-bomb`, ~1983), a bomb (`action-bomb-chain`) |
| Bombs placed and thrown | dropped (duck + A: `movement-tutorial-exit`, `shop-seed-2-bomb`), a lit bomb picked up and put down, a bomb placed while holding one, a bomb thrown (`action-bomb-chain`), a bomb taken out with the item key, lit in the hand and thrown up into a ceiling (`item-seed-38-lamp`, ~1268-1393; `item-seed-22-red-lamp`, ~1358-1483) |
| Whip | a caveman (`temple-seed-1`, ~982), a man trap (`jungle-seed-1`, ~2212), zombies (`enemy-skeleton-seed-3`, ~1059; `jungle-seed-10`, ~2716), jars (`enemy-mines-seed-1-kills`, ~837; `shop-seed-12-dice`, ~988), a skull (`enemy-skeleton-seed-3`, ~1061), the damsel (`damsel-seed-6-death`), the shopkeeper (`shop-seed-11-whip`) |
| Thrown items hitting enemies | a rock killing a snake (`action-throw-rock-snake`, ~1100); trap arrows killing snakes (`mines-seed-7`, ~948; `shop-seed-22-kissing-theft`, ~1134) and hitting a caveman (`temple-seed-1`, ~877); a rope's head killing a bat (`movement-tutorial-exit`, ~1020) and a spider (`ending-seed-1`, ~5813) |
| Items breaking | thrown jars (`shop-seed-11-whip`, ~723; `ice-seed-2`, ~1604; `jungle-seed-1`, ~886) and skulls (`enemy-skeleton-seed-3`, ~860; `shop-seed-12-dice`, ~1909) |
| Items in water, lava and on spikes | a rock and a jar in water (`jungle-seed-9-market`, ~1616), a crate (`jungle-seed-1`, ~1394), a skull (`enemy-skeleton-seed-3`, ~840); a jar in lava (`temple-seed-43`, ~1825), arrows (`temple-seed-1`, ~1574); an arrow on spikes (`bonus-moon-archery`, ~958) |
| Ropes | thrown up into a ceiling, climbed and jumped off the top (`movement-tutorial-exit`, ~1015-1129), as the held item, thrown down a ledge (`movement-tutorial-exit`, ~1180) and put down (`movement-items-rope`, ~1395); none thrown under a ceiling, and dropped with duck + S (`action-rope-tunnel`, ~1140-1162) |
| Carrying | a stunned caveman picked up, thrown, picked up again and thrown up (`action-carry-bomb-caveman`, ~897-992; `fuzz-room-change-ends-event`, ~1137-1181), the damsel (`damsel-seed-6-*`, `bonus-sun-damsel`), a rock (`movement-items-rope`) |
| Sticky bombs | the paste stolen, a bomb thrown that sticks to a wall (`item-seed-53-paste`, ~985-1105) |
| Paying | `shop-seed-3-bomb-buy-theft`, `shop-seed-12-dice`, `tunnelman-seed-2-donate` |
| Damage | fall damage and stuns (`movement-ladder-crawl`, `movement-*`, ~889), a bat (`movement-bat-death`), a snake (`shop-seed-3-bomb-buy-theft`, ~984), a spider (`enemy-mines-seed-1-kills`, ~1584), a caveman (`title-shortcut5`, ~892), a yeti (`ice-seed-2`, ~1583), a UFO (`fuzz-room-change-ends-event`, ~1052), a lit arrow trap (`trap-dark-arrow`, ~2016), a shotgun (`bonus-stars-shotgun`), Kali (`weapon-seed-38-web-cannon`) |
| Deaths | a bat (`movement-bat-death`), the boulder (`trap-boulder-crush`), a smash trap (`trap-smash-crush`), spikes (`fuzz-room-change-ends-event`, `enemy-yeti-king-seed-14`), an explosion (`bonus-sun-damsel`, ~1222; `shop-seed-2-bomb`), shotguns (`shop-seed-11-whip`, `shop-seed-3-bomb-buy-theft`, `shop-seed-22-kissing-theft`), the teleporter (`weapon-seed-40-teleporter`), the ghost (`enemy-ghost-seed-2`), the Yeti King (`enemy-yeti-king-seed-14`), Olmec (`olmec-death-seed-1`) |

Not covered yet: a bomb kill of an enemy that isn't stunned (`area-route --steps '*oSnake'` and `'*oEnemy'` found
none on mines seed 1 within 3,000 expansions: the snakes there patrol floors the blast doesn't reach from where
the search stands), a jar or skull thrown at an enemy (`'+oJar,/oEnemy'` found only a stomp), a dead body carried,
a rope that a wall stops (duck + S with walls on both sides), a sticky bomb stuck to an enemy, and an arrow
killing the player.

## Game flow (851-3159)

Every replay below is identical to the oracle with `--fields`; the `flow-*` ones were added for this ticket.
`replay-coverage --by-object` lists the rooms each replay visits and its deaths as well as its objects.

| Behavior | Replays |
|---|---|
| Area transitions | `rTransition1` (mines, 27 replays: `mines-seed-*`, `shop-*`, ...), `rTransition1x` with the Tunnel Man (`tunnelman-seed-2-donate`), `rTransition2` (`jungle-*`, `flow-lake-jungle-seed-1`), `rTransition3` (`ice-*`), `rTransition4` (`temple-*`, `ending-seed-1`); the tutorial's exit to `rTransition1`, whose `scrClearGlobals` starts a real game (`movement-tutorial-exit`) |
| Rooms | `rLevel2` (`ice-*`), the lake `rLevel3` with `oJaws` (`flow-lake-jungle-seed-1`), `rOlmec` and the ending `rEnd`-`rEnd3`, `rCredits2` (`ending-seed-1`), `rCredits1` (`title-locked`), the bonus rooms (`bonus-*`) |
| Keys in a transition room | jump, bomb, rope, item, Escape, F1, F10, run and the directions do nothing there: no `oPlayer1` exists (`flow-transition-keys`) |
| Deaths | 21 replays, 22 deaths: the Deaths row above, and the pause screen's F1 and bomb key (`flow-pause-game-over-new-game`) |
| Game over → `rHighscores` | `bonus-*`, `fuzz-room-change-ends-event`, `olmec-death-seed-1`; then the title and a new game, so `scrClearGlobals` is checked (`flow-pause-game-over-new-game`) |
| Restart | the tutorial restarted after a death (`movement-bat-death`); the tutorial's title door back to `rTitle` (titleStart 3, `scrClearGlobals`) and a new game (`flow-tutorial-title`) |
| Pause | in a level, mid-fall, with keys pressed while paused, then F1 and the bomb key killing (`flow-pause-game-over-new-game`); in the tutorial and on the title (`flow-tutorial-title`); in `rHighscores` and quitting with F10 (`flow-pause-game-over-new-game`); on the title and quitting with the rope key (`ending-seed-1`). While paused only `oScreen` and `oGamepad` are active; the trace compares the rest as deactivated |
| Timers across a pause | `global.time`, `global.xtime`, the "chill" message and the ghost, 700 frames late after two pauses (`flow-pause-ghost-timer`); the ghost unpaused (`enemy-ghost-seed-2`); the dark-level clock (`trap-dark-*`) |
| HUD (`scrDrawHUD`, every level frame, its locals compared) | life changes (every damage replay above), life clamped to 0 after F1's -99 (`flow-pause-game-over-new-game`), the held item for each `pickupItemType` the item replays hold, the compass arrows (`item-seed-3-compass`), the sticky bomb icon (`item-seed-53-paste`), the equipment row (`item-*`, `equip-*`), the arrows beside the bow (`bonus-moon-archery`), `global.collect` (every money pickup) |
| `oLevel` deactivation | every level frame of every replay compares the active and deactivated sets. Instances that moved out of the 96 px window, froze and came back: bones and a skull (`enemy-mines-seed-1-kills`, 1201-1342), a piranha and a bubble (`jungle-seed-10`, 1938-2192); `oEnemySight` flickering at the window's edge (`jungle-seed-10`) |
| The `instance_activate_region` bug | objects right of and below the player stay active outside the window, inside the oversized region: a bat for 108 frames (`action-throw-rock-snake`, from ~805), gems and gold (`movement-bat-death`, `enemy-mines-seed-1-kills`) |

Not covered yet: dying in a transition room is impossible (no `oPlayer1`; `oPDummy*` can't be hurt), and so is
whipping or bombing there. The HUD at its extremes isn't reached: `oPlayer1`'s step caps life, bombs and ropes at
99, and no replay has more than a few thousand dollars. `rTransition2x` and `rTransition3x` (leaving levels 8 and
12) and custom levels (`rLoadLevel`, the editor) aren't in a committed replay. No replay throws an item out of the
window on purpose and follows it back.

## Goal-directed (851-3189)

`fuzz-sweep --directed` alternates bursts of random play (60-240 frames) with routes that `area-route`'s
search (`harness::route`, 600 states per goal) finds toward the level's exit, or sometimes a shop's ware (then
pay), the damsel (then out the exit with her), the dice or a shopkeeper. It skips transitions with the attack
key. All 97 runs swept match the oracle frame for frame:

```sh
scripts/cargo run --release -p harness --bin fuzz-sweep -- --directed --seeds 1-100 --jobs 4
```

That's 97 runs (game seeds 1-100 but 95, 98 and 100, which the time box cut), 3,000 frames
after the prelude each (361,228 frames), starting in turn in the first mines level and levels 5, 9 and 13.
A run takes 1-2 minutes, about half of it the route searches.

### Coverage


Deepest level per run (level: runs): 2: 14, 3: 3, 4: 8, 5: 9, 6: 6, 7: 3, 8: 6, 9: 5, 10: 10, 11: 3, 12: 4, 13: 20, 14: 4, 15: 2  
Rooms (room: runs): rHighscores: 83, rIntro: 97, rLevel: 97, rLevel2: 24, rLevel3: 4, rTitle: 97, rTransition1: 74, rTransition2: 15, rTransition3: 19, rTransition3x: 2, rTransition4: 6  
Deaths: 124  
Held by the player: oBomb, oBombBag, oBow, oDamsel, oGloves, oHawkman, oJar, oParaPickup, oPaste, oRock, oRopeThrow, oSpikeShoes, oSpringShoes, oYeti

Objects by ticket. *Active* objects were in or near the view on some frame, so their events ran; *inactive* ones only existed deactivated (generated, but never near the player); *never* ones weren't created at all.

| Ticket | Area | Active | Inactive only | Never |
|---|---|---|---|---|
| 851-3147 | enemies | 33 of 40: oAlien oAlienBoss oAlienEject oBarrier oBarrierEmitter oBat oCaveman oDeadFish oEnemySight oFireFrog oFrog oGiantSpider oGiantSpiderHang oHawkman oJaws oMagma oMagmaMan oMagmaTrail oManTrap oMonkey oPiranha oScarab oSkeleton oSnake oSpider oSpiderHang oTombLord oUFO oUFOCrash oVampire oYeti oYetiKing oZombie |  | oCavemanWorship oEntranceCheck oGhost oHawkmanWorship oOlmec oOlmecDebris oOlmecSlam |
| 851-3152 | items | 52 of 77: oArrow oBomb oBombBag oBombBox oBones oBow oCapePickup oChest oCompass oCrate oCrown oCrystalSkull oDice oEmeraldBig oFakeBones oFireFrogBomb oFishBone oFlare oFlareCrate oGloves oGoldBar oGoldBars oGoldChunk oGoldIdol oGoldNugget oJar oJetpack oKey oLockedChest oMachete oMattock oMitt oParaPickup oPaste oPistol oRock oRope oRopePile oRopeThrow oRopeTop oRubyBig oSapphireBig oSceptre oShotgun oSkull oSpectacles oSpikeShoes oSpringShoes oTeleporter oWebCannon oWhip oWhipPre |  | oAnkh oBall oBall2 oBasketball oCape oChain oChain2 oDiamond oEmerald oFireFrogArmed oJordans oKapala oLampItem oLampRedItem oMachetePre oMattockHead oMattockHit oMattockPre oParaUsed oParachute oRopeBag oRuby oSapphire oSlash oUdjatEye |
| 851-3157 | traps | 16 of 22: oArrowTrapLeft oArrowTrapLeftLit oArrowTrapRight oArrowTrapRightLit oArrowTrapTest oCeilingTrap oDoor oGiantTikiHead oSmashTrap oSpearTrapBottom oSpearTrapLit oSpearTrapTop oSpearsLeft oSpikes oSpringTrap oTrapBlock |  | oArrowRepeaterL oArrowRepeaterR oBoulder oSmashTrapLit oTemp oThwompTrap |
| 851-3158 | effects | 29 of 41: oBlood oBloodTrail oBone oBubble oBullet oBurn oDrip oExplosion oFlame oFlameTrail oFlareSpark oFly oItemsGet oLaser oLaserExplode oLaserTrail oLeaf oPoof oPsychicCreate oPsychicWave oRubble oRubbleDark oRubbleDarkSmall oRubbleSmall oShotgunBlastLeft oShotgunBlastRight oSmokePuff oWeb oWebBall |  | oBigCollect oBloodSpark oHeart oLavaDrip oPsychicCreate2 oPsychicCreateP oPsychicWaveP oRopeBurn oSmallCollect oVolcanoFlame oYellowBall oYellowTrail |
| 851-3146 | terrain | 46 of 58: oAlienShip oAltarLeft oAltarRight oBlackFadeUp oBlock oBrick oBrickSmooth oCaveBG2 oCaveBGEntrance oDark oDarkFall oEntrance oExit oFrozenCaveman oGoldDoor oGrave oHardBlock oIce oIceBlock oIceBottom oKaliHead oLadderOrange oLadderTop oLamp oLampRed oLava oLeaves oLush oMoai oMoai2 oMoai3 oMoaiInside oPushBlock oSacAltarLeft oSacAltarRight oSign oSplash oTemple oTempleFake oThinIce oTikiTorch oTree oTreeBranch oVine oWaterSwim oXMarket |  | oAlienShipFloor oCaveBG oGold oGoldBig oLavaSolid oLeavesTile oMsgSign oTreeBranchTile oTreeTile oVineTop oXGold oXocBlock |
| 851-3151 | characters | 6 of 8: oDamsel oDamselKiss oPlayer1 oShopkeeper oTunnelMan oYellHelp |  | oCharacter oShopkeeper2 |
| 851-3159 | game flow | 26 of 77: oBlack oBlackBG oGame oGamepad oGlobals oHighscores oIntro oLevel oMenuBottom oMenuLL oMenuLR oMenuLeft oMenuRight oMenuTop oMenuUL oMenuUR oMusic oPDummy oPDummy3 oPDummy4 oRoom oScreen oSolid oSprite oTitle oTransition |  | oBackdrop oBricks oCaveUp oCheckBox oCheckBox2 oCheckBox3 oCheckBox4 oCredits1 oCredits2 oCursObj oDetritus oDrawnSprite oEnd oEnd2 oEnd3 oEndCustom oEnemy oFinalBoss oForeground oFrill oHintHand oItem oJoyConfig oJoyConfigButton oKeyConfig oKeyConfigButton oLadder oLevelEditor oLoadLevel oMenu oMenuSel oMoonRoom oMoveableSolid oMovingSolid oOKButton oPDummy2 oPDummy5 oPDummy6 oPageDown oPageUp oPlatform oRadioBox oRubblePiece oSlider1 oSlider2 oSliderBG oStarsRoom oSunRoom oTreasure oWalDamsel oWater |

The enemies neither sweep makes active have replays of their own (`fidelity/replays/enemy-*.txt`, 851-3147),
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
