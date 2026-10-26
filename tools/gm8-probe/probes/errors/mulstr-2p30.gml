// real * string with a count of 2^30: 1073741824 bytes of "a". Where does the 32-bit GM8 runner stop building a
// huge string (851-3282)? One count per probe, as each may run the runner out of memory.
var n, s;
n = 1073741824;
s = n * "a";
out("mulstr_2p30_a_len", string_length(s));
