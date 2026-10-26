// speedwrite-2p63 two steps later, after the runner has drawn frames: friction-2p63's step doesn't stop the game on
// a component of 2^63. Does a speed write in GML still stop it then, as it does in the first create event?
// (A step probe.)
// step:
var p;
p = 1; repeat (63) p = p * 2;
if (global.probe_step == 3) {
    speed = p - 1024;
    out("late_hspeed_speed_2p63-1024", hspeed);
    speed = p;
    out("late_hspeed_speed_2p63", hspeed);
    out("end", "ok");
    game_end();
}
