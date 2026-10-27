// `draw_surface` of a surface that doesn't exist (851-3308): GM8's built-in checks it first (Spelunky.exe 0x556a80,
// read only to describe it) and shows "Trying to use non-existing surface.". OpenGMK draws nothing.
out("surface_missing_exists", surface_exists(7));
draw_surface(7, 0, 0);
out("surface_missing_drawn", "drawn");
