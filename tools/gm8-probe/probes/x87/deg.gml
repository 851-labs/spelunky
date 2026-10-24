// sin and cos of every half degree from 0 to 359.5, as the game calls them (cos(degtorad(dir))), and
// the motion built-ins on the same angles: setting direction and speed sets hspeed and vspeed (cos and
// sin), and setting hspeed and vspeed sets direction (atan2, in degrees) and speed. Too long for run-all
// (README.md, "Real x87").
var k, r, a, b;
for (k = 0; k < 720; k += 1) {
    r = degtorad(k / 2);
    out("rad_" + string(k), r);
    out("sin_" + string(k), sin(r));
    out("cos_" + string(k), cos(r));
}
for (k = 0; k < 720; k += 1) {
    direction = k / 2;
    speed = 1 + (k mod 8) / 2;
    out("hsp_" + string(k), hspeed);
    out("vsp_" + string(k), vspeed);
}
for (a = 0; a < 25; a += 1) {
    for (b = 0; b < 25; b += 1) {
        hspeed = a / 2 - 6;
        vspeed = b / 2 - 6;
        out("dir_" + string(a) + "_" + string(b), direction);
        out("spd_" + string(a) + "_" + string(b), speed);
    }
}
speed = 0;
