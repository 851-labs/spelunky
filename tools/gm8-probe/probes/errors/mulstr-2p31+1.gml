// real * string with a count of 2^31 + 1, cut to 32 bits -2^31 + 1: none (""), unlike 2^31 (mulstr-2p31)?
var p31;
p31 = 2147483648;
out("mulstr_2p31+1_a", (p31 + 1) * "a");
