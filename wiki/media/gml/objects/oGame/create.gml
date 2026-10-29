// ==== action 0: lib 1 id 605: comment: Create Game ====
// ==== action 1: lib 1 id 603: code ====
drawStatus = 0;
moneyCount = 0;
globalvar debug;
debug = false;
global.ghostExists = false;

damsel = false;
idol = false;
altar = false;

// Black Market
genClothingShop = false;
genBombShop = false;
genSupplyShop = false;
genRareShop = false;
genWeaponShop = false;

if (instance_number(oGame) > 1) instance_destroy();

moveableSolidGrav = 1; // how fast moveable solids should accelerate downwards
time = 1;

// sound_loop(sndMusicTest);

if (global.gameStart) scrInitLevel();
