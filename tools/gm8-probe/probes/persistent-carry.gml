// Which persistent instances leave a room, and in what order (851-3315). Reading the runner (Spelunky.exe 0x53e270):
// leaving a room, GM8 runs the cleanup (0x4aefb0: the destroyed instances of the active array go), then the room end
// events, then scans the room's array of active instances (0x4ad5e8) for the persistent ones (+0xca) that aren't
// destroyed (+0x108), copying each, in that order, into a new instance with its id (0x4ab45c, 0x4ab670). Deactivated
// instances are in another array, so they don't travel. OpenGMK (`load_room`, `remove_as_vec`) takes every persistent
// instance, deactivated and destroyed ones too, in its slot order, after the room end events, with no cleanup first.
// Persistent oBlacks (no events of their own; tags a to r) in rIntro:
// - Step 1 creates a (depth 5), b (-3), c (5), d (0), e (10), f (1), h (3) and k (2); the frame's draw sorts them.
// - Step 2 deactivates and reactivates c (to the end of the active array, 851-3311), creates g (7, after it),
//   deactivates e, destroys f, destroys k and then deactivates it by id, and restarts the room.
// - oBlack's room end event (object_event_add; step 2's, not game_end's) logs each tag (`re_order`); a logs
//   instance_count and instance_number(oBlack) and creates r (4, persistent); h destroys itself.
// - In the new room, in oBricks' create event during the load (scrSetupWalls), the oBlacks with `with (all)` (the
//   active array: the order they arrived in), every persistent instance there, the oBlacks through `with (oBlack)`
//   (their object's list), and instance_count.
// - In the new room's first begin step (s3), the same: the load's draw has sorted the active array but not the lists.
//   r is destroyed, so that frame's cleanup rebuilds the lists (0x4ad67c, 0x4ad814), from the sorted active array
//   (851-3311): s4. Then instance_activate_object(oBlack), which brings back any that arrived deactivated (act).
global.probe_load = false;
global.probe_re = "";
// step:
var k, t, i, c;
k = global.probe_step;
if (k == 1) {
    object_event_add(oBlack, ev_other, ev_room_end,
        "if (global.probe_step != 2) exit;"
        + "global.probe_re += ptag;"
        + "if (ptag == 'a') {"
        + "    out('re_count', instance_count);"
        + "    out('re_black', instance_number(oBlack));"
        + "    var q;"
        + "    q = instance_create(300, 50, oBlack);"
        + "    q.ptag = 'r';"
        + "    q.depth = 4;"
        + "    q.visible = false;"
        + "    q.persistent = true;"
        + "}"
        + "if (ptag == 'h') instance_destroy();");
    t = "abcdefhk";
    for (i = 1; i <= 8; i += 1) {
        c = instance_create(300, 50, oBlack);
        c.ptag = string_char_at(t, i);
        c.visible = false;
        c.persistent = true;
        global.probe_id[i] = c;
    }
    global.probe_id[1].depth = 5;
    global.probe_id[2].depth = -3;
    global.probe_id[3].depth = 5;
    global.probe_id[4].depth = 0;
    global.probe_id[5].depth = 10;
    global.probe_id[6].depth = 1;
    global.probe_id[7].depth = 3;
    global.probe_id[8].depth = 2;
    out("count_1", instance_count);
}
if (k == 2) {
    SS_IsHandleValid("s2");
    instance_deactivate_object(global.probe_id[3]);
    instance_activate_object(global.probe_id[3]);
    c = instance_create(300, 50, oBlack);
    c.ptag = "g";
    c.depth = 7;
    c.visible = false;
    c.persistent = true;
    instance_deactivate_object(global.probe_id[5]);
    with (global.probe_id[6]) instance_destroy();
    with (global.probe_id[8]) instance_destroy();
    instance_deactivate_object(global.probe_id[8]);
    SS_IsHandleValid("s2b");
    out("count_2", instance_count);
    global.probe_load = true;
    room_restart();
}
if (k == 3) {
    SS_IsHandleValid("s3");
    out("count_3", instance_count);
    with (oBlack) if (ptag == "r") instance_destroy();
}
if (k == 4) {
    SS_IsHandleValid("s4");
    instance_activate_object(oBlack);
    SS_IsHandleValid("act");
    out("count_act", instance_count);
    out("end", "ok");
    game_end();
}
// script scrSetupWalls:
if (global.probe_load) {
    global.probe_load = false;
    out("re_order", global.probe_re);
    SS_IsHandleValid("load");
    out("count_load", instance_count);
}
// script SS_IsHandleValid:
// The oBlacks in the active array's order (`with (all)`), then every persistent instance in it, then the oBlacks in
// their object's list's order (`with (oBlack)`).
var s, p, w;
s = "";
p = "";
w = "";
with (all) {
    if (object_index == oBlack) s += ptag;
    if (persistent) {
        if (object_index == oBlack) p += ptag + " ";
        else p += object_get_name(object_index) + " ";
    }
}
out(argument0 + "_order", s);
out(argument0 + "_persistent", p);
with (oBlack) w += ptag;
out(argument0 + "_with", w);
