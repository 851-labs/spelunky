// motion_add adds to the components (unsnapped, 851-3262) and recomputes the speed, which is snapped (speed-2p63):
// does a speed from 2^63 up stop the game when each component stays below it?
var p;
p = 1; repeat (63) p = p * 2;
motion_add(0, 3 * p / 4);
out("motion_add_hspeed_3x2p61", hspeed);
out("motion_add_speed_3x2p61", speed);
motion_add(270, 3 * p / 4);
out("motion_add_speed_3x2p61_3x2p61", speed);
