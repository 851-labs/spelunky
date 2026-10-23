# Coverage

What the committed replays (`fidelity/replays/`) exercised, [all together](#all-committed-replays), and what the
fuzz sweeps did: [goal-directed](#goal-directed-851-3189) runs, which get through levels, and plain
[random](#random-input-851-3183) ones.

## All committed replays

Every replay in `fidelity/replays/` matched the oracle with `--fields` when it was added. `replay-coverage` plays
them on the port (no oracle) and reports which objects they created, by gameplay ticket, in a few minutes:

```sh
scripts/cargo run --release -p harness --bin replay-coverage                # all of fidelity/replays/
scripts/cargo run --release -p harness --bin replay-coverage -- --by-object a.txt b.txt  # and which replay made what
```

66 replays, 129,321 frames:

Objects by ticket. *Active* objects were in or near the view on some frame, so their events ran; *inactive* ones only existed deactivated (generated, but never near the player); *never* ones weren't created at all.

| Ticket | Area | Active | Inactive only | Never |
|---|---|---|---|---|
| 851-3147 | enemies | 35 of 40: oAlienBoss oBarrier oBarrierEmitter oBat oCaveman oCavemanWorship oDeadFish oEnemySight oFireFrog oFrog oGhost oGiantSpider oGiantSpiderHang oHawkman oHawkmanWorship oMagma oMagmaTrail oManTrap oMonkey oOlmec oOlmecDebris oOlmecSlam oPiranha oScarab oSkeleton oSnake oSpider oSpiderHang oTombLord oUFO oUFOCrash oVampire oYeti oYetiKing oZombie |  | oAlien oAlienEject oEntranceCheck oJaws oMagmaMan |
| 851-3152 | items | 59 of 77: oAnkh oArrow oBomb oBombBag oBombBox oBones oBow oCape oCapePickup oChest oCompass oCrate oCrown oDice oEmerald oEmeraldBig oFakeBones oFireFrogBomb oFlare oFlareCrate oGloves oGoldBar oGoldBars oGoldChunk oGoldIdol oGoldNugget oJar oJetpack oKey oLockedChest oMachete oMachetePre oMattock oMattockHead oMattockHit oMattockPre oMitt oParaPickup oPaste oPistol oRock oRope oRopePile oRopeThrow oRopeTop oRuby oRubyBig oSapphire oSapphireBig oShotgun oSkull oSlash oSpectacles oSpikeShoes oSpringShoes oTeleporter oWebCannon oWhip oWhipPre |  | oBall oBall2 oBasketball oChain oChain2 oCrystalSkull oDiamond oFireFrogArmed oFishBone oJordans oKapala oLampItem oLampRedItem oParaUsed oParachute oRopeBag oSceptre oUdjatEye |
| 851-3157 | traps | 19 of 22: oArrowRepeaterL oArrowRepeaterR oArrowTrapLeft oArrowTrapLeftLit oArrowTrapRight oArrowTrapRightLit oArrowTrapTest oBoulder oCeilingTrap oDoor oGiantTikiHead oSmashTrap oSpearTrapBottom oSpearTrapLit oSpearTrapTop oSpearsLeft oSpikes oSpringTrap oTrapBlock |  | oSmashTrapLit oTemp oThwompTrap |
| 851-3158 | effects | 33 of 41: oBigCollect oBlood oBloodTrail oBone oBubble oBullet oBurn oDrip oExplosion oFlame oFlameTrail oFlareSpark oFly oHeart oItemsGet oLaser oLaserExplode oLaserTrail oLeaf oPoof oPsychicCreate2 oRubble oRubbleDark oRubbleDarkSmall oRubbleSmall oShotgunBlastLeft oShotgunBlastRight oSmokePuff oVolcanoFlame oWeb oWebBall oYellowBall oYellowTrail |  | oBloodSpark oLavaDrip oPsychicCreate oPsychicCreateP oPsychicWave oPsychicWaveP oRopeBurn oSmallCollect |
| 851-3146 | terrain | 47 of 58: oAlienShip oAltarLeft oAltarRight oBlackFadeUp oBlock oBrick oBrickSmooth oCaveBG2 oCaveBGEntrance oDark oDarkFall oEntrance oExit oFrozenCaveman oGoldDoor oGrave oHardBlock oIce oIceBlock oIceBottom oKaliHead oLadderOrange oLadderTop oLamp oLampRed oLava oLavaSolid oLeaves oLush oMoai oMoai2 oMoai3 oMoaiInside oMsgSign oPushBlock oSacAltarLeft oSacAltarRight oSign oSplash oTemple oTempleFake oThinIce oTree oTreeBranch oVine oWaterSwim oXMarket |  | oAlienShipFloor oCaveBG oGold oGoldBig oLeavesTile oTikiTorch oTreeBranchTile oTreeTile oVineTop oXGold oXocBlock |
| 851-3151 | characters | 7 of 8: oDamsel oDamselKiss oPlayer1 oShopkeeper oShopkeeper2 oTunnelMan oYellHelp |  | oCharacter |
| 851-3159 | game flow | 37 of 77: oBlack oBlackBG oCredits1 oCredits2 oEnd oEnd2 oEnd3 oFinalBoss oGame oGamepad oGlobals oHighscores oIntro oLevel oMenuBottom oMenuLL oMenuLR oMenuLeft oMenuRight oMenuTop oMenuUL oMenuUR oMoonRoom oMusic oPDummy oPDummy2 oPDummy3 oPDummy4 oPDummy5 oRoom oScreen oSolid oSprite oStarsRoom oSunRoom oTitle oTransition |  | oBackdrop oBricks oCaveUp oCheckBox oCheckBox2 oCheckBox3 oCheckBox4 oCursObj oDetritus oDrawnSprite oEndCustom oEnemy oForeground oFrill oHintHand oItem oJoyConfig oJoyConfigButton oKeyConfig oKeyConfigButton oLadder oLevelEditor oLoadLevel oMenu oMenuSel oMoveableSolid oMovingSolid oOKButton oPDummy6 oPageDown oPageUp oPlatform oRadioBox oRubblePiece oSlider1 oSlider2 oSliderBG oTreasure oWalDamsel oWater |

Why the terrain (851-3146) and effects (851-3158) objects that no replay creates are missing:

- Never created by 1.1, so unreachable: `oGold` and `oGoldBig` (only `oLaser` destroys them), `oAlienShipFloor`,
  `oCaveBG` (`scrInitLevel`'s `instance_create` is commented out) and `oSmallCollect` (`oPlayer1`'s is commented
  out). `oVineTop` is generated only for a `P` tile, which no room or obstacle string has.
- The level editor's tiles (F2 on the title, `rLevelEditor`, `scrCreateTile`): `oTreeTile`, `oLeavesTile`,
  `oTreeBranchTile` and `oVineTop`. A custom level (`scrLoadLevel`) builds the ordinary `oTree`/`oVine...` instead.
- The City of Gold: `oXGold` (the sceptre, `oTombLord`'s drop on level 13, touching `oGoldDoor` while the player has
  the crown from the Ice Caves) and `oXocBlock` (its `X` tile, `scrRoomGen4`). Also `oPsychicCreateP` and
  `oPsychicWaveP`, the sceptre's own attack (`scrUseItem`).
- Covered by the fuzz sweeps below but by no committed replay: `oTikiTorch` (dark Jungle levels, 1 in 20 per
  spot), `oLavaDrip`, `oPsychicCreate` and `oPsychicWave` (the alien boss's attack).
- Not yet reached: `oBloodSpark` (blood soaked up with the Kapala, Kali's gift at favor 16, two sacrifices) and
  `oRopeBurn` (a rope segment with lava 12 pixels to its right).

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
