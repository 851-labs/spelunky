// real * string with a count of 5 * 2^26: 335544320 bytes of "a", between mulstr-2p28 (which GM8 builds) and
// mulstr-3x2p27 (which stops it) (851-3282).
var n, s;
n = 335544320;
s = n * "a";
out("mulstr_5x2p26_a_len", string_length(s));
