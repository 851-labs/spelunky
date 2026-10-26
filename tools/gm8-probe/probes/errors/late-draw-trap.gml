// late-scope with the trap in the draw stage, the frame's last but animation: a `core`'s draw event traps (in
// SS_GetSoundLength, once flagged). Is the animation after it skipped, for the core and for oScreen (no sprite,
// so each advances by image_speed, 1, every frame), and does the next frame run as usual? (A step probe.)
global.probe_trap_draw = false;
global.probe_log = "";
// step:
if (global.probe_step == 1) {
    global.probe_core = instance_create(0, 0, core);
}
if (global.probe_step >= 2) {
    out("draw_trap_image_" + string(global.probe_step), image_index);
    out("draw_trap_core_image_" + string(global.probe_step), global.probe_core.image_index);
    out("draw_trap_log_" + string(global.probe_step), global.probe_log);
}
if (global.probe_step == 2) {
    global.probe_trap_draw = true;
}
if (global.probe_step == 4) {
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundPosition:
global.probe_log += "d" + string(global.probe_step);
return 0;
// script SS_GetSoundLength:
var p;
if (global.probe_trap_draw) {
    global.probe_trap_draw = false;
    global.probe_log += "t";
    p = 1; repeat (64) p = p * 2;
    // + 1: Rosetta's floor(2^64) is 0, and the draw event divides by it.
    return floor(p) + 1;
}
return 1;
