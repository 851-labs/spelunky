// a |= 2^63 on a local: the FISTP traps on Intel. Does op= swallow that, as it does type errors and /= 0?
var p63, a;
p63 = 4294967296 * 2147483648;
a = 1;
a |= p63;
out("orassign_local", a);
