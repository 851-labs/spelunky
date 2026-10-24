// x / tiny: does the division-by-zero check use the comparison tolerance, or exact zero? Each line only runs if
// the one before didn't stop the game.
var e;
e = 1; repeat (14) e /= 10;
out("div_1_1e-14", 1 / e);
e = 1; repeat (300) e /= 10;
out("div_1_1e-300", 1 / e);
e = 1; repeat (1074) e /= 2;
out("div_1_2^-1074", 1 / e);
