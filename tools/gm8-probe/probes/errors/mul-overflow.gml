// a * b past the largest double, 1e308 * 10 in powers of two (see add-overflow).
var p;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
out("mul_2p1023_2", p * 2);
