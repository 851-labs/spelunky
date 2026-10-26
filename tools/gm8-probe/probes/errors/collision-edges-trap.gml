// When the rectangle test converts its corners (851-3306): GM8 (Spelunky.exe 0x4ac834) compares the unrounded
// corners with the box, and rounds them (Delphi's Round into an Int64, which traps on Intel from 2^63) only for a
// precise test whose box meets the rectangle and whose instance has a mask; OpenGMK rounds them first, for every
// instance. An oBlack `a` with sBlock's mask at (100, 100), box 100..115. One test a step: a line after one that
// traps is missing, and the next step goes on. A corner at 2^64 without prec, one with prec where the boxes don't
// meet (either side), one where they do (each side), and a point at 2^64 outside the box. (A step probe.)
global.probe_a = 0;
// step:
var p64, k, a;
p64 = 1; repeat (64) p64 = p64 * 2;
k = global.probe_step;
if (k == 1) {
    a = instance_create(100, 100, oBlack);
    a.visible = false;
    a.image_speed = 0;
    a.mask_index = sBlock;
    global.probe_a = a;
    out("cet_near", collision_rectangle(100, 104, 110, 112, oBlack, true, false) != noone);
}
if (k == 2) {
    out("cet_far_imprecise_before", 1);
    out("cet_far_imprecise", collision_rectangle(100, 104, p64, 112, oBlack, false, false) != noone);
}
if (k == 3) {
    out("cet_far_miss_before", 1);
    out("cet_far_miss", collision_rectangle(p64, 104, p64 * 2, 112, oBlack, true, false) != noone);
}
if (k == 4) {
    out("cet_far_miss_neg_before", 1);
    out("cet_far_miss_neg", collision_rectangle(-p64, 104, 50, 112, oBlack, true, false) != noone);
}
if (k == 5) {
    out("cet_far_max_before", 1);
    out("cet_far_max", collision_rectangle(100, 104, p64, 112, oBlack, true, false) != noone);
}
if (k == 6) {
    out("cet_far_min_before", 1);
    out("cet_far_min", collision_rectangle(-p64, 104, 108, 112, oBlack, true, false) != noone);
}
if (k == 7) {
    out("cet_far_point_before", 1);
    out("cet_far_point", collision_point(p64, 108, oBlack, true, false) != noone);
}
if (k == 8) {
    out("end", "ok");
    game_end();
}
