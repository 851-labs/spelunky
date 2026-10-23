# Random-input fuzzing coverage

What the random-input sweep (851-3183) exercised. All 100 runs match the oracle frame for frame (after #33):

```sh
scripts/cargo run --release -p harness --bin fuzz-sweep -- --seeds 1-100 --jobs 3   # verdicts; about 40 s a run
scripts/cargo run --release -p harness --bin fuzz-sweep -- --seeds 1-100 --port-only # this report, in seconds
```

That's 100 runs of 3,000 random frames each, after the prelude (372,550 frames in all), with game seeds 1-100. The starts go
in turn: the first mines level (through START), and levels 5, 9 and 13 (through the title's shortcut
doors).

## Coverage

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
