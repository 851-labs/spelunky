// `draw_surface_stretched` of a surface that doesn't exist (851-3308), with a width and height of 0 or less, which
// the draw itself would skip: the built-in checks the surface first (Spelunky.exe 0x556df0, read only to describe
// it), so this shows "Trying to use non-existing surface." too.
out("surface_missing_stretched_exists", surface_exists(7));
draw_surface_stretched(7, 0, 0, -1, -1);
out("surface_missing_stretched_drawn", "drawn");
