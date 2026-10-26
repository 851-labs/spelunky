// The default draw of an instance without a draw event (Spelunky.exe 0x4bf4f5) takes Delphi's Floor of image_index
// for the frame to draw, whose Trunc traps from 2^63. An `oBlack` (a sprite, no events) is drawn between two `core`s
// (depths 30, 20 and 10), whose draw events log through SS_GetSoundPosition. With its image_index at 2^63, is the
// first core drawn, and the second core and the animation (each core advances by image_speed, 1, a frame) skipped?
// (A step probe.)
global.probe_log = "";
// step:
var p, c1, b, c3;
p = 1; repeat (63) p = p * 2;
if (global.probe_step == 1) {
    c1 = instance_create(0, 0, core);
    b = instance_create(0, 0, oBlack);
    c3 = instance_create(0, 0, core);
    c1.depth = 30;
    b.depth = 20;
    c3.depth = 10;
    b.image_speed = 0;
    global.probe_c1 = c1;
    global.probe_b = b;
    global.probe_c3 = c3;
}
c1 = global.probe_c1;
b = global.probe_b;
c3 = global.probe_c3;
if (global.probe_step >= 2) {
    out("draw_floor_log_" + string(global.probe_step), global.probe_log);
    out("draw_floor_c1_" + string(global.probe_step), c1.image_index);
    out("draw_floor_c3_" + string(global.probe_step), c3.image_index);
    out("draw_floor_b_" + string(global.probe_step), b.image_index);
}
if (global.probe_step == 2) {
    b.image_index = p;
}
if (global.probe_step == 3) {
    b.image_index = 0;
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
return 1;
