// string_format of big and tiny reals, where Delphi's FloatToDecimal scales by powers of ten past 10^22 (and
// past 2^63, where string() traps on Intel). Nothing here should error.
var p, p63, t, v;
p63 = 4294967296 * 2147483648;
p = 1; repeat (1023) p = p * 2;
t = 1; repeat (1074) t = t / 2;
v = 1; repeat (25) v = v * 10;
out("sfb_2p63_0_2", "[" + string_format(p63, 0, 2) + "]");
out("sfb_-2p63_0_0", "[" + string_format(-p63, 0, 0) + "]");
out("sfb_1e25_0_2", "[" + string_format(v, 0, 2) + "]");
out("sfb_1.5e25_0_2", "[" + string_format(v * 1.5, 0, 2) + "]");
out("sfb_2p1023_0_2", "[" + string_format(p, 0, 2) + "]");
out("sfb_2p-1074_0_18", "[" + string_format(t, 0, 18) + "]");
out("sfb_1e-25_0_18", "[" + string_format(1 / v, 0, 18) + "]");
