// The draw stage's own conversions (851-3304; Spelunky.exe, read only to describe it). A background layer (0x4a459c)
// rounds its tile size, w * xscale and h * yscale, into an Int64 and keeps 32 bits (nothing is drawn if either is 0
// or less), then rounds x; only if that (tiled into the view if htiled) is left of the view's right edge does it
// round y. Each view's projection (0x4a2440) stores the sine and cosine of -view_angle * pi / 180 as singles; past
// 2^63 `fsin` gives its operand back, which traps from 2^128 - 2^103 (2^137 degrees in step 9 is about 3e39 radians;
// 2^126 in step 10 about 1.5e36). Layer 1 shows backgroundNight (32x32) in
// rIntro, whose view 0 is 320 wide and at most at x 640, behind (or, in step 8, in front of) two `core`s (depths 30
// and 10) whose draw events log; each advances by image_speed, 1, a frame. (A step probe.)
global.probe_log = "";
// step:
var c1, c3, p, a, k;
p = 1; repeat (63) p = p * 2;
a = 1; repeat (126) a = a * 2;
k = global.probe_step;
if (k == 1) {
    c1 = instance_create(0, 0, core);
    c3 = instance_create(0, 0, core);
    c1.depth = 30;
    c3.depth = 10;
    global.probe_c1 = c1;
    global.probe_c3 = c3;
    background_index[1] = backgroundNight;
    background_visible[1] = true;
    background_foreground[1] = false;
    background_htiled[1] = true;
    background_vtiled[1] = true;
    background_x[1] = 0;
    background_y[1] = 0;
    background_xscale[1] = 1;
    background_yscale[1] = 1;
    background_hspeed[1] = 0;
    background_vspeed[1] = 0;
}
c1 = global.probe_c1;
c3 = global.probe_c3;
if (k >= 2) {
    out("draw_stage_log_" + string(k), global.probe_log);
    out("draw_stage_c1_" + string(k), c1.image_index);
    out("draw_stage_c3_" + string(k), c3.image_index);
    global.probe_log = "";
}
if (k == 2) background_x[1] = p;
if (k == 3) {
    background_x[1] = 0;
    background_y[1] = p;
}
if (k == 4) {
    background_htiled[1] = false;
    background_x[1] = 2 * p / 2147483648 + 5000;
}
if (k == 5) background_x[1] = 2 * p / 2147483648;
if (k == 6) {
    background_htiled[1] = true;
    background_x[1] = 0;
    background_y[1] = 0;
    background_xscale[1] = p / 16;
}
if (k == 7) {
    background_xscale[1] = 134217728;
    background_y[1] = p;
}
if (k == 8) {
    background_xscale[1] = 1;
    background_y[1] = 0;
    background_foreground[1] = true;
    background_x[1] = p;
}
if (k == 9) {
    background_visible[1] = false;
    background_foreground[1] = false;
    background_x[1] = 0;
    view_angle[0] = a * 2048;
}
if (k == 10) view_angle[0] = a;
if (k == 11) view_angle[0] = 0;
if (k == 13) {
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
