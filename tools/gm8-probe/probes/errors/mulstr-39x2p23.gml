// real * string with a count of 39 * 2^23: 327155712 bytes (312 MiB) of "a", at the edge where the 32-bit GM8
// runner under Wine on a real Intel CPU stops building huge strings (mulstr-2p28+1 builds, mulstr-5x2p26
// doesn't; 851-3282).
var n, s;
n = 327155712;
s = n * "a";
out("mulstr_39x2p23_a_len", string_length(s));
