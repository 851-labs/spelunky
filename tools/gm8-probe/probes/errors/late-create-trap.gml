// A trap in the first room's create events stops the game ("Unexpected error occured when running the game."),
// where every other errors/ probe runs. Does one in a create event of a room loaded later, from the main loop,
// stop it too, or does it skip the rest of the load, as a trap in a step skips the rest of the frame
// (late-scope)? oScreen restarts rIntro, whose oBricks calls scrSetupWalls from its create event (a trap here,
// once flagged; the game's version only touches oBrick, which rIntro has none of). After oBricks the room
// places oGlobals, oGame and four oBatIntro. The probe ends in the next begin step, before oGame's step reads
// variables its create event would set. (A step probe.)
global.probe_trap_create = false;
hspeed = 1;
// step:
if (global.probe_step == 2) {
    out("create_trap_x_2", x);
    global.probe_trap_create = true;
    room_restart();
}
if (global.probe_step == 3) {
    out("create_trap_x_3", x);
    out("create_trap_room", room == rIntro);
    out("create_trap_bricks", instance_number(oBricks));
    out("create_trap_globals", instance_number(oGlobals));
    out("create_trap_game", instance_number(oGame));
    out("create_trap_bats", instance_number(oBatIntro));
    with (oIntro) out("create_trap_intro_created", variable_local_exists("fadeLevel"));
    with (oGame) out("create_trap_game_created", variable_local_exists("drawStatus"));
    with (instance_find(oBatIntro, 0)) out("create_trap_bat_created", variable_local_exists("yAcc"));
    out("end", "ok");
    game_end();
}
// script scrSetupWalls:
var p;
if (global.probe_trap_create) {
    global.probe_trap_create = false;
    out("create_trap_in_create", 1);
    p = 1; repeat (64) p = p * 2;
    floor(p);
    out("create_trap_after_trap", 1);
}
