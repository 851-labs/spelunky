// place_meeting(x, y, obj) moves the instance to (x, y) with the position setter, tests, and moves it back
// (Spelunky.exe 0x4e46fc), each move recomputing the bounding box (bbox-trap). A `core` with sTunnelLeft as its mask:
// from x = 2^63 (where the box traps) to (100, 50), does the move back trap, losing the answer; and from (100, 50) to
// x = 2^63, does the move there trap, leaving the instance there? (A step probe.)
global.probe_c = 0;
// step:
var p, c, k;
p = 1; repeat (63) p = p * 2;
k = global.probe_step;
if (k == 1) {
    c = instance_create(100, 50, core);
    global.probe_c = c;
    c.mask_index = sTunnelLeft;
    c.x = p;
}
c = global.probe_c;
if (k >= 2) {
    out("bbox_place_" + string(k), string(c.bbox_left) + " " + string(c.bbox_top) + " " + string(c.bbox_right) + " " + string(c.bbox_bottom));
    out("bbox_place_x_" + string(k), c.x);
}
if (k == 2) {
    with (c) out("bbox_place_back_2", place_meeting(100, 50, oBlack));
}
if (k == 3) {
    c.x = 100;
    with (c) out("bbox_place_there_3", place_meeting(p, 50, oBlack));
}
if (k == 4) {
    c.x = 100;
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundLength:
return 1;
