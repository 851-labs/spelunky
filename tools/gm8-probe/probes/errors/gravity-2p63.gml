// The step's gravity adds to the components through motion_add's routine (unsnapped) and recomputes the speed,
// which is snapped (speed-2p63): does a step that takes the speed to 2^63 stop the game? (A step probe.)
var p;
p = 1; repeat (63) p = p * 2;
gravity_direction = 0;
gravity = p - 1024;
// step:
var p;
p = 1; repeat (63) p = p * 2;
if (global.probe_step == 1) {
    out("gravity_before_hspeed", hspeed);
}
if (global.probe_step == 2) {
    out("gravity_hspeed_2p63-1024", hspeed);
    out("gravity_speed_2p63-1024", speed);
    hspeed = 0;
    gravity = p;
}
if (global.probe_step == 3) {
    out("gravity_speed_2p63", speed);
    out("end", "ok");
    game_end();
}
