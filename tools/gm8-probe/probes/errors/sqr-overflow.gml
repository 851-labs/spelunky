// sqr(x) past the largest double (see add-overflow).
var p;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
out("sqr_2p1023", sqr(p));
