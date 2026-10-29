// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
makeActive();
setCollisionBounds(-4, -4, 4, 4);
xVel = 0;
yVel = -6;
image_speed = 0.6;

// stats
type = "Alien Eject";
hp = 1;
invincible = 0;
countsAsKill = true;

EJECT = 0;
DEPLOY = 1;
FLOAT = 2;

status = 0;

dir = rand(0,1);
counter = 0;
