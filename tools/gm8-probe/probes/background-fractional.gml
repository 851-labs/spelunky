// Where GM8 puts a tiled background layer's copies at a fractional position or scale (851-3970; its tiled draw,
// Spelunky.exe 0x4a459c, which `gm::gm8::draw_background` describes, rounds `w * xscale`, `h * yscale`, `x` and
// `y` before it steps the columns and rows). A 16 x 16 background made from a surface, gray with a white first column
// and first row, is tiled through a 128 x 128 view at 1x into a 128 x 128 surface, black behind it, at four settings
// (`case_N`: x, y, xscale, yscale). For each, row 8 and column 8 of the surface ("w" white, "g" gray, "." black, "r"
// the surface's red): where each copy's white line falls says where the copy starts.
var t, b, s, k, bx, by, bxs, bys;
t = surface_create(16, 16);
surface_set_target(t);
draw_clear(c_gray);
draw_set_color(c_white);
for (k = 0; k < 16; k += 1) {
    draw_point(0, k);
    draw_point(k, 0);
}
surface_reset_target();
b = background_create_from_surface(t, 0, 0, 16, 16, false, false);
instance_deactivate_all(true);
visible = false;
for (k = 0; k < 8; k += 1) {
    background_visible[k] = false;
    view_visible[k] = false;
}
background_index[0] = b;
background_visible[0] = true;
background_foreground[0] = false;
background_htiled[0] = true;
background_vtiled[0] = true;
background_hspeed[0] = 0;
background_vspeed[0] = 0;
background_showcolor = true;
background_color = c_black;
view_enabled = true;
view_visible[0] = true;
view_xview[0] = 0;
view_yview[0] = 0;
view_wview[0] = 128;
view_hview[0] = 128;
view_xport[0] = 0;
view_yport[0] = 0;
view_wport[0] = 128;
view_hport[0] = 128;
view_angle[0] = 0;
view_object[0] = noone;
out("tile_row0", SS_GetSoundVol(t, 1, 0));
out("tile_row5", SS_GetSoundVol(t, 1, 5));
s = surface_create(128, 128);
for (k = 0; k < 4; k += 1) {
    if (k == 0) { bx = 3.6; by = 2.4; bxs = 1; bys = 1; }
    if (k == 1) { bx = 0; by = 0; bxs = 1.3; bys = 1.3; }
    if (k == 2) { bx = 3.6; by = 2.5; bxs = 1.3; bys = 0.7; }
    if (k == 3) { bx = -5.5; by = -9.4; bxs = 1.55; bys = 1.45; }
    background_x[0] = bx;
    background_y[0] = by;
    background_xscale[0] = bxs;
    background_yscale[0] = bys;
    surface_set_target(s);
    draw_clear(c_red);
    screen_redraw();
    surface_reset_target();
    out("case_" + string(k), string(bx) + " " + string(by) + " " + string(bxs) + " " + string(bys));
    out("row8_" + string(k), SS_GetSoundVol(s, 1, 8));
    out("col8_" + string(k), SS_GetSoundVol(s, 0, 8));
}
// script SS_GetSoundVol:
// scan(surf, along x?, at): the first 128 pixels of a row (along x) or column of the surface.
var i, c, t, n;
t = "";
n = surface_get_width(argument0);
for (i = 0; i < n; i += 1) {
    if (argument1) c = surface_getpixel(argument0, i, argument2) else c = surface_getpixel(argument0, argument2, i);
    if (c == c_black) t += "." else if (c == c_red) t += "r" else if (c == c_white) t += "w" else if (c == c_gray) t += "g" else t += "?";
}
return t;
