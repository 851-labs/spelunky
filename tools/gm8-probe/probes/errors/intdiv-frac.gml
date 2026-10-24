// div by a fraction: is the divisor truncated to an integer first (7 div 0.5 would divide by 0)?
var e;
out("intdiv_7_2.5", 7 div 2.5);
out("intdiv_7_0.5", 7 div 0.5);
out("intdiv_1_0.3", 1 div 0.3);
e = 1; repeat (14) e /= 10;
out("intdiv_7_1e-14", 7 div e);
