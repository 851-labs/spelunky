// gamepad

globalvar joyid;

joyid = 1;

if (not joystick_exists(1) and joystick_exists(2)) joyid = 2;

//

global.levelSelect = 0; //DEBUG

global.customLevel = false;
global.testLevel = "";

global.firstTime = false;
global.usedShortcut = false;

global.darkLevel = false;
global.lake = false;

global.music = true;
global.message = "";
global.message2 = "";
global.messageTimer = 0;
global.darknessLerp = 0;
global.hadDarkLevel = false;

global.udjatBlink = false;
global.udjatCounter = 0;

global.cleanSolids = false;

// Tunnel Man
global.tunnel1Max = 100000;
global.tunnel2Max = 200000;
global.tunnel3Max = 300000;
global.tunnel1 = highscore_value(8) - 2000000;
global.tunnel2 = highscore_value(9) - 1000000;

// Minigames
global.mini1 = 0;
global.mini2 = 0;
global.mini3 = 0;
global.scoresStart = 0;
global.isDamsel = false;
global.isTunnelMan = false;

// Highscores
global.newMoney = false;
global.newKills = false;
global.newSaves = false;
global.newTime = false;

scrClearGlobals();

if (highscore_value(1) <= 9000000 and
    highscore_value(2) <= 8000000 and
    highscore_value(3) <= 7000000 and
    highscore_value(4) <= 6000000 and
    highscore_value(5) <= 5000000 and
    highscore_value(6) <= 4000000 and
    highscore_value(7) <= 3000000 and
    highscore_value(8) <= 2000000 and
    highscore_value(9) <= 1000000)
{
    scrResetHighscores();
    global.firstTime = true;
}
//room_goto(rLevel);
