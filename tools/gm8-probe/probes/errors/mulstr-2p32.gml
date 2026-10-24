// real * string with a count past 32 bits: cut to 32 bits ("aa"), or a 64-bit count (4 GB of "a")?
var p32;
p32 = 4294967296;
out("mulstr_2p32+2_a", (p32 + 2) * "a");
