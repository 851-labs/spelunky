// An integer argument from 2^63 up: Delphi's Round into an Int64 (the FISTP that floor(1e20) traps on)?
// ord(chr(n)) is the byte chr keeps.
var p63;
p63 = 4294967296 * 2147483648;
out("chr_2p63", ord(chr(p63)));
