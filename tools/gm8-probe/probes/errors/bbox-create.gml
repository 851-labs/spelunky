// Creating an instance computes its bounding box at the end of its constructor (Spelunky.exe 0x4ab59d), which the
// room's add routine (0x4aedb7) calls after counting the new instance in its list and its id, and before storing it
// there. So instance_create of an `oBlack` (a sprite, no events) at x = 2^63 traps with the list's last slot never
// filled: does the game go on (the lines of steps 2 and 3), and with how many oBlacks and which next id? (A step
// probe.)
global.probe_id = 0;
// step:
var p;
p = 1; repeat (63) p = p * 2;
if (global.probe_step == 1) {
    global.probe_id = instance_create(0, 0, core);
    out("bbox_create_1", 1);
    instance_create(p, 0, oBlack);
}
if (global.probe_step >= 2) {
    out("bbox_create_blacks_" + string(global.probe_step), instance_number(oBlack));
    out("bbox_create_ids_" + string(global.probe_step), instance_create(0, 0, core) - global.probe_id);
}
if (global.probe_step == 3) {
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundLength:
return 1;
