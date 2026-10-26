// late-scope with the trap in a create event that instance_create runs from oScreen's begin step: the instance
// exists before its create event runs, so is it left in the room, its create event cut short, while the rest of
// the frame is skipped? The `core`'s create event calls SS_GetSoundBytesPerSecond, which traps here once flagged.
// The probe ends in the next begin step, before the core's draw reads the `bps` its create event never set.
// (A step probe.)
global.probe_trap_create = false;
global.probe_log = "";
hspeed = 1;
// step:
if (global.probe_step == 2) {
    out("instcreate_x_2", x);
    global.probe_trap_create = true;
    global.probe_core = instance_create(0, 0, core);
    out("instcreate_after_trap", 1);
}
if (global.probe_step == 3) {
    out("instcreate_x_3", x);
    out("instcreate_log", global.probe_log);
    out("instcreate_cores", instance_number(core));
    with (core) out("instcreate_core_bps", variable_local_exists("bps"));
    with (core) out("instcreate_core_handle", variable_local_exists("handle"));
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
var p;
global.probe_log += "c" + string(global.probe_step);
if (global.probe_trap_create) {
    global.probe_trap_create = false;
    p = 1; repeat (64) p = p * 2;
    // + 1: Rosetta's floor(2^64) is 0, and the core's draw divides by bps.
    return floor(p) + 1;
}
return 1;
// script SS_GetSoundPosition:
global.probe_log += "d" + string(global.probe_step);
return 0;
// script SS_GetSoundLength:
return 1;
