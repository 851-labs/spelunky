// a / b past the largest double, by a divisor that isn't tiny (div-tiny has 1 / 2^-1074; see add-overflow).
var p;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
out("div_2p1023_0.5", p / 0.5);
