// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
makeActive();
setCollisionBounds(2, 0, sprite_width-2, sprite_height);
xVel = 2.5;
image_speed = 0.5;

// stats
type = "ManTrap";
hp = 3;
invincible = 0;
favor = 2;

IDLE = 0;
WALK = 1;
ATTACK = 2;
SLEEPY = 96;
EATING = 97;
STUNNED = 98;
DEAD = 99;
status = IDLE;

ateShopkeeper = false;

bounced = false;
dead = false;
counter = 0;

LEFT = 0;
RIGHT = 1;
facing = RIGHT;

shakeCounter = 0;
shakeToggle = 1;
