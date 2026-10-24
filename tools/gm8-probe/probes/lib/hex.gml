// hex(x): the IEEE-754 bits of real x as 16 hex digits, or "nonfinite" for a NaN or an infinity.
// Uses only operations that are exact whenever the FPU rounds to at least 53 bits (scaling by 2,
// subtracting powers of 2 from whole numbers, and m - 1 on m in [1, 2)), and floor only below 4: GM8's
// floor wraps to 32 bits from 2^31 up (numbers.gml). Comparisons are between whole numbers or have a margin,
// so GM8's 1e-13 tolerance can't bite.
var val, a, neg, e, m, v, top, out, k, p, d;
val = argument0;
// Comparisons subtract, so inf == inf is a comparison with a NaN (inf - inf). On OpenGMK every one of
// those is false; on GM8 some are true (div0.gml), so NaN and inf can't be told apart safely here.
if (!(val == val)) return "nonfinite";
// The sign, -0 included: atan2(+-0, -1) is +-pi.
neg = arctan2(val, -1) < 0;
a = abs(val);
top = 0;
v = 0;
{
    e = 0;
    m = a;
    while (m >= 4) { m /= 2; e += 1; }
    while (floor(m) >= 2) { m /= 2; e += 1; }
    while (floor(m) < 1 && e > -1074) { m *= 2; e -= 1; }
    if (floor(m) < 1) {
        top = 0; // zero
    } else if (e < -1022) {
        v = a;
        repeat (1074) v *= 2;
    } else {
        top = e + 1023;
        v = (m - 1) * 4503599627370496;
    }
}
if (neg) top += 2048;
// v < 2^52: 13 hex digits, top first, by subtracting each digit's place value.
out = "";
p = 4503599627370496;
repeat (13) {
    p /= 16;
    d = 0;
    while (v >= p) { v -= p; d += 1; }
    out += string_char_at("0123456789abcdef", d + 1);
}
m = "";
p = 4096;
repeat (3) {
    p /= 16;
    d = 0;
    while (top >= p) { top -= p; d += 1; }
    m += string_char_at("0123456789abcdef", d + 1);
}
return m + out;
