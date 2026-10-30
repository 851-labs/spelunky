// A WM_KEYDOWN for a key that's already down, as keyboard auto-repeat sends (851-3774): is it pressed again,
// and does it set keyboard_key and keyboard_lastkey? keyboard_key_press goes through GM8's message loop, so
// each press or release shows up on the next step. N goes down, M is tapped, both variables are cleared, N
// goes down again while still held, then up. (Wine only: OpenGMK doesn't implement keyboard_key_press.)
// step:
var k;
k = global.probe_step;
if (k >= 2) {
    out("s" + string(k) + "_pressed_n", keyboard_check_pressed(ord("N")));
    out("s" + string(k) + "_held_n", keyboard_check(ord("N")));
    out("s" + string(k) + "_pressed_any", keyboard_check_pressed(vk_anykey));
    out("s" + string(k) + "_key", keyboard_key);
    out("s" + string(k) + "_lastkey", keyboard_lastkey);
}
if (k == 3) keyboard_key_press(ord("N"));
if (k == 5) keyboard_key_press(ord("M"));
if (k == 7) keyboard_key_release(ord("M"));
if (k == 9) {
    keyboard_key = 0;
    keyboard_lastkey = 0;
}
if (k == 10) keyboard_key_press(ord("N"));
if (k == 12) keyboard_key_release(ord("N"));
if (k == 14) { out("end", "ok"); game_end(); }
