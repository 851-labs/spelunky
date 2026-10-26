// step-trap with more instances. The runner's code (Spelunky.exe 0x53e670) moves the instances one at a time:
// friction, gravity, then the move, where OpenGMK does every instance's friction and gravity before any move. So
// when the middle of three `core` instances traps in friction's snap (as in step-trap), has the first one had
// its gravity and moved, and the last one neither? All three move 1 pixel right and have a gravity of 1 down;
// the middle one is also as in step-trap. (A step probe: each line is read before the next step's motion.)
hspeed = 1;
// step:
var p, c1, c2, c3;
p = 1; repeat (62) p = p * 2;
if (global.probe_step == 1) {
    c1 = instance_create(0, 0, core);
    c2 = instance_create(0, 0, core);
    c3 = instance_create(0, 0, core);
    with (c1) { hspeed = 1; gravity = 1; }
    with (c3) { hspeed = 1; gravity = 1; }
    with (c2) {
        direction = 1;
        speed = 1.5 * p;
        friction = -p;
        gravity_direction = 270;
        gravity = p / 4194304;
    }
    global.probe_c1 = c1;
    global.probe_c2 = c2;
    global.probe_c3 = c3;
    out("order_x_1", x);
}
if (global.probe_step == 2) {
    c1 = global.probe_c1;
    c2 = global.probe_c2;
    c3 = global.probe_c3;
    out("order_ids", c1 < c2 and c2 < c3);
    out("order_x_2", x);
    out("order_c1_x", c1.x);
    out("order_c1_y", c1.y);
    out("order_c1_v", c1.vspeed);
    out("order_c2_h", c2.hspeed);
    out("order_c2_v", c2.vspeed);
    out("order_c2_s", c2.speed);
    out("order_c2_x", c2.x);
    out("order_c3_x", c3.x);
    out("order_c3_y", c3.y);
    out("order_c3_v", c3.vspeed);
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundLength:
return 1;
