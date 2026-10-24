// Results too small for a double: the x87's underflow exception (masked, unlike overflow?). 2^-1074 / 2
// rounds to 0.
var t;
t = 1; repeat (1074) t = t / 2; // 2^-1074
out("uf_2p-1074", t);
out("uf_2p-1074_div_2", t / 2);
out("uf_2p-1074_sq", t * t);
out("uf_3x2p-1074_div_2", (t * 3) / 2);
out("uf_-2p-1074_div_4", -t / 4);
