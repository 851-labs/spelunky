// How the point and rectangle tests compare unrounded coordinates with a bounding box (851-3306). GM8's point test
// (Spelunky.exe 0x4ac734) passes the box for left <= x < right + 1, and its rectangle test (0x4ac834) takes the
// smaller and larger corner of the unrounded reals and passes for min < right + 1 and max >= left; only a precise
// test then rounds the corners (Delphi's Round into an Int64, cut to 32 bits) for the pixels it walks, the rounded
// rectangle's intersection with the box. OpenGMK tests a point as left <= x <= right and rounds the corners before
// it compares them. The line test (0x4aceec) already passes for min < right + 1, as OpenGMK's does.
//
// An oBlack `a` with sBlock's mask (16 x 16, solid) at (100, 100), box 100..115 both ways; for each coordinate v,
// the tests that meet it, as bits: 1 collision_point(v, 108), 2 collision_point(108, v), 4 the rectangle (v, 108,
// v, 108), 8 (90, 104, v, 112), 16 the same reversed, 32 (v, 104, 130, 112), 64 reversed, 128 (104, 90, 112, v),
// 256 (104, v, 112, 130), 512 collision_line(v, 108, v, 108), 1024 collision_line(108, v, 108, v), and with prec
// 2048 instance_position(v, 108). Then an oMenuTop `c` with sMattockHitR's mask at (200, 100), whose frame 0 has
// the one pixel (197, 94), the precise tests near it; an oMenuLeft `e` without a sprite or mask, whose box is the
// pixel (-100000, -100000), the tests near it; and rectangles with corners cut to 32 bits.
var a, c, e, v, n, i, p32, np, pr, cut, w, u, k;
a = instance_create(100, 100, oBlack);
a.visible = false;
a.image_speed = 0;
a.mask_index = sBlock;
c = instance_create(200, 100, oMenuTop);
c.visible = false;
c.image_speed = 0;
c.mask_index = sMattockHitR;
e = instance_create(300, 100, oMenuLeft);
e.visible = false;
e.image_speed = 0;
e.sprite_index = -1;
e.mask_index = -1;
out("edges_box_a", string(a.bbox_left) + " " + string(a.bbox_top) + " " + string(a.bbox_right) + " " + string(a.bbox_bottom));
out("edges_box_c", string(c.bbox_left) + " " + string(c.bbox_top) + " " + string(c.bbox_right) + " " + string(c.bbox_bottom));
out("edges_box_e", string(e.bbox_left) + " " + string(e.bbox_top) + " " + string(e.bbox_right) + " " + string(e.bbox_bottom));
n = 0;
v[n] = 99; n += 1;
v[n] = 99.4; n += 1;
v[n] = 99.5; n += 1;
v[n] = 99.6; n += 1;
v[n] = 99.99; n += 1;
v[n] = 100; n += 1;
v[n] = 100.5; n += 1;
v[n] = 114.5; n += 1;
v[n] = 115; n += 1;
v[n] = 115.4; n += 1;
v[n] = 115.5; n += 1;
v[n] = 115.6; n += 1;
v[n] = 115.99; n += 1;
v[n] = 116; n += 1;
v[n] = 116.5; n += 1;
for (i = 0; i < n; i += 1) {
    out("edges_v_" + string(i), v[i]);
    for (k = 0; k < 2; k += 1) {
        np = 0;
        np += 1 * (collision_point(v[i], 108, oBlack, k, false) != noone);
        np += 2 * (collision_point(108, v[i], oBlack, k, false) != noone);
        np += 4 * (collision_rectangle(v[i], 108, v[i], 108, oBlack, k, false) != noone);
        np += 8 * (collision_rectangle(90, 104, v[i], 112, oBlack, k, false) != noone);
        np += 16 * (collision_rectangle(v[i], 112, 90, 104, oBlack, k, false) != noone);
        np += 32 * (collision_rectangle(v[i], 104, 130, 112, oBlack, k, false) != noone);
        np += 64 * (collision_rectangle(130, 112, v[i], 104, oBlack, k, false) != noone);
        np += 128 * (collision_rectangle(104, 90, 112, v[i], oBlack, k, false) != noone);
        np += 256 * (collision_rectangle(104, v[i], 112, 130, oBlack, k, false) != noone);
        np += 512 * (collision_line(v[i], 108, v[i], 108, oBlack, k, false) != noone);
        np += 1024 * (collision_line(108, v[i], 108, v[i], oBlack, k, false) != noone);
        if (k) {
            np += 2048 * (instance_position(v[i], 108, oBlack) != noone);
            out("edges_p_" + string(i), np);
        } else {
            out("edges_np_" + string(i), np);
        }
    }
}
// The one-pixel mask, precise: 1 collision_point(w, 94), 2 collision_point(197, w - 103), 4 the rectangle (w, 94,
// w, 94), 8 (190, 94, w, 94), 16 (w, 94, 205, 94), 32 collision_line(w, 94, w, 94).
n = 0;
w[n] = 196; n += 1;
w[n] = 196.4; n += 1;
w[n] = 196.5; n += 1;
w[n] = 196.6; n += 1;
w[n] = 197; n += 1;
w[n] = 197.4; n += 1;
w[n] = 197.5; n += 1;
w[n] = 197.6; n += 1;
w[n] = 198; n += 1;
for (i = 0; i < n; i += 1) {
    pr = 0;
    pr += 1 * (collision_point(w[i], 94, oMenuTop, true, false) != noone);
    pr += 2 * (collision_point(197, w[i] - 103, oMenuTop, true, false) != noone);
    pr += 4 * (collision_rectangle(w[i], 94, w[i], 94, oMenuTop, true, false) != noone);
    pr += 8 * (collision_rectangle(190, 94, w[i], 94, oMenuTop, true, false) != noone);
    pr += 16 * (collision_rectangle(w[i], 94, 205, 94, oMenuTop, true, false) != noone);
    pr += 32 * (collision_line(w[i], 94, w[i], 94, oMenuTop, true, false) != noone);
    out("edges_hole_w_" + string(i), w[i]);
    out("edges_hole_" + string(i), pr);
}
// No mask, without and with prec: 1 collision_point(u, -100000), 2 the rectangle (u, -100000, u, -100000), 4
// (u, -100000, -99990, -100000), 8 (-100010, -100000, u, -100000), 16 collision_line(u, -100000, u, -100000).
n = 0;
u[n] = -100000.6; n += 1;
u[n] = -100000.5; n += 1;
u[n] = -100000; n += 1;
u[n] = -99999.5; n += 1;
u[n] = -99999.4; n += 1;
u[n] = -99999.01; n += 1;
u[n] = -99999; n += 1;
for (i = 0; i < n; i += 1) {
    out("edges_none_u_" + string(i), u[i]);
    for (k = 0; k < 2; k += 1) {
        np = 0;
        np += 1 * (collision_point(u[i], -100000, oMenuLeft, k, false) != noone);
        np += 2 * (collision_rectangle(u[i], -100000, u[i], -100000, oMenuLeft, k, false) != noone);
        np += 4 * (collision_rectangle(u[i], -100000, -99990, -100000, oMenuLeft, k, false) != noone);
        np += 8 * (collision_rectangle(-100010, -100000, u[i], -100000, oMenuLeft, k, false) != noone);
        np += 16 * (collision_line(u[i], -100000, u[i], -100000, oMenuLeft, k, false) != noone);
        out("edges_none_" + string(k) + "_" + string(i), np);
    }
}
// Corners past 2^32, which the precise test cuts to 32 bits after it picks the smaller and larger: (2^32 + 50,
// 104, 105, 112), (2^32 - 10, 104, 108, 112), (100, 104, 2^32 + 105, 112), (2^32 + 100, 104, 2^32 + 110, 112), as
// 1 without prec and 2 with.
p32 = 1; repeat (32) p32 = p32 * 2;
cut[0] = p32 + 50; cut[1] = 104; cut[2] = 105; cut[3] = 112;
cut[4] = p32 - 10; cut[5] = 104; cut[6] = 108; cut[7] = 112;
cut[8] = 100; cut[9] = 104; cut[10] = p32 + 105; cut[11] = 112;
cut[12] = p32 + 100; cut[13] = 104; cut[14] = p32 + 110; cut[15] = 112;
for (i = 0; i < 4; i += 1) {
    np = 0;
    for (k = 0; k < 2; k += 1) {
        np += (k + 1) * (collision_rectangle(cut[4 * i], cut[4 * i + 1], cut[4 * i + 2], cut[4 * i + 3], oBlack, k, false) != noone);
    }
    out("edges_cut_" + string(i), np);
}
