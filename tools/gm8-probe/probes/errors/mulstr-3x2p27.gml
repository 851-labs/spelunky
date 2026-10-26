// real * string with a count of 3 * 2^27: 402653184 bytes of "a", between mulstr-2p28 (which GM8 builds) and
// mulstr-2p29 (which stops it) (851-3282).
var n, s;
n = 402653184;
s = n * "a";
out("mulstr_3x2p27_a_len", string_length(s));
