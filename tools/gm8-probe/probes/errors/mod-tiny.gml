// mod by a tiny or negative-zero divisor
var e, z;
out("mod_7_0.5", 7 mod 0.5);
e = 1; repeat (14) e /= 10;
out("mod_7_1e-14", 7 mod e);
z = 0 * -1;
out("mod_7_-0", 7 mod z);
