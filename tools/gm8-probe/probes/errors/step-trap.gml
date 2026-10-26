// friction-2p63, gravity-2p63 and gravity-overflow: in the step, a trap doesn't stop the game, but what it traps
// on isn't stored. How much of the instance's motion does it skip? Friction takes the speed past 2^63 at 1
// degree, so hspeed's snap traps: is vspeed still recomputed, gravity (2^40 downwards) still added, and the
// instance still moved? (A step probe: oScreen moves.)
var p;
p = 1; repeat (62) p = p * 2;
x = 0;
y = 0;
direction = 1;
speed = 1.5 * p;
friction = -p;
gravity_direction = 270;
gravity = p / 4194304;
// step:
if (global.probe_step == 1) {
    out("step_trap_before_h", hspeed);
    out("step_trap_before_v", vspeed);
}
if (global.probe_step == 2) {
    out("step_trap_h", hspeed);
    out("step_trap_v", vspeed);
    out("step_trap_s", speed);
    out("step_trap_d", direction);
    out("step_trap_x", x);
    out("step_trap_y", y);
    out("end", "ok");
    game_end();
}
