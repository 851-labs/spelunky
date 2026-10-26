// motion_add (851-3262), which is the runner's own routine behind gravity: hspeed and vspeed after
// motion_add(dir, amount) on every half degree, after hspeed = (k mod 7) / 4 - 0.75 and vspeed =
// (k mod 5) / 3 - 0.5, amount 0.5 + (k mod 6) / 4. GM8 reads the literal 0.75 as 0.7500000000000001
// (851-3280), so the speeds motion_add starts from are printed too (ma0h_, ma0v_). Too long for
// run-all (README.md, "Angles").
var k;
for (k = 0; k < 720; k += 1) {
    hspeed = (k mod 7) / 4 - 0.75;
    vspeed = (k mod 5) / 3 - 0.5;
    out("ma0h_" + string(k), hspeed);
    out("ma0v_" + string(k), vspeed);
    motion_add(k / 2, 0.5 + (k mod 6) / 4);
    out("mah_" + string(k), hspeed);
    out("mav_" + string(k), vspeed);
}
speed = 0;
