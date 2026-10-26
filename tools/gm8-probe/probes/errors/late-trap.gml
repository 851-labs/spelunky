// late-speedwrite-2p63: a trap in a later step's GML doesn't stop the game with a dialog, as in the first create
// event; the game goes on. Does the rest of that step run? oScreen moves 1 pixel a step: if the trap skips the
// rest of the frame, it doesn't move in the step whose begin step trapped. (A step probe.)
hspeed = 1;
// step:
var p;
p = 1; repeat (64) p = p * 2;
if (global.probe_step == 2) {
    out("late_trap_x_2", x);
}
if (global.probe_step == 3) {
    out("late_trap_x_3", x);
    out("late_trap_floor_2p64", floor(p));
}
if (global.probe_step == 4) {
    out("late_trap_x_4", x);
    out("end", "ok");
    game_end();
}
