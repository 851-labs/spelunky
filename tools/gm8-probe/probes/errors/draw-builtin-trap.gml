// The draw built-ins' own x87 conversions (851-3304; Spelunky.exe, read only to describe it). `draw_sprite` takes
// Delphi's Floor of image_index for a negative subimg (0x554b29); the texture draws store each corner as a single
// (0x4a360c), which traps from 2^128 - 2^103 (T here; T - 2^75 is the largest double below it), and the scaled one
// first stores -xorig * xscale - 0.5 and w * xscale + that as doubles (0x4a3ad8); `draw_text` rounds a real into an
// Int64 to format it (0x55433f); `draw_rectangle` takes Floor of its larger x and y (0x49d2ea); `draw_circle` stores
// x - r .. y + r as doubles (0x49d847) and its centre and rim as singles (0x49d6ec). Case k runs in `core`'s draw
// event (`SS_GetSoundPosition`) in frame k, between an "a" and a "b" in the log: a "b" missing means it trapped.
// sMetalBlock is 16x16 with its origin at 0, 0; sFont 59 frames from ' ' (to 'Z'). Cases 30 to 37 find which
// characters the sprite font and GM's built-in one (font -1) draw. (A step probe.)
var t, u;
t = 1; repeat (103) t = t * 2;
global.probe_t = t * 33554431;
u = 1; repeat (75) u = u * 2;
global.probe_tb = global.probe_t - u;
global.probe_case = 0;
global.probe_log = "";
global.probe_c = 0;
global.probe_font = -1;
global.probe_surf = -1;
// step:
var k, p;
p = 1; repeat (63) p = p * 2;
k = global.probe_step;
if (k == 1) {
    global.probe_c = instance_create(0, 0, core);
    global.probe_c.image_speed = 0;
    global.probe_font = font_add_sprite(sFont, ord(" "), false, 0);
    global.probe_surf = surface_create(16, 16);
}
if (k >= 2) {
    out("draw_" + string(k - 1), global.probe_log);
    global.probe_log = "";
}
global.probe_c.image_index = 0;
if (k == 1) global.probe_c.image_index = p;
if (k == 2) global.probe_c.image_index = p / 2;
global.probe_case = k;
if (k == 38) {
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundLength:
return 1;
// script SS_GetSoundPosition:
var k, p, t, tb, big, s;
p = 1; repeat (63) p = p * 2;
big = 1; repeat (1023) big = big * 2;
s = 1; repeat (123) s = s * 2;
s = s * 1.5;
t = global.probe_t;
tb = global.probe_tb;
k = global.probe_case;
global.probe_log += "a";
switch (k) {
    case 1: draw_sprite(sMetalBlock, -1, 0, 0); break;
    case 2: draw_sprite(sMetalBlock, -1, 0, 0); break;
    case 3: draw_sprite(sMetalBlock, 0, t, 0); break;
    case 4: draw_sprite(sMetalBlock, 0, tb, 0); break;
    case 5: draw_sprite(sMetalBlock, 0, 0, -t); break;
    case 6: draw_sprite_ext(sMetalBlock, 0, 0, 0, big, 1, 0, c_white, 1); break;
    case 7: draw_sprite_ext(sMetalBlock, 0, 0, 0, s, s, 0, c_white, 1); break;
    case 8: draw_sprite_ext(sMetalBlock, 0, 0, 0, s, s, 45, c_white, 1); break;
    case 9: draw_text(0, 0, p); break;
    case 10: draw_text(0, 0, p / 2); break;
    case 11: draw_set_font(global.probe_font); draw_text(t, 0, "A"); break;
    case 12: draw_set_font(global.probe_font); draw_text(t, 0, ""); break;
    case 13: draw_set_font(global.probe_font); draw_text(tb, 0, "A"); break;
    case 14: draw_set_font(global.probe_font); draw_text(t, 0, chr(200)); break;
    case 15: draw_set_font(-1); draw_text(t, 0, "A"); break;
    case 16: draw_set_font(-1); draw_text(t, 0, chr(200)); break;
    case 17: draw_rectangle(0, 0, p, 10, false); break;
    case 18: draw_rectangle(p, 0, 0, 10, false); break;
    case 19: draw_rectangle(0, 0, p / 2, 10, false); break;
    case 20: draw_rectangle(-t, 0, 0, 10, false); break;
    case 21: draw_rectangle(0, -2 * p, 10, -2 * p, false); break;
    case 22: draw_circle(t, 0, 1, false); break;
    case 23: draw_circle(0, 0, t, false); break;
    case 24: draw_circle(0, 0, tb, false); break;
    case 25: draw_circle(big * 1.5, 0, big * 1.5, false); break;
    case 26: draw_surface(global.probe_surf, t, 0); break;
    case 27: draw_surface(global.probe_surf, tb, 0); break;
    case 28: draw_surface_stretched(global.probe_surf, 0, 0, 2 * t, 10); break;
    case 29: draw_surface_stretched(global.probe_surf, 0, 0, -2 * t, 10); break;
    case 30: draw_set_font(global.probe_font); draw_text(t, 0, chr(31)); break;
    case 31: draw_set_font(global.probe_font); draw_text(t, 0, "Z"); break;
    case 32: draw_set_font(global.probe_font); draw_text(t, 0, "["); break;
    case 33: draw_set_font(global.probe_font); draw_text(t, 0, "#"); break;
    case 34: draw_set_font(-1); draw_text(t, 0, chr(31)); break;
    case 35: draw_set_font(-1); draw_text(t, 0, chr(32)); break;
    case 36: draw_set_font(-1); draw_text(t, 0, chr(127)); break;
    case 37: draw_set_font(-1); draw_text(t, 0, chr(128)); break;
}
global.probe_log += "b";
return 0;
