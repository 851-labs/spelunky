// The runner snaps speed to a whole number within 0.0001 after hspeed or vspeed changes (motion-snap), through
// Delphi's Round into an Int64: does a speed from 2^63 up stop the game, as round(2^63) does (round-2p63), even
// when both components are below 2^63?
var p;
p = 1; repeat (63) p = p * 2; // 2^63
hspeed = p - 1024;
out("speed_2p63-1024_0", speed);
hspeed = 3 * p / 4;
out("speed_3x2p61_0", speed);
vspeed = 3 * p / 4;
out("speed_3x2p61_3x2p61", speed);
