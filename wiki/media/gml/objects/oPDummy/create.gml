// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
// dummy actor for transitions and ending

TRANSITION = 0;
START = 1;
END = 2;
LAVA = 3;
STOPPED = 99;
status = 0;

yVel = 0;

if (global.isDamsel) sprite_index = sDamselRunL;
else if (global.isTunnelMan) sprite_index = sTunnelRunL;
else sprite_index = sRunLeft;

LEFT = 0;
RIGHT = 1;
facing = RIGHT;
