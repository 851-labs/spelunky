// tile_layer_find on GM 8.0 (851-3787): OpenGMK reads 8.0 as measuring a tile 0 wide and 0 high, so never finding
// one. Which tile does it find, and where: the box's edges, the depth, ties between tiles at one depth (added with
// tiles at other depths between them, before and after a draw sorts them), a deleted tile, and a scaled one.
var a, b, c, d, e, k, w;
a = tile_add(bgTrees, 0, 0, 16, 48, 100, 100, 3);
out("a", a);
out("left_top", tile_layer_find(3, 100, 100));
out("below_left", tile_layer_find(3, 99.99, 100));
out("right_in", tile_layer_find(3, 115.99, 147.99));
out("right_edge", tile_layer_find(3, 116, 120));
out("bottom_edge", tile_layer_find(3, 108, 148));
out("above_top", tile_layer_find(3, 108, 99.99));
out("depth_frac", tile_layer_find(3.4, 108, 120));
out("depth_other", tile_layer_find(4, 108, 120));
b = tile_add(bgTrees, 0, 0, 16, 16, 300, 300, 5);
k = tile_add(bgTrees, 0, 0, 16, 16, 300, 300, 9);
c = tile_add(bgTrees, 0, 0, 16, 16, 300, 300, 5);
k = tile_add(bgTrees, 0, 0, 16, 16, 300, 300, 1);
d = tile_add(bgTrees, 0, 0, 16, 16, 300, 300, 5);
k = tile_add(bgTrees, 0, 0, 16, 16, 300, 300, 9);
e = tile_add(bgTrees, 0, 0, 16, 16, 300, 300, 5);
out("b", b);
out("c", c);
out("d", d);
out("e", e);
out("tie", tile_layer_find(5, 304, 304));
out("tie_9", tile_layer_find(9, 304, 304));
screen_redraw();
out("tie_drawn", tile_layer_find(5, 304, 304));
tile_delete(b);
out("tie_after_delete", tile_layer_find(5, 304, 304));
w = tile_add(bgTrees, 0, 0, 16, 16, 500, 500, 7);
tile_set_scale(w, 2, 2);
out("w", w);
out("scaled_unscaled_box", tile_layer_find(7, 508, 508));
out("scaled_outside", tile_layer_find(7, 520, 520));
