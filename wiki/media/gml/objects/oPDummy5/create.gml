// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
// dummy actor for title

image_speed = 0.6;
if (global.isDamsel) sprite_index = sDamselClimb2;
if (global.isTunnelMan) sprite_index = sTunnelClimb2;
else sprite_index = sClimbUp2;
status = 0;

xVel = 0;
yVel = -1;
