// At the start of the draw, each visible view with an object to follow (Spelunky.exe 0x4beca4) rounds the target's
// x, then its y, into an Int64 (trapping from 2^63), and keeps the low 32 bits; the rest is integer arithmetic, and
// the view's new position is stored at the end. Views 1 and 2 (100x100, borders of 10) follow a `core` each: view
// 1's at (300, 100), view 2's at x = 2^63 in frame 1 and 2^32 + 300 in frame 2. Does view 1 move and view 2 not, and
// is the rest of the draw skipped (the cores' draw events log through SS_GetSoundPosition)? And does 2^32 + 300
// follow as 300? (A step probe.)
global.probe_log = "";
// step:
var p, k, i, c1, c2;
p = 1; repeat (63) p = p * 2;
k = global.probe_step;
if (k == 1) {
    c1 = instance_create(300, 100, core);
    c2 = instance_create(300, 100, core);
    global.probe_c2 = c2;
    for (i = 1; i <= 2; i += 1) {
        view_visible[i] = true;
        view_xview[i] = 0;
        view_yview[i] = 0;
        view_wview[i] = 100;
        view_hview[i] = 100;
        view_xport[i] = 0;
        view_yport[i] = 0;
        view_wport[i] = 100;
        view_hport[i] = 100;
        view_hborder[i] = 10;
        view_vborder[i] = 10;
        view_hspeed[i] = -1;
        view_vspeed[i] = -1;
    }
    view_object[1] = c1;
    view_object[2] = c2;
    c2.x = p;
}
if (k >= 2) {
    for (i = 1; i <= 2; i += 1) {
        out("view_x" + string(i) + "_" + string(k), view_xview[i]);
        out("view_y" + string(i) + "_" + string(k), view_yview[i]);
    }
    out("view_log_" + string(k), global.probe_log);
}
if (k == 2) {
    global.probe_c2.x = p / 2147483648 + 300;
}
if (k == 3) {
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundPosition:
global.probe_log += "d" + string(global.probe_step);
return 0;
// script SS_GetSoundLength:
return 1;
