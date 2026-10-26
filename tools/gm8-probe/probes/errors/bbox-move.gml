// The step's move stores x + hspeed and y + vspeed through the position setter (Spelunky.exe 0x53e6f0, 0x4abf7c),
// which recomputes the bounding box (bbox-trap). A `core` with sTunnelLeft as its mask moves from 2^62 to 2^63, and a
// second core, after it in the list, 1 pixel right. Are the first one's x stored and its box left as the mask's, and
// the second one not moved? (A step probe.)
global.probe_c = 0;
// step:
var p, c, c2;
p = 1; repeat (63) p = p * 2;
if (global.probe_step == 1) {
    c = instance_create(p / 2, 50, core);
    global.probe_c = c;
    c.mask_index = sTunnelLeft;
    c.hspeed = p / 2;
    c2 = instance_create(0, 0, core);
    global.probe_c2 = c2;
    c2.hspeed = 1;
}
c = global.probe_c;
c2 = global.probe_c2;
if (global.probe_step == 2) {
    out("bbox_move_2", string(c.bbox_left) + " " + string(c.bbox_top) + " " + string(c.bbox_right) + " " + string(c.bbox_bottom));
    out("bbox_move_x_2", c.x);
    out("bbox_move_y_2", c.y);
    out("bbox_move_c2_2", c2.x);
    c.hspeed = 0;
    c.x = 100;
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundLength:
return 1;
