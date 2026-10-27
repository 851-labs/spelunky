// (De)activation and instance order (851-3311). GM8 keeps a room's instances in two arrays, the active one (destroyed
// instances too, until the cleanup) and the inactive one. Reading the runner (Spelunky.exe): the (de)activation
// built-ins only flag instances (+0x109), then 0x4af00c moves the unflagged ones out of the inactive array to the end of
// the active one, and the flagged ones out of the active array to the end of the inactive one, each keeping its order,
// and if anything moved, rebuilds the per-object lists from the active array (0x4ad67c, 0x4ad814), destroyed instances
// included. The depth sort before a draw (0x4aef24, an insertion sort) reorders the active array alone, not the lists;
// the cleanup (0x4aefb0) only removes the destroyed instances of the active array. And the built-ins flag destroyed
// instances like any other. OpenGMK flips a state in place in one array, keeps the inactive instances in the depth sort,
// and rebuilds the lists (without destroyed instances) after every `self`, `other` and `all` call.
// Four `core`s (tags 1-4, depth 5, no sprite) in rIntro, each line their order: through `with (core)` (the lists), through
// instance_find(core, n), through `with (all)` (the active array), and their draw events (SS_GetSoundPosition), each
// line named `order_` and then:
// a: as created. b: after deactivating and reactivating 2. c: after deactivating 4, 1 and 3 in three calls, then
// reactivating `core`. d: after 3's depth goes to 9 and a screen_redraw() (its draws). e: after instance_activate_all()
// with nothing deactivated. f: after 2 deactivates the region holding it and 4 with notme, then activate_all.
// g: a fifth core (8), destroyed, then instance_deactivate_object(core) and instance_activate_object(core), with
// instance_count after each. And an oMenuTop (7, no events) destroyed and then deactivated by id: instance_count over
// the next steps, after it's reactivated in step 2, and the draws of step 2's frame.
// step:
var k, i, c, m;
k = global.probe_step;
if (k == 1) {
    out("order_a_cores", instance_number(core));
    for (i = 1; i <= 4; i += 1) {
        c = instance_create(-5000 - 10 * i, -5000, core);
        c.tag = i;
        c.depth = 5;
        global.probe_c[i] = c;
    }
    global.probe_c[4].x = global.probe_c[2].x;
    SS_IsHandleValid("a");
    instance_deactivate_object(global.probe_c[2]);
    instance_activate_object(global.probe_c[2]);
    SS_IsHandleValid("b");
    instance_deactivate_object(global.probe_c[4]);
    instance_deactivate_object(global.probe_c[1]);
    instance_deactivate_object(global.probe_c[3]);
    instance_activate_object(core);
    SS_IsHandleValid("c");
    global.probe_c[3].depth = 9;
    global.probe_log = "";
    screen_redraw();
    out("order_d_draw", global.probe_log);
    SS_IsHandleValid("d");
    instance_activate_all();
    SS_IsHandleValid("e");
    with (global.probe_c[2]) instance_deactivate_region(x - 1, y - 1, 2, 2, true, true);
    out("order_f_c4_exists", instance_exists(global.probe_c[4]));
    instance_activate_all();
    SS_IsHandleValid("f");
    // A destroyed oMenuTop, deactivated by id.
    m = instance_count;
    c = instance_create(-6000, -6000, oMenuTop);
    global.probe_c[7] = c;
    out("order_g7_create", instance_count - m);
    with (c) instance_destroy();
    out("order_g7_destroy", instance_count - m);
    instance_deactivate_object(c);
    out("order_g7_deactivate", instance_count - m);
    // A destroyed core, deactivated by object.
    c = instance_create(-6000, -6000, core);
    c.tag = 8;
    c.depth = 5;
    global.probe_c[8] = c;
    m = instance_count;
    with (c) instance_destroy();
    instance_deactivate_object(core);
    out("order_g_deactivate", instance_count - m);
    instance_activate_object(core);
    out("order_g_activate", instance_count - m);
    SS_IsHandleValid("g");
    global.probe_m = m;
}
if (k == 2) {
    m = global.probe_m;
    out("order_h_count", instance_count - m);
    out("order_h7_exists", instance_exists(global.probe_c[7]));
    instance_activate_object(oMenuTop);
    out("order_h7_activate", instance_count - m);
    out("order_h7_exists_after", instance_exists(global.probe_c[7]));
    out("order_h7_number", instance_number(oMenuTop));
    SS_IsHandleValid("h");
    global.probe_log = "";
}
if (k == 3) {
    out("order_i_draw", global.probe_log);
    out("order_i_count", instance_count - global.probe_m);
    SS_IsHandleValid("i");
    out("end", "ok");
    game_end();
}
// script SS_IsHandleValid:
var p, s, n, i, c;
p = "order_" + argument0;
s = "";
with (core) s += string(tag);
out(p + "_with", s);
s = "";
n = instance_number(core);
for (i = 0; i < n; i += 1) {
    c = instance_find(core, i);
    s += string(c.tag);
}
out(p + "_find", s);
s = "";
with (all) if (object_index == core) s += string(tag);
out(p + "_all", s);
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundPosition:
global.probe_log += string(tag);
return 0;
// script SS_GetSoundLength:
return 1;
