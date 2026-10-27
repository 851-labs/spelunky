// Lookups by instance id while two instances share the id (851-3310). A persistent instance arriving in a room that
// placed one with its id marks that one destroyed until the frame's cleanup, and comes after it in the room's
// instance array. Reading the runner (Spelunky.exe), its lookups treat the pair in three ways:
// - instance_exists, instance_number, instance_find(id, 0) and the collision tests take the first instance with the
//   id that isn't destroyed: the persistent one.
// - `with (id)` and distance_to_object(id) take the first with the id, and do nothing if it's destroyed.
// - Variable reads and writes through an id take the last with the id, unless one of two cached array positions,
//   shared by reads and writes, holds an instance with it (for a read, one that isn't destroyed). A destroyed one
//   is then skipped without an error for a write.
// instance_deactivate_object(id) deactivates every instance with the id, destroyed ones too.
// OpenGMK let the first with the id decide everywhere; the port and patch 0030 follow GM8 but for the cache and the
// destroyed one's deactivation.
// oGlobals is persistent and placed in rIntro, so a room_restart() there makes the pair. Step 1 moves the
// persistent oGlobals to (300, 50) and gives it sBlack (16x16), which the placed one doesn't have, then
// restarts the room. The lookups (SS_IsHandleValid below) run twice: in oBricks' create event during the load
// (prefix "c": the placed oGlobals is already marked) and in the next begin step ("s"). Each run first reads x
// through two other ids, so that neither cached position is the placed oGlobals'.
global.probe_lookups = false;
// step:
var g;
if (global.probe_step == 1) {
    g = instance_find(oGlobals, 0);
    global.probe_g = g;
    g.x = 300;
    g.y = 50;
    g.sprite_index = sBlack;
    global.probe_lookups = true;
    room_restart();
}
if (global.probe_step == 2) {
    SS_IsHandleValid("s");
    out("end", "ok");
    game_end();
}
// script scrSetupWalls:
if (global.probe_lookups) {
    global.probe_lookups = false;
    SS_IsHandleValid("c");
}
// script SS_IsHandleValid:
var p, g, k, v, n;
p = argument0;
g = global.probe_g;
// By object (which skips the destroyed one), the persistent oGlobals back where step 1 put it.
with (oGlobals) {
    x = 300;
    y = 50;
}
k = id;
v = k.x;
k = instance_find(oIntro, 0);
v = k.x;
out(p + "_globals", instance_number(oGlobals));
out(p + "_exists", instance_exists(g));
out(p + "_number", instance_number(g));
out(p + "_find", instance_find(g, 0));
n = 0;
with (g) n += 1;
out(p + "_with", n);
out(p + "_distance", distance_to_object(g));
out(p + "_point", collision_point(304, 54, g, false, false));
out(p + "_rectangle", collision_rectangle(296, 46, 304, 54, g, false, false));
out(p + "_line", collision_line(290, 54, 310, 54, g, false, false));
out(p + "_position", instance_position(304, 54, g));
out(p + "_position_meeting", position_meeting(304, 54, g));
sprite_index = sBlack;
out(p + "_place", instance_place(296, 46, g));
out(p + "_place_meeting", place_meeting(296, 46, g));
sprite_index = -1;
out(p + "_read", g.x);
g.x = 310;
with (oGlobals) out(p + "_write", x);
// The cache. With the persistent oGlobals deactivated alone, a write's scan finds the placed one, caches its
// position and writes nothing. Once the persistent one is back, a write takes the cached, destroyed one and again
// writes nothing; a read skips it.
with (oGlobals) instance_deactivate_object(self);
out(p + "_alone_exists", instance_exists(g));
g.x = 5;
instance_activate_object(g);
out(p + "_back_exists", instance_exists(g));
g.x = 7;
with (oGlobals) out(p + "_cached_write", x);
out(p + "_cached_read", g.x);
// Deactivating by id takes both, the destroyed one leaving the count.
instance_deactivate_object(g);
out(p + "_deact_exists", instance_exists(g));
out(p + "_deact_globals", instance_number(oGlobals));
out(p + "_deact_count", instance_count);
instance_activate_object(g);
out(p + "_act_exists", instance_exists(g));
out(p + "_act_globals", instance_number(oGlobals));
out(p + "_act_count", instance_count);
