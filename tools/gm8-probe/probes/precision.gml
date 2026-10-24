// How the FPU rounds. Each GML operation stores its result as a double, so a 64-bit (x87 extended)
// mantissa only shows through double rounding; a 24-bit one (Direct3D's default FPU mode) shows
// everywhere. Operands come from variables so nothing can be folded at compile time.
var one, big, b, i, t;
one = 1;
big = 16777216;
out("pc_2p24_plus_1", big + one);
// 1 + (2^-53 + 2^-64): 1 + 2^-52 with a 53-bit mantissa, 1 with a 64-bit one (double rounding).
b = 1;
repeat (53) b /= 2;
i = 1;
repeat (64) i /= 2;
b += i;
out("pc_double_rounding_add", one + b);
// The same for a product: (1 + 2^-26)(1 + 2^-27 + 2^-51) = A + 2^-53 + 2^-77, A = 1 + 2^-26 + 2^-27 + 2^-51
// (even). 53 bits round it up to A + 2^-52; 64 bits drop 2^-77, and the tie then goes to A.
t = 1;
repeat (26) t /= 2;
b = 1;
repeat (27) b /= 2;
i = 1;
repeat (51) i /= 2;
out("pc_double_rounding_mul", (one + t) * (one + b + i));
// A division and a square root that round to nearest even differently through 64 bits are rare;
// these are ordinary ones.
out("third", one / 3);
out("sqrt2", sqrt(2));
out("tenth_plus_fifth", 0.1 + 0.2);
// Intel's fsin reduces with a 66-bit pi, so fsin(double(pi)) is 1.2246063538223773e-16, where the
// true sine is 1.2246467991473532e-16. Real x87 hardware gives the first.
out("sin_pi", sin(pi));
out("pi", pi);
