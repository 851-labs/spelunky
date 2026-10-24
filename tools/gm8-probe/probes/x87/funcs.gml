// arctan, sqrt, ln, exp, power and point_distance of v = k / 100 (0.01 to 10). Too long for run-all
// (README.md, "Real x87").
var k, v;
for (k = 1; k <= 1000; k += 1) {
    v = k / 100;
    out("arctan_" + string(k), arctan(v));
    out("sqrt_" + string(k), sqrt(v));
    out("ln_" + string(k), ln(v));
    out("exp_" + string(k), exp(v));
    out("power_" + string(k), power(v, 3 / 2));
    out("pdist_" + string(k), point_distance(0, 0, v, 13 / 10));
}
