// Integer arguments that are an infinity or a NaN. On a real Intel CPU exp(1000) is already an error, so
// these are Rosetta's.
var pinf, nan, n;
pinf = exp(1000);
nan = power(-8, 1 / 3);
out("chr_inf", ord(chr(pinf)));
out("chr_nan", ord(chr(nan)));
out("charat_inf", string_char_at("abc", pinf));
n = 0; repeat (pinf) { n += 1; if (n >= 3) break; } out("repeat_inf", n);
n = 0; repeat (nan) { n += 1; if (n >= 3) break; } out("repeat_nan", n);
