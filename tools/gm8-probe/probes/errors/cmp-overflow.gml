// Comparisons subtract (a - b against 1e-13), so comparing two huge values of opposite signs overflows.
var p;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
out("cmp_2p1023_gt_-2p1023", p > -p);
