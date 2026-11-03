// How far GM8 tiles a background layer through a scaled view (851-3967; Spelunky.exe's tiled draw, 0x4a459c, which
// `gm::gm8::draw_background` describes). backgroundClouds (32x32, opaque, no black pixel) is tiled through a 64x64
// view drawn at 2x onto black. The layer's right and bottom edges end exactly at the view's, so whether the last
// column and row of pixels are lit says whether GM8 draws a tile past the view's edge, which the half-pixel shift (a
// whole pixel at 2x) would need to cover them. It's drawn into a 128x128 surface, which the view is stretched over,
// as oScreen draws and as the settings program's window is. (To the screen, a port set after the room started
// doesn't scale the window's picture, so that wouldn't test it.) Each scan is a row or column's first 132 pixels:
// "x" lit, "." black, "r" the surface's red, which the room's black clear covers.
var b, s, i;
b = backgroundClouds;
instance_deactivate_all(true);
visible = false;
for (i = 0; i < 8; i += 1) {
    background_visible[i] = false;
    view_visible[i] = false;
}
background_index[0] = b;
background_visible[0] = true;
background_foreground[0] = false;
background_htiled[0] = true;
background_vtiled[0] = true;
background_x[0] = 0;
background_y[0] = 0;
background_xscale[0] = 1;
background_yscale[0] = 1;
background_hspeed[0] = 0;
background_vspeed[0] = 0;
background_showcolor = true;
background_color = c_black;
view_enabled = true;
view_visible[0] = true;
view_xview[0] = 0;
view_yview[0] = 0;
view_wview[0] = 64;
view_hview[0] = 64;
view_xport[0] = 0;
view_yport[0] = 0;
view_wport[0] = 128;
view_hport[0] = 128;
view_angle[0] = 0;
view_object[0] = noone;
out("background_w", background_get_width(b));
out("background_h", background_get_height(b));
s = surface_create(128, 128);
surface_set_target(s);
draw_clear(c_red);
screen_redraw();
surface_reset_target();
out("row10", SS_GetSoundVol(s, 1, 10));
out("col10", SS_GetSoundVol(s, 0, 10));
out("row126", SS_GetSoundVol(s, 1, 126));
out("row127", SS_GetSoundVol(s, 1, 127));
// script SS_GetSoundVol:
// scan(surf, along x?, at): the first 132 pixels of a row (along x) or column of the surface.
var i, c, t;
t = "";
for (i = 0; i < 132; i += 1) {
    if (argument1) c = surface_getpixel(argument0, i, argument2) else c = surface_getpixel(argument0, argument2, i);
    if (c == c_black) t += "." else if (c == c_red) t += "r" else t += "x";
}
return t;
