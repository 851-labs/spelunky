// point_distance squares its gaps, so a huge gap overflows on the way to a finite answer.
var p;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
out("pdist_2p1023", point_distance(0, 0, p, 0));
