// The same through an instance variable (851-3796): iv[0, 0] and plain iv with only iv[1, 0] set, then the
// 1-D form, ia[0] with only ia[3] set, and a 1-D read of the 2-D one, iv[0].
iv[1, 0] = 1;
out("iv_0_0", iv[0, 0]);
out("iv_plain", iv);
out("iv_1d_0", iv[0]);
ia[3] = 1;
out("ia_0", ia[0]);
out("iv_2_0", iv[2, 0]);
