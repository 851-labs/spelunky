// make_color_rgb's components past a byte (int-args: past 32 bits they're kept): is it r | g << 8 | b << 16,
// where overlapping bits merge, or r + g * 256 + b * 65536, where they add? In an Int64, where 2^64 wraps to 0,
// or in reals? Nothing here should error.
var p32, p40, p47, p48, p56;
p32 = 4294967296;
p40 = p32 * 256;
p47 = p40 * 128;
p48 = p40 * 256;
p56 = p48 * 256;
out("color_255_255_255", make_color_rgb(255, 255, 255));
out("color_-1_0_0", make_color_rgb(-1, 0, 0));
out("color_0_-1_0", make_color_rgb(0, -1, 0));
out("color_0_0_-1", make_color_rgb(0, 0, -1));
out("color_-1_1_0", make_color_rgb(-1, 1, 0));
out("color_-1_-1_0", make_color_rgb(-1, -1, 0));
out("color_-256_1_0", make_color_rgb(-256, 1, 0));
out("color_256_1_0", make_color_rgb(256, 1, 0));
out("color_0_256_0", make_color_rgb(0, 256, 0));
out("color_0_256_1", make_color_rgb(0, 256, 1));
out("color_1.5_2.5_-0.5", make_color_rgb(1.5, 2.5, -0.5));
out("color_0_2p32_0", make_color_rgb(0, p32, 0));
out("color_0_0_2p40", make_color_rgb(0, 0, p40));
out("color_0_0_2p47", make_color_rgb(0, 0, p47));
out("color_0_0_2p48", make_color_rgb(0, 0, p48));
out("color_0_2p56_0", make_color_rgb(0, p56, 0));
out("color_1_0_2p48", make_color_rgb(1, 0, p48));
out("color_-1_0_1", make_color_rgb(-1, 0, 1));
out("color_65536_0_1", make_color_rgb(65536, 0, 1));
