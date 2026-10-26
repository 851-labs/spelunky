// After the draw, each of the room's 8 background layers scrolls (Spelunky.exe 0x4af130): x + hspeed, then
// y + vspeed, each stored through the x87, which traps past the largest double. Layers 2 to 4 (hidden) scroll 1
// pixel a frame each way; layer 3's x sum overflows in frame 1, its y sum in frame 2. Is the sum that traps left
// unstored, the y after a trapping x not scrolled, the later layers not scrolled, and the animation after it skipped
// (a `core` advances by image_speed, 1, a frame)? (A step probe.)
global.probe_c = 0;
// step:
var big, k, i;
big = 1; repeat (1023) big = big * 2;
big = big * 1.5;
k = global.probe_step;
if (k == 1) {
    global.probe_c = instance_create(0, 0, core);
    for (i = 2; i <= 4; i += 1) {
        background_visible[i] = false;
        background_x[i] = 0;
        background_y[i] = 0;
        background_hspeed[i] = 1;
        background_vspeed[i] = 1;
    }
    background_x[3] = big;
    background_hspeed[3] = big;
}
if (k >= 2) {
    for (i = 2; i <= 4; i += 1) {
        out("bg_x" + string(i) + "_" + string(k), background_x[i]);
        out("bg_y" + string(i) + "_" + string(k), background_y[i]);
    }
    out("bg_image_" + string(k), global.probe_c.image_index);
}
if (k == 2) {
    background_x[3] = 0;
    background_hspeed[3] = 1;
    background_y[3] = big;
    background_vspeed[3] = big;
}
if (k == 3) {
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundLength:
return 1;
