// The step's friction changes the speed and recomputes the components, each snapped (speedwrite-2p63). Negative
// friction speeds a moving instance up: does a step that takes hspeed to 2^63 stop the game? (A step probe:
// oScreen, which is persistent, moves.)
var p;
p = 1; repeat (62) p = p * 2;
speed = p;
friction = -(p - 1024);
// step:
var p;
p = 1; repeat (62) p = p * 2;
if (global.probe_step == 1) {
    out("friction_before_hspeed_2p62", hspeed);
}
if (global.probe_step == 2) {
    out("friction_hspeed_2p62_-2p62+1024", hspeed);
    out("friction_speed_2p62_-2p62+1024", speed);
    speed = p;
    friction = -p;
}
if (global.probe_step == 3) {
    out("friction_hspeed_2p62_-2p62", hspeed);
    out("end", "ok");
    game_end();
}
