// radtodeg(x) is x * 180 / pi: past the largest double for a huge x.
var p;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
out("radtodeg_2p1023", radtodeg(p));
