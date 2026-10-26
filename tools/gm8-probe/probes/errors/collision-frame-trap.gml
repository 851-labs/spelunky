// The precise collision tests' own conversions (851-3305): once the bounding boxes meet, each test rounds the
// instance's position (and a point's, a rectangle's corners or a line's ends) with Delphi's Round into an Int64,
// and takes Floor of image_index (a Trunc into an Int64), whatever the mask, before it reads a pixel. So on Intel
// each traps from 2^63 (collision-frame has the frames below it). An oBlack `a` with sMattockHitR's mask at
// (100, 100), whose frame 0 alone has the pixel (97, 94), and an oMenuTop `b` with sBlock's mask scaled to one
// pixel at (200, 60). One trap a step: a line after one that traps is missing, and the next step goes on. Does a
// non-precise test, or one outside the box, still floor (no); does -2^63 fit; does place_meeting's move back
// happen when its test traps; does a line's far end round? And an oMenuLeft `d` with sBlock's mask moved to
// x = 2^63 (its box trapping and staying sBlock's own, bbox-trap): do a point inside that box, and an instance
// meeting it, round its x? (A step probe.)
global.probe_a = 0;
// step:
var p63, p64, k, a, b, d;
p63 = 1; repeat (63) p63 = p63 * 2;
p64 = p63 * 2;
k = global.probe_step;
if (k == 1) {
    a = instance_create(100, 100, oBlack);
    a.visible = false;
    a.image_speed = 0;
    a.mask_index = sMattockHitR;
    b = instance_create(200, 60, oMenuTop);
    b.visible = false;
    b.image_speed = 0;
    b.mask_index = sBlock;
    b.image_xscale = 1 / 16;
    b.image_yscale = 1 / 16;
    global.probe_a = a;
    global.probe_b = b;
    global.probe_d = noone;
    out("cft_frame0", collision_point(97, 94, oBlack, true, false) != noone);
    with (a) out("cft_self0", place_meeting(203, 66, oMenuTop));
}
a = global.probe_a;
b = global.probe_b;
d = global.probe_d;
if (k == 2) {
    a.image_index = p63;
    out("cft_point_before", 1);
    out("cft_point", collision_point(97, 94, oBlack, true, false) != noone);
    out("cft_point_after", 1);
}
if (k == 3) {
    out("cft_index_3", a.image_index);
    out("cft_point_imprecise", collision_point(97, 94, oBlack, false, false) != noone);
    out("cft_point_outside", collision_point(50, 50, oBlack, true, false) != noone);
    out("cft_rect_imprecise", collision_rectangle(97, 94, 97, 94, oBlack, false, false) != noone);
    out("cft_rect_before", 1);
    out("cft_rect", collision_rectangle(97, 94, 97, 94, oBlack, true, false) != noone);
}
if (k == 4) {
    out("cft_line_before", 1);
    out("cft_line", collision_line(97, 94, 97, 94, oBlack, true, false) != noone);
}
if (k == 5) {
    out("cft_self_before", 1);
    with (a) out("cft_self", place_meeting(203, 66, oMenuTop));
}
if (k == 6) {
    out("cft_self_x_6", a.x);
    out("cft_self_y_6", a.y);
    a.x = 100;
    a.y = 100;
    a.image_index = 0;
    b.image_index = p63;
    out("cft_other_before", 1);
    with (a) out("cft_other", place_meeting(203, 66, oMenuTop));
}
if (k == 7) {
    out("cft_other_x_7", a.x);
    out("cft_other_y_7", a.y);
    a.x = 100;
    a.y = 100;
    b.image_index = 0;
    a.image_index = -p63 - 2048;
    out("cft_neg_before", 1);
    out("cft_neg", collision_point(97, 94, oBlack, true, false) != noone);
}
if (k == 8) {
    a.image_index = -p63;
    out("cft_neg2p63", collision_point(97, 94, oBlack, true, false) != noone);
    a.image_index = 0;
    out("cft_line_far_before", 1);
    out("cft_line_far", collision_line(97, 94, 97, p64, oBlack, true, false) != noone);
}
if (k == 9) {
    d = instance_create(0, 0, oMenuLeft);
    d.visible = false;
    d.image_speed = 0;
    d.mask_index = sBlock;
    global.probe_d = d;
    out("cft_d_before", 1);
    d.x = p63;
}
if (k == 10) {
    out("cft_d_box", string(d.bbox_left) + " " + string(d.bbox_top) + " " + string(d.bbox_right) + " " + string(d.bbox_bottom));
    out("cft_pos_imprecise", collision_point(5, 5, oMenuLeft, false, false) != noone);
    out("cft_pos_before", 1);
    out("cft_pos", collision_point(5, 5, oMenuLeft, true, false) != noone);
}
if (k == 11) {
    b.x = 8;
    b.y = 8;
    out("cft_pos_inst_before", 1);
    with (b) out("cft_pos_inst", place_meeting(8, 8, oMenuLeft));
}
if (k == 12) {
    out("end", "ok");
    game_end();
}
