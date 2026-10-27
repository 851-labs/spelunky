// View following by instance id (851-3309). GM8's routine (Spelunky.exe 0x4beca4) scans the room's instance array for
// the first instance with the id that isn't marked destroyed; deactivated instances are in another array. OpenGMK
// (and the port) take the first instance with the id, then follow it only if it's active. The two differ only while
// two instances share an id: a persistent instance arriving in a room that places one with its id, which the load
// marks destroyed until the frame's cleanup. oGlobals is persistent and placed in rIntro, so a room_restart() there
// makes that pair; the load ends with a draw, and its views follow.
// 1. In rIntro, views 1-3 (100x100 at x = 500, border 10, speed -1) each follow an oBlack at (300, 50): one alive,
//    one destroyed, one deactivated, through screen_redraw() before the cleanup: 290, 500, 500 (no follow).
// 2. Views 4 and 5 of rIntro, set with room_set_view, follow oGlobals (moved to (300, 50)) by id and by object; then
//    room_restart(). In the new room's first begin step: each view after the load's draw, then after a screen_redraw()
//    with the views put back at 500 (the placed oGlobals still there, marked), then a frame later. By object, every
//    one follows (290); by id, GM8 skips the marked one where OpenGMK stops at it (500).
// step:
var k, i, w, g;
k = global.probe_step;
if (k == 1) {
    for (i = 1; i <= 3; i += 1) {
        view_visible[i] = true;
        view_xview[i] = 500;
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
        w = instance_create(300, 50, oBlack);
        w.visible = false;
        view_object[i] = w;
        if (i == 2) with (w) instance_destroy();
        if (i == 3) instance_deactivate_object(w);
    }
    screen_redraw();
    for (i = 1; i <= 3; i += 1) out("id_redraw_x" + string(i), view_xview[i]);
    out("id_globals", instance_number(oGlobals));
    g = instance_find(oGlobals, 0);
    g.x = 300;
    g.y = 50;
    room_set_view(rIntro, 4, true, 500, 0, 100, 100, 0, 0, 100, 100, 10, 10, -1, -1, g);
    room_set_view(rIntro, 5, true, 500, 0, 100, 100, 0, 0, 100, 100, 10, 10, -1, -1, oGlobals);
    room_restart();
}
if (k == 2) {
    out("id_globals_2", instance_number(oGlobals));
    for (i = 4; i <= 5; i += 1) {
        out("id_load_x" + string(i), view_xview[i]);
        view_xview[i] = 500;
    }
    screen_redraw();
    for (i = 4; i <= 5; i += 1) out("id_redraw_x" + string(i), view_xview[i]);
}
if (k == 3) {
    for (i = 4; i <= 5; i += 1) out("id_next_x" + string(i), view_xview[i]);
    out("end", "ok");
    game_end();
}
