// real * string with a count of 2^28 + 1: one byte past mulstr-2p28, which GM8 builds. If it fails, as
// mulstr-5x2p26 does, the limit is exactly 2^28 bytes (851-3282).
var n, s;
n = 268435457;
s = n * "a";
out("mulstr_2p28+1_a_len", string_length(s));
