// How often, and when, a key press <any key> event runs for one press (851-3773). oKeyConfig's any-key press
// event adds 1 to its status for every key but Escape (0 after its create event), so its status counts the runs.
// The key is F1: oScreen's F1 press event evaluates `paused && global.plife > 0 && isLevel()`, which calls isLevel()
// whatever paused is (no short-circuit), so the isLevel below records the count from inside the specific key's
// event. keyboard_key_press goes through GM8's message loop, so the press arrives on the next step. (Wine only:
// OpenGMK doesn't implement keyboard_key_press.)
// step:
var k;
if (global.probe_step == 1) {
    global.kc = instance_create(0, 0, oKeyConfig);
    keyboard_key_press(vk_f1);
}
if (global.probe_step >= 2) {
    k = global.kc;
    out("status_" + string(global.probe_step), k.status);
    out("pressed_any_" + string(global.probe_step), keyboard_check_pressed(vk_anykey));
}
if (global.probe_step == 4) keyboard_key_release(vk_f1);
if (global.probe_step == 6) { out("end", "ok"); game_end(); }
// script isLevel:
var k;
if (event_type == ev_keypress) {
    k = global.kc;
    out("in_keypress_" + string(event_number) + "_kc_status", k.status);
}
if (isRoom("rTutorial") || isRoom("rLoadLevel") || isRoom("rLevel") || isRoom("rLevel2") || isRoom("rLevel3") || isRoom("rOlmec")) return true;
return false;
