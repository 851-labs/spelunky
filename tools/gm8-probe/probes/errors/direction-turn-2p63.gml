// A direction write recomputes the components from the speed, each snapped (speedwrite-2p63): with a speed of
// 2^63 at 45 degrees both are below 2^63; does turning to 0 degrees, which makes hspeed 2^63, stop the game?
var p;
p = 1; repeat (63) p = p * 2;
direction = 45;
speed = p;
out("hspeed_45_2p63", hspeed);
out("vspeed_45_2p63", vspeed);
direction = 0;
out("hspeed_0_2p63", hspeed);
