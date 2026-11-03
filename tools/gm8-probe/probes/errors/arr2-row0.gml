// A 2-D array whose only element is past row 0 (851-3796): element [0, 0], which is also the plain variable,
// reads 0 in GM8 (OpenGMK: "uninitialized variable"); the other elements of row 0 are errors.
global.pb[1, 5] = 1;
out("pb_1_2", global.pb[1, 2]);
out("pb_0_0", global.pb[0, 0]);
out("pb_plain", global.pb);
out("pb_0_1", global.pb[0, 1]);
