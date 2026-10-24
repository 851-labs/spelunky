// Truthiness, comparisons, rounding, string(), integer conversion, bitwise operators, div and mod.
// Nothing here errors on OpenGMK; see the other probes for the ones that might.
var h, t, eps, big, k, nz;
h = 0.5;
t = 0; if (h) t = 1; out("if_0.5", t);
// 0.5 - 2^-54, the double just below 0.5
k = 1; repeat (54) k /= 2;
t = 0; if (h - k) t = 1; out("if_below_0.5", t);
t = 0; if (-h) t = 1; out("if_-0.5", t);
t = 0; if (!h) t = 1; out("not_0.5", t);
t = 0; if (!(h - k)) t = 1; out("not_below_0.5", t);
out("and_0.5", h && h);

// The comparison tolerance: 1e-13 on OpenGMK. 9e-14 and 1.1e-13 as 1 / 10^n multiples.
eps = 1; repeat (14) eps /= 10;
out("eq_1_1+1e-14", 1 == 1 + eps);
out("eq_1_1+9e-14", 1 == 1 + 9 * eps);
out("eq_1_1+1.1e-13", 1 == 1 + 11 * eps);
out("eq_1_1+2e-13", 1 == 1 + 20 * eps);
out("lt_1_1+5e-14", 1 < 1 + 5 * eps);
out("le_1+5e-14_1", 1 + 5 * eps <= 1);
out("gt_1+5e-14_1", 1 + 5 * eps > 1);
out("ne_1_1+5e-14", 1 != 1 + 5 * eps);
out("eq_0.1+0.2_0.3", 0.1 + 0.2 == 0.3);
big = 1099511627776;
out("eq_2p40_2p40+1e-4", big == big + 0.0001);
out("eq_tiny_0", eps * eps == 0);

// round, floor, ceil and frac, at ties and past 2^31
out("round_0.5", round(0.5));
out("round_1.5", round(1.5));
out("round_2.5", round(2.5));
out("round_-0.5", round(-0.5));
out("round_-1.5", round(-1.5));
out("round_-2.5", round(-2.5));
out("round_0.5-2^-54", round(h - k));
out("round_2p31+0.5", round(2147483648.5));
out("round_2p40+0.5", round(big + 0.5));
out("floor_2p31-0.5", floor(2147483647.5));
out("floor_2p31+0.5", floor(2147483648.5));
out("floor_2p32+0.5", floor(4294967296.5));
out("floor_2p40+0.5", floor(big + 0.5));
out("floor_-2p31-0.5", floor(-2147483648.5));
out("floor_-2p40-0.5", floor(-big - 0.5));
out("floor_1e20", floor(big * big * 100));
out("ceil_2p40+0.5", ceil(big + 0.5));
out("ceil_-0.5", ceil(-0.5));
out("frac_2p40+0.25", frac(big + 0.25));
out("frac_-1.25", frac(-1.25));
out("sign_1e-14", sign(eps));
out("abs_-0", abs(-0));

// string() of reals: ties at the third decimal, -0, big and tiny values
nz = 0 * -1;
out("neg_zero", nz);
out("str_0.125", string(0.125));
out("str_0.375", string(0.375));
out("str_0.625", string(0.625));
out("str_-0.125", string(-0.125));
out("str_2.5", string(2.5));
out("str_-0.5", string(-0.5));
out("str_-0", string(nz));
out("str_-0.004", string(-0.004));
out("str_-0.005", string(-0.005));
out("str_0.005", string(0.005));
out("str_0.015", string(0.015));
out("str_1.005", string(1.005));
out("str_2.675", string(2.675));
out("str_0.995", string(0.995));
out("str_99.995", string(99.995));
out("str_1/3", string(1 / 3));
out("str_2p40", string(big));
out("str_2p40+0.5", string(big + 0.5));
out("str_1e15", string(1000000000000000));
out("str_1e15+0.3", string(1000000000000000 + 0.3));
out("str_2p53", string(big * 8192));
out("str_1e20", string(big * big * 100));
out("str_1e-5", string(eps * 1000000000));
out("str_-1e-14", string(-eps));
out("str_int_string", string(12) + "|" + string(-3));

// string <-> real conversions that should work on any reading
out("real_12", real("12"));
out("real_ 12 ", real(" 12 "));
out("real_1.5", real("1.5"));
out("real_-0", real("-0"));
out("real_empty", real(""));
out("real_.5", real(".5"));
out("real_+3", real("+3"));
out("real_0.1", real("0.1"));
out("real_0.3", real("0.3"));
out("lit_0.3", 0.3);
out("lit_0.1", 0.1);
out("real_1e3", real("1e3"));

// Bitwise operators: operands are rounded to integers; what width?
out("and_5_3", 5 & 3);
out("and_-1_255", (-1) & 255);
out("or_-5_2", (-5) | 2);
out("xor_-5_1", (-5) ^ 1);
out("and_3.7_1", 3.7 & 1);
out("and_2.5_3", 2.5 & 3);
out("or_-2.5_0", (-2.5) | 0);
out("or_2p31_0", 2147483648 | 0);
out("or_2p32+5_0", 4294967301 | 0);
out("or_2p40_1", big | 1);
out("or_1e20_0", (big * big * 100) | 0);
out("shl_1_31", 1 << 31);
out("shl_1_32", 1 << 32);
out("shl_1_40", 1 << 40);
out("shr_-1_1", (-1) >> 1);
out("shr_2p31_1", 2147483648 >> 1);
out("shr_-8_1", (-8) >> 1);

// div and mod with nonzero divisors
out("div_7_2", 7 div 2);
out("div_-7_2", (-7) div 2);
out("div_7_-2", 7 div -2);
out("div_7.5_2", 7.5 div 2);
out("div_2p40+1_3", (big + 1) div 3);
out("mod_-7_3", (-7) mod 3);
out("mod_7_-3", 7 mod -3);
out("mod_5.5_2", 5.5 mod 2);
out("mod_-5.5_2", (-5.5) mod 2);
out("mod_1_0.1", 1 mod 0.1);
out("mod_2p40+3_7", (big + 3) mod 7);
