// speedwrite-2p63 from below: a speed write snaps each new component through Delphi's Round into an Int64,
// which holds -2^63 but not the next double down. Does hspeed = -2^63 go through, and -2^63 - 2048 stop the game?
var p;
p = 1; repeat (63) p = p * 2;
speed = -p;
out("hspeed_speed_-2p63", hspeed);
speed = -p - 2048;
out("hspeed_speed_-2p63-2048", hspeed);
