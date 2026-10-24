// point_distance whose distance itself is past the largest double (pdist-overflow's isn't): the x87 squares
// in a register, but storing the root overflows.
var p;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
out("pdist_1.5x2p1023_1.5x2p1023", point_distance(0, 0, p * 1.5, p * 1.5));
