// real * string with a count of 2^27: 134217728 bytes of "a". Where does the 32-bit GM8 runner stop building a
// huge string (851-3282)? One count per probe, as each may run the runner out of memory.
var n, s;
n = 134217728;
s = n * "a";
out("mulstr_2p27_a_len", string_length(s));
