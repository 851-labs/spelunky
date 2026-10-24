// string_format of infinities and NaN, made as `nonfinite` makes them: on GM8 on a real Intel CPU (and the
// oracle), exp(1000) is already an error.
var pinf, nan;
pinf = exp(1000);
nan = power(-8, 1 / 3);
out("sf_inf_0_2", "[" + string_format(pinf, 0, 2) + "]");
out("sf_-inf_6_2", "[" + string_format(-pinf, 6, 2) + "]");
out("sf_nan_0_2", "[" + string_format(nan, 0, 2) + "]");
