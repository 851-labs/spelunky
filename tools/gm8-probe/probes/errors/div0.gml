// Real division by zero, then what the infinities and NaN print as.
var z, pinf, ninf, nan;
z = 0;
pinf = 1 / z;
out("div_1_0", pinf);
ninf = -1 / z;
out("div_-1_0", ninf);
nan = z / z;
out("div_0_0", nan);
out("str_inf", string(pinf));
out("str_-inf", string(ninf));
out("str_nan", string(nan));
out("inf_minus_inf", pinf - pinf);
out("nan_eq_nan", nan == nan);
out("nan_ne_nan", nan != nan);
out("nan_gt_0.5", nan > 0.5);
out("inf_or_0", pinf | 0);
out("round_inf", round(pinf));
// Comparisons with a NaN (inf == inf subtracts to one)
out("nan_lt_0", nan < 0);
out("nan_le_0", nan <= 0);
out("nan_gt_0", nan > 0);
out("nan_ge_0", nan >= 0);
out("nan_value", nan);
out("inf_eq_inf", pinf == pinf);
out("inf_gt_1", pinf > 1);
out("ninf_lt_0", ninf < 0);
out("floor_inf", floor(pinf));
