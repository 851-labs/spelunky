// What GM8 draws (851-3308; Spelunky.exe, read only to describe it) for a character a sprite font lacks, which
// its character draw (0x5193a8) skips while the line's loop still advances by the space's width (0x518ee4), and for
// `draw_surface_stretched` with a width or height of 0 or less, which its scaled draw (0x4a10c0) skips. Each case
// draws on a cleared black surface and reads it back: a text case gives one character per column, "x" where a pixel
// isn't black (`SS_GetSoundVol`); a surface case the lit pixels' count and bounds (`SS_SetSoundVol`). sFont has 59
// 16x16 frames from ' ' (to 'Z'), the space blank.
var f, fs, f31, f36, s, src;
f = font_add_sprite(sFont, ord(" "), false, 0);
fs = font_add_sprite(sFont, ord(" "), false, 4);
// From chr(31), so the space is the '!' frame, which shows whether a missing character draws the space; from '$',
// with no space nor '#'.
f31 = font_add_sprite(sFont, 31, false, 0);
f36 = font_add_sprite(sFont, ord("$"), false, 0);
out("font_frames", sprite_get_number(sFont));
out("font_w", sprite_get_width(sFont));
out("font_h", sprite_get_height(sFont));
s = surface_create(96, 16);
draw_set_color(c_white);
draw_set_alpha(1);
draw_set_font(f);
out("text_AB", SS_GetSoundVol(s, "AB"));
out("text_A_B", SS_GetSoundVol(s, "A B"));
out("text_A200B", SS_GetSoundVol(s, "A" + chr(200) + "B"));
out("text_A31B", SS_GetSoundVol(s, "A" + chr(31) + "B"));
out("text_A[B", SS_GetSoundVol(s, "A[B"));
out("text_A[[B", SS_GetSoundVol(s, "A[[B"));
out("text_A#B", SS_GetSoundVol(s, "A\#B"));
out("text_200", SS_GetSoundVol(s, chr(200)));
draw_set_font(fs);
out("sep4_A_B", SS_GetSoundVol(s, "A B"));
out("sep4_A200B", SS_GetSoundVol(s, "A" + chr(200) + "B"));
draw_set_font(f31);
out("from31_A_B", SS_GetSoundVol(s, "A B"));
out("from31_AB", SS_GetSoundVol(s, "AB"));
out("from31_A200B", SS_GetSoundVol(s, "A" + chr(200) + "B"));
out("from31_A[B", SS_GetSoundVol(s, "A[B"));
draw_set_font(f36);
out("from36_A_B", SS_GetSoundVol(s, "A B"));
out("from36_A#B", SS_GetSoundVol(s, "A\#B"));
out("from36_A200B", SS_GetSoundVol(s, "A" + chr(200) + "B"));
draw_set_font(-1);
src = surface_create(4, 4);
surface_set_target(src);
draw_clear(c_white);
surface_reset_target();
s = surface_create(32, 32);
out("surface_plain", SS_SetSoundVol(s, src, 0, 0));
out("stretched_4_4", SS_SetSoundVol(s, src, 4, 4));
out("stretched_8_2", SS_SetSoundVol(s, src, 8, 2));
out("stretched_-4_4", SS_SetSoundVol(s, src, -4, 4));
out("stretched_4_-4", SS_SetSoundVol(s, src, 4, -4));
out("stretched_-4_-4", SS_SetSoundVol(s, src, -4, -4));
out("stretched_0_4", SS_SetSoundVol(s, src, 0, 4));
out("stretched_4_0", SS_SetSoundVol(s, src, 4, 0));
out("stretched_-0_4", SS_SetSoundVol(s, src, -0, 4));
// script SS_GetSoundVol:
// text(surf, string): the columns `draw_text(0, 0, string)` lights.
var s, i, j, lit;
surface_set_target(argument0);
draw_clear(c_black);
draw_text(0, 0, argument1);
surface_reset_target();
s = "";
for (i = 0; i < surface_get_width(argument0); i += 1) {
    lit = false;
    for (j = 0; j < surface_get_height(argument0); j += 1) {
        if (surface_getpixel(argument0, i, j) != 0) lit = true;
    }
    if (lit) s += "x"; else s += ".";
}
return s;
// script SS_SetSoundVol:
// stretched(surf, src, w, h): the pixels `draw_surface_stretched(src, 8, 8, w, h)` lights (or, for a width of 0
// and a height of 0, `draw_surface(src, 8, 8)`'s), as "n l t r b".
var n, l, t, r, b, i, j;
surface_set_target(argument0);
draw_clear(c_black);
if (argument2 == 0 && argument3 == 0) draw_surface(argument1, 8, 8);
else draw_surface_stretched(argument1, 8, 8, argument2, argument3);
surface_reset_target();
n = 0; l = -1; t = -1; r = -1; b = -1;
for (i = 0; i < surface_get_width(argument0); i += 1) {
    for (j = 0; j < surface_get_height(argument0); j += 1) {
        if (surface_getpixel(argument0, i, j) != 0) {
            n += 1;
            if (l < 0) l = i;
            r = i;
            if (t < 0 || j < t) t = j;
            if (j > b) b = j;
        }
    }
}
return string(n) + " " + string(l) + " " + string(t) + " " + string(r) + " " + string(b);
