// A repeat count from 2^63 up. Counts the runs, stopping at 3.
var p63, n;
p63 = 4294967296 * 2147483648;
n = 0;
repeat (p63) { n += 1; if (n >= 3) break; }
out("repeat_2p63", n);
