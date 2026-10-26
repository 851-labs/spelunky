// real * string with a count of 19 * 2^24: 318767104 bytes (304 MiB) of "a", at the edge where the 32-bit GM8
// runner under Wine on a real Intel CPU stops building huge strings (mulstr-2p28+1 builds, mulstr-5x2p26
// doesn't; 851-3282).
var n, s;
n = 318767104;
s = n * "a";
out("mulstr_19x2p24_a_len", string_length(s));
