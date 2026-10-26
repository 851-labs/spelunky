// lengthdir-2p63 from below: the snap's Int64 holds -2^63 but not the next double down.
var p;
p = 1; repeat (63) p = p * 2;
out("lengthdir_x_-2p63_0", lengthdir_x(-p, 0));
out("lengthdir_x_-2p63-2048_0", lengthdir_x(-p - 2048, 0));
