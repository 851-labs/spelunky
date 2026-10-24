// Infinities and NaNs without dividing by zero (an error on GM8): exp overflows to inf, and
// power(-8, 1/3) is a NaN on both runners. How they compare, test and print.
var pinf, ninf, nan, t;
pinf = exp(1000);
ninf = -pinf;
nan = power(-8, 1 / 3);
out("nf_str_inf", string(pinf));
out("nf_str_-inf", string(ninf));
out("nf_str_nan", string(nan));
out("nf_nan_eq_nan", nan == nan);
out("nf_nan_ne_nan", nan != nan);
out("nf_nan_lt_0", nan < 0);
out("nf_nan_le_0", nan <= 0);
out("nf_nan_gt_0", nan > 0);
out("nf_nan_ge_0", nan >= 0);
t = 0; if (nan) t = 1; out("nf_if_nan", t);
out("nf_inf_eq_inf", pinf == pinf);
out("nf_inf_ne_inf", pinf != pinf);
out("nf_inf_gt_1", pinf > 1);
out("nf_ninf_lt_0", ninf < 0);
out("nf_str_inf_minus_inf", string(pinf - pinf));
out("nf_round_inf", string(round(pinf)));
out("nf_floor_inf", string(floor(pinf)));
out("nf_or_inf_0", pinf | 0);
