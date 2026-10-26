// The step's friction recomputes the components from the new speed, but a component of 2^63 doesn't stop the game
// there (errors/friction-2p63), as it does after a speed write. Does friction snap them to a whole number within
// 0.0001 at all? At 90 degrees the snap makes hspeed 0, and without it hspeed is speed * cos(90 * Pi / 180), about
// 6e-17 * speed. (A step probe: oScreen moves, one case a step.)
direction = 90;
speed = 2;
friction = 1;
// step:
if (global.probe_step == 2) {
    out("friction_h_90_2_1", hspeed);
    out("friction_v_90_2_1", vspeed);
    out("friction_s_90_2_1", speed);
    out("friction_d_90_2_1", direction);
    direction = 30;
    speed = 3;
    friction = 1;
}
if (global.probe_step == 3) {
    out("friction_h_30_3_1", hspeed);
    out("friction_v_30_3_1", vspeed);
    out("friction_s_30_3_1", speed);
    direction = 200;
    speed = 10;
    friction = 0.3;
}
if (global.probe_step == 4) {
    out("friction_h_200_10_0.3", hspeed);
    out("friction_v_200_10_0.3", vspeed);
    out("friction_s_200_10_0.3", speed);
    out("friction_d_200_10_0.3", direction);
    // The write snaps hspeed to 1000; after the step the speed is 1000.00004.
    friction = 0;
    speed = 0;
    direction = 0;
    speed = 1000.00005;
    friction = 0.00001;
}
if (global.probe_step == 5) {
    out("friction_h_0_1000.00005_0.00001", hspeed);
    out("friction_s_0_1000.00005_0.00001", speed);
    out("end", "ok");
    game_end();
}
