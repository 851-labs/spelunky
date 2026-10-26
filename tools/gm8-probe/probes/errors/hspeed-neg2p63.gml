// speed-2p63 with a negative component: the speed from hspeed = -2^63 is 2^63, whose snap stops the game on
// Intel where the component itself would fit an Int64.
var p;
p = 1; repeat (63) p = p * 2;
hspeed = -(p - 1024);
out("speed_-2p63+1024_0", speed);
hspeed = -p;
out("speed_-2p63_0", speed);
