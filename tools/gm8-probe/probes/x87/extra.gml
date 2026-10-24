// The formulas the other x87 probes leave open (851-3257): radtodeg of k/100 radians and of every half
// degree's degtorad; lengthdir_x and lengthdir_y of every half degree, next to the motion variables for
// the same speed and direction (on OpenGMK hspeed and vspeed snap to a whole number and lengthdir doesn't;
// GM8 gave the same bits for both, snap included); and
// point_direction between equal points away from the origin. Arithmetic is the same under Rosetta as on
// Intel, so this can tell the formulas apart on either. Too long for run-all (README.md, "Real x87").
var k, d, len;
for (k = -1000; k <= 1000; k += 1) {
    out("r2d_" + string(k), radtodeg(k / 100));
}
for (k = 0; k < 720; k += 1) {
    d = k / 2;
    len = 1 + (k mod 8) / 2;
    out("r2dd_" + string(k), radtodeg(degtorad(d)));
    out("ldx_" + string(k), lengthdir_x(len, d));
    out("ldy_" + string(k), lengthdir_y(len, d));
    direction = d;
    speed = len;
    out("hsp_" + string(k), hspeed);
    out("vsp_" + string(k), vspeed);
}
speed = 0;
out("pdir_same", point_direction(3, 5, 3, 5));
out("pdir_right", point_direction(3, 5, 4, 5));
out("pdir_left", point_direction(3, 5, 2, 5));
