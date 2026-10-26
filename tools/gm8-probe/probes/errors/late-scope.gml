// late-trap: a trap in a later step's GML skips the rest of that frame. How much of it, and does it leave
// anything behind? oScreen traps in its begin step, inside a script it calls inside `with` a `core` (the
// SuperSound demo object: no sprite, and a draw event that calls SS_GetSoundPosition, which logs here). Does
// the core still move, animate and draw in that frame, and are `self` and `other` oScreen again in the next?
// (A step probe.)
global.probe_log = "";
global.probe_screen = id;
hspeed = 1;
// step:
var c;
if (global.probe_step == 1) {
    c = instance_create(0, 0, core);
    c.hspeed = 1;
    c.image_speed = 1;
    global.probe_core = c;
}
c = global.probe_core;
if (global.probe_step == 3) {
    out("scope_x_3", x);
    out("scope_core_x_3", c.x);
    out("scope_core_image_3", c.image_index);
    out("scope_log_3", global.probe_log);
    with (c) {
        out("scope_trap", SS_IsHandleValid(1));
    }
    out("scope_after_trap", 1);
}
if (global.probe_step == 4) {
    out("scope_x_4", x);
    out("scope_core_x_4", c.x);
    out("scope_core_image_4", c.image_index);
    out("scope_log_4", global.probe_log);
    out("scope_self_4", id == global.probe_screen);
    out("scope_other_4", other.id == global.probe_screen);
    with (c) {
        out("scope_with_other_4", other.id == global.probe_screen);
    }
    out("end", "ok");
    game_end();
}
// script SS_IsHandleValid:
// floor(2^64) traps, inside `with` and two script calls.
var p;
p = argument0;
repeat (64) p = p * 2;
global.probe_log += "t";
return floor(p);
// script SS_GetSoundBytesPerSecond:
// core's create event: bps, which its draw event divides by.
global.probe_log += "c" + string(global.probe_step);
return 1;
// script SS_GetSoundPosition:
global.probe_log += "d" + string(global.probe_step);
return 0;
// script SS_GetSoundLength:
return 1;
