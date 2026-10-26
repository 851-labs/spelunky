// What the runner does to speed, and to the components, after hspeed or vspeed changes (851-3281). In
// motion-range every tiny speed reads 0 where point_distance gives the tiny distance, and a huge hspeed reads
// back 1 ulp off what was written. Each case starts from rest, sets the components, then reads all four
// variables: is speed snapped to a whole number, as direction is, and are the components recomputed from it?
var p, g, h;
p = 1; repeat (1023) p = p * 2; // 2^1023
g = 1; repeat (600) g = g * 2; // 2^600
h = 1; repeat (600) h = h / 2; // 2^-600
hspeed = 0; vspeed = 0; hspeed = h;
out("h_2p-600_0", hspeed); out("v_2p-600_0", vspeed); out("s_2p-600_0", speed); out("d_2p-600_0", direction);
hspeed = 0; vspeed = 0; hspeed = 0.00005;
out("h_0.00005_0", hspeed); out("v_0.00005_0", vspeed); out("s_0.00005_0", speed); out("d_0.00005_0", direction);
hspeed = 0; vspeed = 0; hspeed = 0.0002;
out("h_0.0002_0", hspeed); out("v_0.0002_0", vspeed); out("s_0.0002_0", speed); out("d_0.0002_0", direction);
hspeed = 0; vspeed = 0; hspeed = 5.00001;
out("h_5.00001_0", hspeed); out("v_5.00001_0", vspeed); out("s_5.00001_0", speed); out("d_5.00001_0", direction);
hspeed = 0; vspeed = 0; hspeed = 3.00003; vspeed = 4.00004;
out("h_3.00003_4.00004", hspeed); out("v_3.00003_4.00004", vspeed); out("s_3.00003_4.00004", speed); out("d_3.00003_4.00004", direction);
hspeed = 0; vspeed = 0; hspeed = 0.3; vspeed = 0.7;
out("h_0.3_0.7", hspeed); out("v_0.3_0.7", vspeed); out("s_0.3_0.7", speed); out("d_0.3_0.7", direction);
hspeed = 0; vspeed = 0; hspeed = 0.75; vspeed = -1;
out("h_0.75_-1", hspeed); out("v_0.75_-1", vspeed); out("s_0.75_-1", speed); out("d_0.75_-1", direction);
hspeed = 0; vspeed = 0; hspeed = 1000.3; vspeed = 2000.7;
out("h_1000.3_2000.7", hspeed); out("v_1000.3_2000.7", vspeed); out("s_1000.3_2000.7", speed); out("d_1000.3_2000.7", direction);
hspeed = 0; vspeed = 0; hspeed = 0.3 * 1000000000000; vspeed = 0.7 * 1000000000000;
out("h_0.3e12_0.7e12", hspeed); out("v_0.3e12_0.7e12", vspeed); out("s_0.3e12_0.7e12", speed); out("d_0.3e12_0.7e12", direction);
hspeed = 0; vspeed = 0; hspeed = 0.75 * g; vspeed = -g;
out("h_0.75x2p600_-2p600", hspeed); out("v_0.75x2p600_-2p600", vspeed); out("s_0.75x2p600_-2p600", speed); out("d_0.75x2p600_-2p600", direction);
hspeed = 0; vspeed = 0; hspeed = 0.75 * p;
out("h_0.75x2p1023_0", hspeed); out("v_0.75x2p1023_0", vspeed); out("s_0.75x2p1023_0", speed); out("d_0.75x2p1023_0", direction);
hspeed = 0; vspeed = 0; vspeed = -p;
out("h_0_-2p1023", hspeed); out("v_0_-2p1023", vspeed); out("s_0_-2p1023", speed); out("d_0_-2p1023", direction);
hspeed = 0.75 * p;
out("h_0.75x2p1023_-2p1023", hspeed); out("v_0.75x2p1023_-2p1023", vspeed); out("s_0.75x2p1023_-2p1023", speed); out("d_0.75x2p1023_-2p1023", direction);
// The other way round: speed and direction written, the components read.
speed = 0; direction = 0; speed = h;
out("h_speed_2p-600", hspeed); out("s_speed_2p-600", speed);
speed = 0; direction = 0; speed = 5.00001;
out("h_speed_5.00001", hspeed); out("s_speed_5.00001", speed);
