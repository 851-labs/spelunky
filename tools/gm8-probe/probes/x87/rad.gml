// sin, cos and tan in radians: k / 100 (0.01 to 10), k / 64 (to 10, exact) and k * k / 3 (to 30000,
// where the argument reduction's pi shows). Too long for run-all (README.md, "Real x87").
var k, r;
for (k = 1; k <= 1000; k += 1) {
    r = k / 100;
    out("sin_c" + string(k), sin(r));
    out("cos_c" + string(k), cos(r));
    out("tan_c" + string(k), tan(r));
}
for (k = 1; k <= 640; k += 1) {
    r = k / 64;
    out("sin_s" + string(k), sin(r));
    out("cos_s" + string(k), cos(r));
    out("tan_s" + string(k), tan(r));
}
for (k = 1; k <= 300; k += 1) {
    r = k * k / 3;
    out("sin_q" + string(k), sin(r));
    out("cos_q" + string(k), cos(r));
    out("tan_q" + string(k), tan(r));
}
