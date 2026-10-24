// string_format with negative decimals: how ties round to a multiple of 10^-decimals.
out("sfn_125_-1", "[" + string_format(125, 0, -1) + "]");
out("sfn_135_-1", "[" + string_format(135, 0, -1) + "]");
out("sfn_-125_-1", "[" + string_format(-125, 0, -1) + "]");
out("sfn_5_-1", "[" + string_format(5, 0, -1) + "]");
out("sfn_15_-1", "[" + string_format(15, 0, -1) + "]");
out("sfn_250_-2", "[" + string_format(250, 0, -2) + "]");
out("sfn_50000_-5", "[" + string_format(50000, 0, -5) + "]");
out("sfn_124.9_-1", "[" + string_format(124.9, 0, -1) + "]");
out("sfn_1234.5_8_-2", "[" + string_format(1234.5, 8, -2) + "]");
out("sfn_7_0_-20", "[" + string_format(7, 0, -20) + "]");
