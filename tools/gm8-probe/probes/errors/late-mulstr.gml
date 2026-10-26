// mulstr-39x2p23 in a later step: GM8 running out of memory stops the game with the same dialog as the x87's traps
// in the first room's create events. Is it the same exception later on, which skips the rest of the frame (oScreen
// doesn't move) and lets the game go on, as late-trap's floor(2^64) does? The counts are computed when the step
// runs: GM8 folds `(39 * 8388608) * "a"` when it compiles the script, and so runs out of memory while it loads.
// (Whether a string as long as GM8 built at game start, 304 MiB, still fits afterwards depends on the address
// space left: on Intel it didn't, also without a dialog.) (A step probe.)
hspeed = 1;
// step:
var n, s;
n = 8388608;
if (global.probe_step == 3) {
    out("late_mulstr_x_3", x);
    s = (39 * n) * "a";
    out("late_mulstr_len", string_length(s));
}
if (global.probe_step == 4) {
    out("late_mulstr_x_4", x);
    out("end", "ok");
    game_end();
}
