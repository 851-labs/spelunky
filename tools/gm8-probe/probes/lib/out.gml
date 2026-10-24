// out(name, value): appends "name = <hex bits> <string(value)>" for a real, or "name = "text"" for a
// string, to probe.txt. It opens and closes the file each time so that what came before an error
// that stops the game is kept.
var f;
f = file_text_open_append("probe.txt");
if (is_string(argument1)) {
    file_text_write_string(f, argument0 + " = " + chr(34) + argument1 + chr(34));
} else {
    file_text_write_string(f, argument0 + " = " + hex(argument1) + " " + string(argument1));
}
file_text_writeln(f);
file_text_close(f);
