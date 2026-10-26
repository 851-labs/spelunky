// lengthdir_x is the runner's hspeed, snap included (851-3257): does its snap's Round stop the game from 2^63
// up, as the motion variables' does (speedwrite-2p63)? (Not checked by the port and the oracle: §7.)
var p;
p = 1; repeat (63) p = p * 2;
out("lengthdir_x_2p63-1024_0", lengthdir_x(p - 1024, 0));
out("lengthdir_x_2p63_0", lengthdir_x(p, 0));
