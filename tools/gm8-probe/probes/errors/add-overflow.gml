// a + b past the largest double: on a real Intel CPU, does the x87's overflow exception stop the game, as
// exp(1000)'s does? (Rosetta doesn't trap: +inf.) The first line is in range.
var p;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
out("add_2p1023_2p1022", p + p / 2);
out("add_2p1023_2p1023", p + p);
