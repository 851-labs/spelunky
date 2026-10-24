// real * string with a count of 2^31: cut to 32 bits it's -2^31, so none (""), or a 64-bit count (2 GB of "a")?
var p31;
p31 = 2147483648;
out("mulstr_2p31_a", p31 * "a");
