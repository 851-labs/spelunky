// gravity-overflow traps in gravity's sum. The move itself is x + hspeed, stored as a double (Spelunky.exe
// 0x53e6ce): past the largest double, does it trap too, leaving x where it was and skipping the rest of the frame
// (the `core`'s draw, which logs through SS_GetSoundPosition)? A core at 2^1023 gets an hspeed of 2^1023. That
// write traps itself on Intel, in the speed's snap (speed-store-overflow), which skips the rest of its frame but
// leaves hspeed written: the move is the next frame's. (A step probe.)
global.probe_log = "";
// step:
var p, c;
p = 1; repeat (1023) p = p * 2;
if (global.probe_step == 1) {
    c = instance_create(0, 0, core);
    global.probe_core = c;
    c.x = p;
    out("move_overflow_x_1", c.x);
    c.hspeed = p;
    out("move_overflow_hspeed_1", c.hspeed);
}
c = global.probe_core;
if (global.probe_step >= 2) {
    out("move_overflow_x_" + string(global.probe_step), c.x);
    out("move_overflow_hspeed_" + string(global.probe_step), c.hspeed);
    out("move_overflow_log_" + string(global.probe_step), global.probe_log);
}
if (global.probe_step == 3) {
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundPosition:
global.probe_log += "d" + string(global.probe_step);
return 0;
// script SS_GetSoundLength:
return 1;
