// a *= 2 past the largest double, on a local (see add-overflow and orassign-local).
var p, a;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
a = p;
a *= 2;
out("mulassign_overflow_local", a);
