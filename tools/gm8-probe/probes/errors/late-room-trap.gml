// late-scope: a trap skips the rest of the frame. GM8 changes rooms at the end of the frame, after its last stage:
// is a change requested before the trap made, lost, or left for the next frame? oScreen (persistent, moving 1
// pixel a step) calls room_restart() and then traps in its begin step; a `core` (not persistent) shows whether
// the room restarted, and logs its draws (SS_GetSoundPosition). The runner's code says the change is left
// pending, so the next frame runs no events (no hook call, no motion) and the room restarts at its end. The
// milliseconds between the hook's calls only show which frames drew: under Wine's software renderer a frame's
// draw takes most of its time. The probe ends at the first call after the restart. (A step probe.)
global.probe_log = "";
hspeed = 1;
// step:
var p;
p = 1; repeat (64) p = p * 2;
if (global.probe_step == 1) {
    global.probe_core = instance_create(0, 0, core);
}
if (global.probe_step >= 2) {
    out("room_trap_ms_" + string(global.probe_step), current_time - global.probe_time);
}
global.probe_time = current_time;
if (global.probe_step >= 3) {
    out("room_trap_x_" + string(global.probe_step), x);
    out("room_trap_core_" + string(global.probe_step), instance_exists(global.probe_core));
    out("room_trap_log_" + string(global.probe_step), global.probe_log);
}
if (global.probe_step == 3) {
    room_restart();
    floor(p);
    out("room_trap_after_trap", 1);
}
if (global.probe_step == 4) {
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
global.probe_log += "c" + string(global.probe_step);
return 1;
// script SS_GetSoundPosition:
global.probe_log += "d" + string(global.probe_step);
return 0;
// script SS_GetSoundLength:
return 1;
