// real * string with a count of 2^29: 536870912 bytes of "a". Where does the 32-bit GM8 runner stop building a
// huge string (851-3282)? One count per probe, as each may run the runner out of memory.
var n, s;
n = 536870912;
s = n * "a";
out("mulstr_2p29_a_len", string_length(s));
