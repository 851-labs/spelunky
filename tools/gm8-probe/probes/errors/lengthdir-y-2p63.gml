// lengthdir-2p63 for lengthdir_y, the runner's vspeed: does its snap stop the game from 2^63 up?
var p;
p = 1; repeat (63) p = p * 2;
out("lengthdir_y_2p63-1024_270", lengthdir_y(p - 1024, 270));
out("lengthdir_y_2p63_270", lengthdir_y(p, 270));
