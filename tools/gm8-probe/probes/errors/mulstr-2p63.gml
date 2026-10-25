// real * string with a count from 2^63 up: Delphi's Round into an Int64 (the FISTP that floor(1e20) traps on)?
var p63;
p63 = 4294967296 * 2147483648;
out("mulstr_2p63_a", p63 * "a");
