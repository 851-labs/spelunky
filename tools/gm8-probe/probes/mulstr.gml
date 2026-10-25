// real * string repeats the string: how is the count rounded, and what does a negative one do? Counts past
// 32 bits are in errors/mulstr-*, one each, as a 64-bit count would build gigabytes. Nothing here should error.
var n, p32;
p32 = 4294967296;
out("mulstr_3_ab", 3 * "ab");
out("mulstr_2.5_a", 2.5 * "a");
out("mulstr_3.5_a", 3.5 * "a");
out("mulstr_0.5_a", 0.5 * "a");
out("mulstr_0.6_a", 0.6 * "a");
n = -3;
out("mulstr_-3_a", n * "a");
out("mulstr_1_empty", 1 * "");
out("mulstr_-2p32+3_a", (-p32 + 3) * "a");
