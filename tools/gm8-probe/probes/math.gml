// Transcendentals and accumulated arithmetic. GM8 (Delphi) computes sin, cos, tan, arctan, arctan2,
// sqrt, ln and exp with x87 instructions; the port and the oracle are correctly rounded.
var n, v, s, p, tiny, ten;
for (n = 1; n <= 120; n += 1) {
    v = n * 37 / 100;
    out("sin_" + string(n), sin(v));
    out("cos_" + string(n), cos(v));
    out("tan_" + string(n), tan(v));
    out("arctan_" + string(n), arctan(v));
    out("arctan2_" + string(n), arctan2(v, 1.7));
    out("sqrt_" + string(n), sqrt(v));
    out("ln_" + string(n), ln(v));
    out("exp_" + string(n), exp(v / 10));
    out("power_" + string(n), power(v, 1.5));
    out("pdir_" + string(n), point_direction(0, 0, n, 7));
    out("pdist_" + string(n), point_distance(0, 0, v, 1.3));
}
// Large arguments, where fsin's 66-bit pi shows
ten = 1; repeat (22) ten *= 10;
out("sin_1e22", sin(ten));
out("sin_1e6", sin(1000000));
out("cos_1e6", cos(1000000));
out("sin_pi", sin(pi));
out("cos_pi/2", cos(pi / 2));
out("sin_2pi", sin(2 * pi));
out("tan_pi/4", tan(pi / 4));
out("power_2_0.5", power(2, 0.5));
out("power_10_-2", power(10, -2));
out("power_-2_3", power(-2, 3));
out("sqr_1.1", sqr(1.1));
out("log2_10", log2(10));
out("log10_2", log10(2));
out("logn_3_10", logn(3, 10));
out("degtorad_33", degtorad(33));
out("radtodeg_1", radtodeg(1));
out("lengthdir_x_10_33", lengthdir_x(10, 33));
out("lengthdir_y_10_33", lengthdir_y(10, 33));
// Accumulation
s = 0; repeat (1000) s += 0.1; out("sum_0.1_x1000", s);
p = 1; repeat (100) p *= 1.1; out("prod_1.1_x100", p);
s = 0; for (n = 1; n <= 1000; n += 1) s += 1 / n; out("harmonic_1000", s);
s = 1; repeat (60) s = s * 0.7 + 0.3 / s; out("iter_mix", s);
// Subnormal results, where a wider exponent range would round twice
tiny = 1; repeat (1022) tiny /= 2;
out("subnormal_0.75", tiny * 0.75);
out("subnormal_mul", (tiny * 3) * (1 / 1048576 / 3));
out("subnormal_div", tiny / 3);
