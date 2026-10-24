// The generator after random_set_seed, and the highscore table on a fresh install.
var n;
random_set_seed(12345);
out("seed_after_set", random_get_seed());
for (n = 0; n < 8; n += 1) out("random_1_" + string(n), random(1));
out("random_100", random(100));
out("random_-5", random(-5));
out("seed_after_draws", random_get_seed());
random_set_seed(-1);
out("seed_-1", random_get_seed());
out("random_after_-1", random(1));
random_set_seed(4294967296 + 7);
out("seed_2p32+7", random_get_seed());
random_set_seed(2.5);
out("seed_2.5", random_get_seed());
out("choose_after_2.5", choose(1, 2, 3, 4, 5));

// Highscores: a fresh table, then one added score
for (n = 1; n <= 10; n += 1) {
    out("hs_value_" + string(n), highscore_value(n));
    out("hs_name_" + string(n), highscore_name(n));
}
out("hs_value_0", highscore_value(0));
out("hs_value_11", highscore_value(11));
out("hs_name_11", highscore_name(11));
highscore_add("probe", 5);
out("hs_after_add_value_1", highscore_value(1));
out("hs_after_add_name_1", highscore_name(1));
out("hs_after_add_value_10", highscore_value(10));
highscore_clear();
out("hs_after_clear_value_1", highscore_value(1));
out("hs_after_clear_name_1", highscore_name(1));
