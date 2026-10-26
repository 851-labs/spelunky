// A direction past 2^63 radians whose wrap doesn't trap (direction-2p63): 2^70 degrees, a quotient by 360 below
// 2^63, wraps to about itself. fsin and fcos return an angle past 2^63 unchanged (sin(1e22) is 1e22), so the
// components are about the speed times the angle in radians: 0 at speed 0, and at speed 1 past 2^63, where the
// snap stops the game on Intel?
var p;
p = 1; repeat (70) p = p * 2;
speed = 0;
direction = p;
out("direction_2p70", direction);
out("hspeed_2p70_0", hspeed);
speed = 1;
out("hspeed_2p70_1", hspeed);
out("vspeed_2p70_1", vspeed);
