// The animation stage (Spelunky.exe 0x53e775, after the draw) stores image_index + image_speed through the x87,
// then compares Delphi's Floor of it (a 32-bit Integer, whose Trunc traps from 2^63) with the frame count. Three
// `core`s with sMetalBlock, drawn and so animated in depth order (c1, c2, c3): in frame 1 c2's sum is past the
// largest double, in frame 2 its image_index is 2^63. Does c1 animate and c3 not, and does c2 keep its old index
// (the sum isn't stored) or the new one (the Floor traps after the store)? Frame 3 tests the 32-bit Floor:
// 2^32 + n + 0.5 (n frames) wraps, 2^32 + 0.5 and 2^31 + 0.5 don't. `1 / ten` checks the x87's rounding after the
// Floor's trap, which leaves its control word set to round toward zero unless the handler resets it. (A step
// probe.)
global.probe_c1 = 0;
// step:
var p, big, ten, n, c1, c2, c3;
p = 1; repeat (63) p = p * 2;
big = 1; repeat (1023) big = big * 2;
big = big * 1.5;
ten = 10;
n = sprite_get_number(sMetalBlock);
if (global.probe_step == 1) {
    c1 = instance_create(0, 0, core);
    c2 = instance_create(0, 0, core);
    c3 = instance_create(0, 0, core);
    c1.depth = 30;
    c2.depth = 20;
    c3.depth = 10;
    with (c1) { sprite_index = sMetalBlock; image_speed = 0.25; }
    with (c2) { sprite_index = sMetalBlock; image_index = big; image_speed = big; }
    with (c3) { sprite_index = sMetalBlock; image_speed = 0.25; }
    global.probe_c1 = c1;
    global.probe_c2 = c2;
    global.probe_c3 = c3;
    out("anim_frames", n);
    out("anim_c2_1", c2.image_index);
}
c1 = global.probe_c1;
c2 = global.probe_c2;
c3 = global.probe_c3;
if (global.probe_step >= 2) {
    out("anim_c1_" + string(global.probe_step), c1.image_index);
    out("anim_c2_" + string(global.probe_step), c2.image_index);
    out("anim_c3_" + string(global.probe_step), c3.image_index);
    out("anim_tenth_" + string(global.probe_step), 1 / ten);
}
if (global.probe_step == 2) {
    c2.image_speed = 0;
    c2.image_index = p;
}
if (global.probe_step == 3) {
    c1.image_speed = 0;
    c1.image_index = p / 2147483648 + n + 0.5;
    c2.image_index = p / 2147483648 + 0.5;
    c3.image_speed = 0;
    c3.image_index = p / 4294967296 + 0.5;
}
if (global.probe_step == 4) {
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundLength:
return 1;
