// bbox-trap with a scale that traps on the right side only: a `core` at (2^62, 50) with sTunnelLeft as its mask
// (origin 8, 8, box 2, 0, 13, 15) gets image_xscale = 2^60, so its left side is round(-6 * 2^60 + 2^62) = -2^61
// (0 in 32 bits) and its right one round(6 * 2^60 + 2^62 - 1), past 2^63. Is the scale stored, and the box left
// with the left side computed and the others the mask's (0, 0, 13, 15)? (A step probe.)
global.probe_c = 0;
// step:
var p, c;
p = 1; repeat (63) p = p * 2;
if (global.probe_step == 1) {
    c = instance_create(p / 2, 50, core);
    global.probe_c = c;
    c.mask_index = sTunnelLeft;
    out("bbox_scale_1", string(c.bbox_left) + " " + string(c.bbox_top) + " " + string(c.bbox_right) + " " + string(c.bbox_bottom));
    c.image_xscale = p / 8;
}
c = global.probe_c;
if (global.probe_step == 2) {
    out("bbox_scale_2", string(c.bbox_left) + " " + string(c.bbox_top) + " " + string(c.bbox_right) + " " + string(c.bbox_bottom));
    out("bbox_scale_xscale_2", c.image_xscale);
    c.image_xscale = 1;
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundLength:
return 1;
