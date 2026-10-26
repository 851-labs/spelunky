// motion_set(dir, speed) sets direction and speed, so the components are recomputed and snapped as for a speed
// write (speedwrite-2p63): at 45 degrees a speed of 2^63 fits; at 0 degrees does it stop the game?
var p;
p = 1; repeat (63) p = p * 2;
motion_set(45, p);
out("motion_set_hspeed_45_2p63", hspeed);
motion_set(0, p);
out("motion_set_hspeed_0_2p63", hspeed);
