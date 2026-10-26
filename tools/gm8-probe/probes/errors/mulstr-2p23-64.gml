// real * string with a count of 2^23 and a 64-byte string: 536870912 bytes. Is GM8's limit on the string's bytes
// or on the count (mulstr-2p28, mulstr-2p29: "a") (851-3282)?
var n, t, s;
n = 8388608;
t = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
s = n * t;
out("mulstr_2p23_64_len", string_length(s));
