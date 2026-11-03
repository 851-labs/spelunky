// collision_line's precise test on sloped lines (851-3962). GM8 (Spelunky.exe 0x4aceec, 0x4a4fd8, 0x48b0d4) rounds
// the clipped ends half to even and, for a line in neither one row nor one column, walks the box's columns (or
// rows) between them, interpolating the other coordinate from the rounded ends' slope and mapping it into the mask
// with Delphi's Floor; OpenGMK's walk lands on another pixel now and then. A 16 x 16 mask made from a surface (an
// irregular pattern of white points on black, the black taken as transparent), read back pixel by pixel (`mask_N`,
// row N, '1' for a pixel that isn't black), on three instances: `a` as it is at (100, 100), `b` turned 30 degrees at
// (200, 100), `c` scaled 2 by -1.5 at (300, 100), each of an object of its own. For each, 300 lines between points
// around it, from a fixed sequence, x and y in halves (`hits_a`: '1' where collision_line(..., prec) met it).
var s, i, j, row, sp, a, b, c, seed, n, k, inst, obj, cx, x1, y1, x2, y2, hits;
s = surface_create(16, 16);
surface_set_target(s);
draw_clear(c_black);
draw_set_color(c_white);
for (i = 0; i < 16; i += 1) {
    for (j = 0; j < 16; j += 1) {
        if ((i * 7 + j * 13 + i * j) mod 5 < 2 && !(i == 0 && j == 15)) draw_point(i, j);
    }
}
surface_reset_target();
for (j = 0; j < 16; j += 1) {
    row = "";
    for (i = 0; i < 16; i += 1) {
        if (surface_getpixel(s, i, j) == c_black) row += "0"; else row += "1";
    }
    out("mask_" + string(j), row);
}
sp = sprite_create_from_surface(s, 0, 0, 16, 16, true, false, 8, 8);
a = instance_create(100, 100, oBlack);
b = instance_create(200, 100, oMenuTop);
c = instance_create(300, 100, oMenuLeft);
with (a) { visible = false; image_speed = 0; sprite_index = sp; mask_index = sp; }
with (b) { visible = false; image_speed = 0; sprite_index = sp; mask_index = sp; image_angle = 30; }
with (c) { visible = false; image_speed = 0; sprite_index = sp; mask_index = sp; image_xscale = 2; image_yscale = -1.5; }
out("box_a", string(a.bbox_left) + " " + string(a.bbox_top) + " " + string(a.bbox_right) + " " + string(a.bbox_bottom));
out("box_b", string(b.bbox_left) + " " + string(b.bbox_top) + " " + string(b.bbox_right) + " " + string(b.bbox_bottom));
out("box_c", string(c.bbox_left) + " " + string(c.bbox_top) + " " + string(c.bbox_right) + " " + string(c.bbox_bottom));
seed = 1;
for (k = 0; k < 3; k += 1) {
    if (k == 0) { obj = oBlack; cx = 100; }
    if (k == 1) { obj = oMenuTop; cx = 200; }
    if (k == 2) { obj = oMenuLeft; cx = 300; }
    hits = "";
    for (n = 0; n < 300; n += 1) {
        seed = (seed * 75 + 74) mod 65537; x1 = cx - 30 + (seed mod 121) / 2;
        seed = (seed * 75 + 74) mod 65537; y1 = 70 + (seed mod 121) / 2;
        seed = (seed * 75 + 74) mod 65537; x2 = cx - 30 + (seed mod 121) / 2;
        seed = (seed * 75 + 74) mod 65537; y2 = 70 + (seed mod 121) / 2;
        if (collision_line(x1, y1, x2, y2, obj, true, false) >= 0) hits += "1"; else hits += "0";
    }
    if (k == 0) out("hits_a", hits);
    if (k == 1) out("hits_b", hits);
    if (k == 2) out("hits_c", hits);
}
