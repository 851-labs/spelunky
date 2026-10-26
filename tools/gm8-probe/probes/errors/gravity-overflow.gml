// friction-2p63 and gravity-2p63: the step's snap doesn't stop the game on a component of 2^63, where a speed
// write's does (late-speedwrite-2p63, in a later step's GML). Does the step stop it on anything? A gravity of
// 2^1023 makes hspeed 2^1023 after one step and overflows it after two, where GML's arithmetic stops the game
// (add-overflow). (A step probe.)
var p;
p = 1; repeat (1023) p = p * 2;
gravity_direction = 0;
gravity = p;
// step:
var p;
p = 1; repeat (1023) p = p * 2;
if (global.probe_step == 2) {
    out("gravity_overflow_hspeed_2p1023", hspeed);
    out("gravity_overflow_speed_2p1023", speed);
}
if (global.probe_step == 3) {
    out("gravity_overflow_hspeed_past_2p1023", hspeed > p);
    out("gravity_overflow_x_past_2p1023", x > p);
    out("end", "ok");
    game_end();
}
