// The runner's speed and direction after hspeed or vspeed changes (851-3281), where the squares of the
// components leave a double's range but speed doesn't. point_distance keeps its squares in an x87 register
// (pdist-range); if the runner's Sqrt(Sqr(hspeed) + Sqr(vspeed)) does too, none of these speeds is inf or 0.
// Each case starts from rest, then sets hspeed and vspeed. Nothing here should error.
var p, g, h, q, t;
p = 1; repeat (1023) p = p * 2; // 2^1023
g = 1; repeat (600) g = g * 2; // 2^600
h = 1; repeat (600) h = h / 2; // 2^-600
q = 1; repeat (540) q = q / 2; // 2^-540
t = 1; repeat (1074) t = t / 2; // 2^-1074
hspeed = 0; vspeed = 0; hspeed = h;
out("speed_2p-600_0", speed); out("direction_2p-600_0", direction);
hspeed = 0; vspeed = 0; vspeed = h;
out("speed_0_2p-600", speed); out("direction_0_2p-600", direction);
hspeed = 0; vspeed = 0; hspeed = 3 * h; vspeed = -4 * h;
out("speed_3x2p-600_-4x2p-600", speed); out("direction_3x2p-600_-4x2p-600", direction);
hspeed = 0; vspeed = 0; hspeed = 5 * q; vspeed = 7 * q;
out("speed_5x2p-540_7x2p-540", speed); out("direction_5x2p-540_7x2p-540", direction);
hspeed = 0; vspeed = 0; hspeed = 3 * t; vspeed = 4 * t;
out("speed_3x2p-1074_4x2p-1074", speed); out("direction_3x2p-1074_4x2p-1074", direction);
hspeed = 0; vspeed = 0; hspeed = t; vspeed = t;
out("speed_2p-1074_2p-1074", speed); out("direction_2p-1074_2p-1074", direction);
hspeed = 0; vspeed = 0; hspeed = g;
out("speed_2p600_0", speed); out("direction_2p600_0", direction);
hspeed = 0; vspeed = 0; hspeed = -g; vspeed = g;
out("speed_-2p600_2p600", speed); out("direction_-2p600_2p600", direction);
hspeed = 0; vspeed = 0; hspeed = p; vspeed = p;
out("speed_2p1023_2p1023", speed); out("direction_2p1023_2p1023", direction);
hspeed = 0; vspeed = 0; hspeed = p * 0.75; vspeed = -p;
out("speed_0.75x2p1023_-2p1023", speed); out("direction_0.75x2p1023_-2p1023", direction);
// The components read back as written.
out("hspeed_0.75x2p1023_-2p1023", hspeed); out("vspeed_0.75x2p1023_-2p1023", vspeed);
