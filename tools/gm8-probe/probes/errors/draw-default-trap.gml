// The default draw's corners (851-3304): an instance without a draw event is drawn through the texture draw
// (Spelunky.exe 0x4a5a58) when untransformed, and the scaled and rotated one (0x4a5aa4) otherwise, which store the
// corners as singles (trapping from T = 2^128 - 2^103) and the scaled offsets as doubles. A position or scale that
// large traps in its setter first (the bounding box, 851-3301), but only after storing it, so the next frame draws
// with it. An `oBlack` (sBlack: 16x16, origin 0, 0; no events) is drawn between two `core`s (depths 30, 20, 10),
// whose draw events log; each `core` advances by image_speed, 1, a frame. Steps 2 to 12 (even) store an x, a scale
// or an angle that traps in the setter, and the frame after each draws with it: x = T, x = T - 2^75, xscale =
// 2^1023, then xscale and yscale = 1.5 * 2^123 (corners at 1.5 * 2^127, below T) and the angle 45 (a corner at
// about 2^127 * 2.1, past it). Steps 14 to 16 undo them. (A step probe.)
var t, u;
t = 1; repeat (103) t = t * 2;
global.probe_t = t * 33554431;
u = 1; repeat (75) u = u * 2;
global.probe_tb = global.probe_t - u;
global.probe_log = "";
// step:
var c1, b, c3, big, s, k;
big = 1; repeat (1023) big = big * 2;
s = 1; repeat (123) s = s * 2;
s = s * 1.5;
k = global.probe_step;
if (k == 1) {
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
if (k >= 2) {
    out("draw_default_log_" + string(k), global.probe_log);
    out("draw_default_c1_" + string(k), c1.image_index);
    out("draw_default_c3_" + string(k), c3.image_index);
    global.probe_log = "";
}
if (k == 2) b.x = global.probe_t;
if (k == 4) b.x = global.probe_tb;
if (k == 6) {
    b.x = 0;
    b.image_xscale = big;
}
if (k == 8) b.image_xscale = s;
if (k == 10) b.image_yscale = s;
if (k == 12) b.image_angle = 45;
if (k == 14) b.image_angle = 0;
if (k == 15) b.image_xscale = 1;
if (k == 16) b.image_yscale = 1;
if (k == 18) {
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
