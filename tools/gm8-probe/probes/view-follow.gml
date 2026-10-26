// View following (851-3303). At the start of the draw, each visible view with an object to follow (Spelunky.exe
// 0x4beca4) takes the target's rounded position, then on each axis keeps the border if `2 * border < size` (a 32-bit
// sum, which wraps), else centres the view; then clamps the view to the room; then, with a speed of 0 or more, limits
// the move to that speed from where the view was. OpenGMK tests `border < size / 2`, and limits the speed before it
// clamps. Views 1-7 in rIntro (960x240) each follow an oBlack, one case each, with OpenGMK's answer:
// 1. a 100x100 view at x = 900, its target at (905, 50), border 10, speed 5: 895, OpenGMK 860;
// 2. the same vertically, at y = 200, the target at (50, 205): 195, OpenGMK 140;
// 3. speed 0, the view at x = 2000, outside the room: it stays, OpenGMK clamps it to 860;
// 4. a 101x101 view, border 50, speeds -5 and -1000000 (any negative speed is unlimited), the target at (300, 150):
//    the border rule (249, 99), OpenGMK centres it (250, 100);
// 5. borders of 2^30 + 10, whose doubles wrap negative, the target at (300, 150): the border rule, clamped (0, 0),
//    OpenGMK centres it (250, 100);
// 6. the view at x = -500, the target at (50, 50), border 10, speed 5: -495, OpenGMK 0;
// 7. a view wider than the room (1000), at 0, border 10, speed 5: -5, OpenGMK -40.
// Each view's position after one frame of following and after two. (A step probe.)
// step:
var k, i, t, w, b;
k = global.probe_step;
if (k == 1) {
    // xview, yview, wview, hview, hborder, vborder, hspeed, vspeed, target x, target y
    b = 1; repeat (30) b = b * 2;
    b = b + 10;
    t[1, 0] = 900;  t[1, 1] = 0;    t[1, 2] = 100;  t[1, 3] = 100; t[1, 4] = 10; t[1, 5] = 10; t[1, 6] = 5;  t[1, 7] = 5;  t[1, 8] = 905; t[1, 9] = 50;
    t[2, 0] = 0;    t[2, 1] = 200;  t[2, 2] = 100;  t[2, 3] = 100; t[2, 4] = 10; t[2, 5] = 10; t[2, 6] = 5;  t[2, 7] = 5;  t[2, 8] = 50;  t[2, 9] = 205;
    t[3, 0] = 2000; t[3, 1] = 0;    t[3, 2] = 100;  t[3, 3] = 100; t[3, 4] = 10; t[3, 5] = 10; t[3, 6] = 0;  t[3, 7] = 0;  t[3, 8] = 905; t[3, 9] = 50;
    t[4, 0] = 0;    t[4, 1] = 0;    t[4, 2] = 101;  t[4, 3] = 101; t[4, 4] = 50; t[4, 5] = 50; t[4, 6] = -5; t[4, 7] = -1000000; t[4, 8] = 300; t[4, 9] = 150;
    t[5, 0] = 0;    t[5, 1] = 0;    t[5, 2] = 100;  t[5, 3] = 100; t[5, 4] = b;  t[5, 5] = b;  t[5, 6] = -1; t[5, 7] = -1; t[5, 8] = 300; t[5, 9] = 150;
    t[6, 0] = -500; t[6, 1] = 0;    t[6, 2] = 100;  t[6, 3] = 100; t[6, 4] = 10; t[6, 5] = 10; t[6, 6] = 5;  t[6, 7] = 5;  t[6, 8] = 50;  t[6, 9] = 50;
    t[7, 0] = 0;    t[7, 1] = 0;    t[7, 2] = 1000; t[7, 3] = 100; t[7, 4] = 10; t[7, 5] = 10; t[7, 6] = 5;  t[7, 7] = 5;  t[7, 8] = 500; t[7, 9] = 50;
    for (i = 1; i <= 7; i += 1) {
        view_visible[i] = true;
        view_xview[i] = t[i, 0];
        view_yview[i] = t[i, 1];
        view_wview[i] = t[i, 2];
        view_hview[i] = t[i, 3];
        view_xport[i] = 0;
        view_yport[i] = 0;
        view_wport[i] = 100;
        view_hport[i] = 100;
        view_hborder[i] = t[i, 4];
        view_vborder[i] = t[i, 5];
        view_hspeed[i] = t[i, 6];
        view_vspeed[i] = t[i, 7];
        w = instance_create(t[i, 8], t[i, 9], oBlack);
        w.visible = false;
        view_object[i] = w;
    }
}
if (k >= 2) {
    for (i = 1; i <= 7; i += 1) {
        out("follow_x" + string(i) + "_" + string(k), view_xview[i]);
        out("follow_y" + string(i) + "_" + string(k), view_yview[i]);
    }
}
if (k == 3) {
    out("end", "ok");
    game_end();
}
