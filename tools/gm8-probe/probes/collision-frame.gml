// Which mask frame the precise collision tests take from image_index (851-3305). Each of GM8's five tests (the
// instance's at Spelunky.exe 0x4ad1a0, the point's at 0x4ac734, the rectangle's at 0x4ac834, the ellipse's and the
// line's) takes Delphi's Floor of image_index (0x4238e8, a 32-bit Integer), and the sprite's routine it calls
// (0x4a4d84, 0x4a4e6c, 0x4a4ee0, 0x4a4f5c, 0x4a4fd8) takes its signed remainder by the frame count, plus the count
// if negative; OpenGMK takes point, rectangle and line frames as an unsigned 64-bit floor, so a negative index is
// frame 0 there. An oBlack with sMattockHitR's mask (3 frames, origin 8, 8; frame f alone has the pixel (-3, -6),
// (-4, 5), (-3, -4) from the origin) at (100, 100), and an oMenuTop with sBlock's mask scaled to one pixel at
// (200, 60). Each line is, for one image_index, the frames whose pixel a test hits, as bits: collision_point,
// collision_rectangle and collision_line (one pixel each), and place_meeting with the oBlack as self and as other.
var a, b, p31, p32, p52, p62, p63, v, n, i, f, w, px, py, s_pt, s_rect, s_line, s_self, s_other, hit;
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
out("frame_count", sprite_get_number(sMattockHitR));
out("frame_box_b", string(b.bbox_left) + " " + string(b.bbox_top) + " " + string(b.bbox_right) + " " + string(b.bbox_bottom));
px[0] = -3; py[0] = -6;
px[1] = -4; py[1] = 5;
px[2] = -3; py[2] = -4;
p31 = 1; repeat (31) p31 = p31 * 2;
p32 = p31 * 2;
p52 = 1; repeat (52) p52 = p52 * 2;
p62 = 1; repeat (62) p62 = p62 * 2;
p63 = p62 * 2;
n = 0;
v[n] = 0; n += 1;
v[n] = 1; n += 1;
v[n] = 2; n += 1;
v[n] = 2.5; n += 1;
v[n] = 3; n += 1;
v[n] = 4.75; n += 1;
v[n] = 5; n += 1;
v[n] = -0.25; n += 1;
v[n] = -0.5; n += 1;
v[n] = -1; n += 1;
v[n] = -1.5; n += 1;
v[n] = -2; n += 1;
v[n] = -2.5; n += 1;
v[n] = -3; n += 1;
v[n] = -3.5; n += 1;
v[n] = -4; n += 1;
v[n] = -7.25; n += 1;
v[n] = p31 - 0.5; n += 1;
v[n] = p31; n += 1;
v[n] = p31 + 0.5; n += 1;
v[n] = p31 + 1.5; n += 1;
v[n] = p32 - 0.5; n += 1;
v[n] = p32; n += 1;
v[n] = p32 + 1.5; n += 1;
v[n] = p32 + 2.5; n += 1;
v[n] = p32 * 2 + 1; n += 1;
v[n] = -p31 - 0.5; n += 1;
v[n] = -p32 + 0.5; n += 1;
v[n] = -p32 - 1; n += 1;
v[n] = p52 + 1; n += 1;
v[n] = p52 * 2 + 2; n += 1;
v[n] = p62; n += 1;
v[n] = p62 + 1024; n += 1;
v[n] = p63 - 1024; n += 1;
v[n] = -p63 + 1024; n += 1;
v[n] = -p63; n += 1;
for (i = 0; i < n; i += 1) {
    a.image_index = v[i];
    s_pt = 0; s_rect = 0; s_line = 0; s_self = 0; s_other = 0;
    w = 1;
    for (f = 0; f < 3; f += 1) {
        s_pt += w * (collision_point(100 + px[f], 100 + py[f], oBlack, true, false) != noone);
        s_rect += w * (collision_rectangle(100 + px[f], 100 + py[f], 100 + px[f], 100 + py[f], oBlack, true, false) != noone);
        s_line += w * (collision_line(100 + px[f], 100 + py[f], 100 + px[f], 100 + py[f], oBlack, true, false) != noone);
        with (a) hit = place_meeting(200 - px[f], 60 - py[f], oMenuTop);
        s_self += w * hit;
        a.x = 200 - px[f];
        a.y = 60 - py[f];
        with (b) hit = place_meeting(x, y, oBlack);
        s_other += w * hit;
        a.x = 100;
        a.y = 100;
        w = w * 2;
    }
    out("frame_index_" + string(i), a.image_index);
    out("frame_point_" + string(i), s_pt);
    out("frame_rect_" + string(i), s_rect);
    out("frame_line_" + string(i), s_line);
    out("frame_self_" + string(i), s_self);
    out("frame_other_" + string(i), s_other);
}
