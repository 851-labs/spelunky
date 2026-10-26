// The runner's speed after vspeed changes, where speed itself is past the largest double (motion-range's
// isn't): if the squares stay in an x87 register, storing the root overflows.
var p;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
hspeed = p * 1.5;
out("speed_1.5x2p1023_0", speed);
vspeed = p * 1.5;
out("speed_1.5x2p1023_1.5x2p1023", speed);
out("direction_1.5x2p1023_1.5x2p1023", direction);
