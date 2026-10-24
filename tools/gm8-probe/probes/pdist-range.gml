// point_distance where the squares leave a double's range but the distance doesn't: GM8 keeps them in an x87
// register (errors/pdist-overflow), so in doubles each of these would be inf or lose bits. Nothing here should
// error.
var p, h, q, t;
p = 1; repeat (1023) p = p * 2; // 2^1023
h = 1; repeat (600) h = h / 2; // 2^-600
q = 1; repeat (540) q = q / 2; // 2^-540
t = 1; repeat (1074) t = t / 2; // 2^-1074
out("pdist_2p1023_2p1023", point_distance(0, 0, p, p));
out("pdist_-2p1023", point_distance(p, 0, 0, 0));
out("pdist_0.75x2p1023_2p1023", point_distance(0, 0, p * 0.75, p));
out("pdist_2p-600", point_distance(0, 0, h, 0));
out("pdist_3x2p-600_2p-600", point_distance(0, 0, 3 * h, h));
out("pdist_5x2p-540_7x2p-540", point_distance(0, 0, 5 * q, 7 * q));
out("pdist_3x2p-1074_4x2p-1074", point_distance(0, 0, 3 * t, 4 * t));
out("pdist_2p-1074_2p-1074", point_distance(0, 0, t, t));
