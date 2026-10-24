// point_direction and arctan2 from (0, 0) to every whole (dx, dy) in [-24, 24]^2, and point_direction
// and point_distance to a fractional grid (dx = i * 3/8 - 7, dy = j * 5/8 - 5), like the game's
// positions after fractional moves. Too long for run-all (README.md, "Real x87").
var i, j, dx, dy, n;
for (i = -24; i <= 24; i += 1) {
    for (j = -24; j <= 24; j += 1) {
        n = string(i) + "_" + string(j);
        out("pdir_" + n, point_direction(0, 0, i, j));
        out("atan2_" + n, arctan2(-j, i));
    }
}
for (i = 0; i < 40; i += 1) {
    for (j = 0; j < 20; j += 1) {
        dx = i * 3 / 8 - 7;
        dy = j * 5 / 8 - 5;
        n = string(i) + "_" + string(j);
        out("fpdir_" + n, point_direction(0, 0, dx, dy));
        out("fpdist_" + n, point_distance(0, 0, dx, dy));
    }
}
