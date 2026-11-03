// A per-object loop and an instance created during it (851-3795). GM8's `with` (Spelunky.exe 0x528510) visits the
// object list's entry at each index below the count it started with, skipping destroyed ones; OpenGMK (`nb_il_iter`)
// stops once its position reaches that count, but each step scans the current list on from the position, so when
// every entry left before the count is destroyed it runs on into instances appended during the loop. `oMenuTop` has
// no events; `oBGEnd3`'s step starts with `isRoom("rEnd3")`, which this probe replaces with a logger that does the
// same destroying and creating from inside the step stage's loop. Each line is the tags the loop visited, in order:
// a: three tagged 1-3; at 1, 2 and 3 are destroyed and a 4th is created.
// b: three tagged 5-7, 7 destroyed before the loop; at 5, an 8th is created.
// c: `with (all)` (the draw order), a 9th tag; at 9, a 10th oMenuTop is created.
// step:
var k, a;
k = global.probe_step;
if (k == 1) {
    global.probe_log = "";
    a = instance_create(-6000, -6000, oMenuTop);
    a.tag = 1;
    global.probe_b = instance_create(-6000, -6000, oMenuTop);
    global.probe_b.tag = 2;
    global.probe_c = instance_create(-6000, -6000, oMenuTop);
    global.probe_c.tag = 3;
    with (oMenuTop) {
        global.probe_log += string(tag) + " ";
        if (tag == 1) {
            with (global.probe_b) instance_destroy();
            with (global.probe_c) instance_destroy();
            a = instance_create(-6000, -6000, oMenuTop);
            a.tag = 4;
        }
    }
    out("with_a", global.probe_log);
    with (oMenuTop) instance_destroy();
}
if (k == 2) {
    global.probe_log = "";
    a = instance_create(-6000, -6000, oMenuTop);
    a.tag = 5;
    a = instance_create(-6000, -6000, oMenuTop);
    a.tag = 6;
    a = instance_create(-6000, -6000, oMenuTop);
    a.tag = 7;
    with (a) instance_destroy();
    with (oMenuTop) {
        global.probe_log += string(tag) + " ";
        if (tag == 5) {
            a = instance_create(-6000, -6000, oMenuTop);
            a.tag = 8;
        }
    }
    out("with_b", global.probe_log);
    with (oMenuTop) instance_destroy();
}
if (k == 3) {
    global.probe_log = "";
    with (all) {
        if (object_index == oMenuTop) global.probe_log += string(tag) + " ";
    }
    a = instance_create(-6000, -6000, oMenuTop);
    a.tag = 9;
    with (all) {
        if (object_index == oMenuTop) {
            global.probe_log += string(tag) + " ";
            if (tag == 9) {
                a = instance_create(-6000, -6000, oMenuTop);
                a.tag = 10;
            }
        }
    }
    out("with_c", global.probe_log);
    with (oMenuTop) instance_destroy();
    // The step stage, case a, for the next step.
    global.probe_log = "";
    a = instance_create(-6000, -6000, oBGEnd3);
    a.tag = 1;
    global.probe_b = instance_create(-6000, -6000, oBGEnd3);
    global.probe_b.tag = 2;
    global.probe_c = instance_create(-6000, -6000, oBGEnd3);
    global.probe_c.tag = 3;
}
if (k == 4) {
    out("step_a", global.probe_log);
    with (oBGEnd3) instance_destroy();
    global.probe_log = "";
    a = instance_create(-6000, -6000, oBGEnd3);
    a.tag = 5;
    a = instance_create(-6000, -6000, oBGEnd3);
    a.tag = 6;
    a = instance_create(-6000, -6000, oBGEnd3);
    a.tag = 7;
    with (a) instance_destroy();
}
if (k == 5) {
    out("step_b", global.probe_log);
    with (oBGEnd3) instance_destroy();
    out("end", "ok");
    game_end();
}
// script isRoom:
var a;
if (object_index == oBGEnd3) {
    global.probe_log += string(tag) + " ";
    if (tag == 1) {
        with (global.probe_b) instance_destroy();
        with (global.probe_c) instance_destroy();
        a = instance_create(-6000, -6000, oBGEnd3);
        a.tag = 4;
    }
    if (tag == 5) {
        a = instance_create(-6000, -6000, oBGEnd3);
        a.tag = 8;
    }
    return true;
}
return room_get_name(room) == argument0;
