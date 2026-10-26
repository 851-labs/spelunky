// div0 in a later step: "Division by 0." is GML's own error, not an exception (late-ln0 has a trap that ln()
// reports as one). Does it still show its dialog after the game has started, where an exception the GML doesn't
// catch (late-trap) skips the rest of the frame without one? (A step probe, expected to stop at the dialog.)
// step:
var z;
z = 0;
if (global.probe_step == 3) {
    out("late_div0_before", 1);
    out("late_div0", 1 / z);
    out("end", "ok");
    game_end();
}
