// late-div0 with ln(0): on Intel the x87 traps inside ln(), which reports it as a GML error ("Error in function
// ln()."), where floor(2^64)'s trap (late-trap) reaches the main loop. Does it show its dialog in a later step?
// (A step probe; on Intel this one is expected to stop at a dialog. Rosetta doesn't trap: -inf.)
// step:
var z;
z = 0;
if (global.probe_step == 3) {
    out("late_ln0_before", 1);
    out("late_ln0", ln(z));
    out("end", "ok");
    game_end();
}
