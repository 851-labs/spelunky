// GM8 stores an instance's bounding box, and recomputes it (Spelunky.exe 0x4aa958) in the setters of its inputs:
// the position (0x4abf7c, which does nothing when both coordinates are unchanged), the scales and angle, the sprite
// and the mask. The routine first copies the mask's box, then overwrites the left, right, top and bottom sides in
// turn, each Round (into an Int64, trapping from 2^63) of its x87 sum; a rotated box first stores the scaled edge
// offsets as doubles, which trap past the largest double. So which writes trap, and which box is left after the
// trap? A `core` (no sprite; its draw event doesn't draw) gets sTunnelLeft (origin 8, 8, box 2, 0, 13, 15) as its
// mask, and in turn: x = 2^63; x = x and y = y; y = 2^63; a rotated scale past the largest double; mask_index set
// at x = 2^63. (A step probe; bbox-scale, bbox-place, bbox-move and bbox-create test the other setters.)
global.probe_c = 0;
// step:
var p, big, c, k;
p = 1; repeat (63) p = p * 2;
big = 1; repeat (1023) big = big * 2;
big = big * 1.5;
k = global.probe_step;
if (k == 1) {
    c = instance_create(100, 50, core);
    global.probe_c = c;
    c.mask_index = sTunnelLeft;
}
c = global.probe_c;
out("bbox_" + string(k), string(c.bbox_left) + " " + string(c.bbox_top) + " " + string(c.bbox_right) + " " + string(c.bbox_bottom));
out("bbox_x_" + string(k), c.x);
out("bbox_y_" + string(k), c.y);
if (k == 1) {
    c.x = p;
}
if (k == 2) {
    c.x = c.x;
    c.y = c.y;
    out("bbox_same_2", 1);
    c.x = 100;
    out("bbox_back_2", string(c.bbox_left) + " " + string(c.bbox_top) + " " + string(c.bbox_right) + " " + string(c.bbox_bottom));
    c.y = p;
}
if (k == 3) {
    c.y = 50;
    c.image_angle = 30;
    out("bbox_rotated_3", string(c.bbox_left) + " " + string(c.bbox_top) + " " + string(c.bbox_right) + " " + string(c.bbox_bottom));
    c.image_xscale = big;
}
if (k == 4) {
    out("bbox_xscale_4", c.image_xscale);
    c.image_xscale = 1;
    c.image_angle = 0;
    c.mask_index = -1;
    c.x = p;
    out("bbox_nomask_4", string(c.bbox_left) + " " + string(c.bbox_top) + " " + string(c.bbox_right) + " " + string(c.bbox_bottom));
    c.mask_index = sTunnelLeft;
}
if (k == 5) {
    out("bbox_mask_5", c.mask_index == sTunnelLeft);
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundLength:
return 1;
