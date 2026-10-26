// `sprite_index = s` (Spelunky.exe 0x4ab0f0) stores the sprite, then resets image_index to 0 if the new sprite's
// frame count is at most Delphi's Floor of it (a 32-bit Integer, whose Trunc traps from 2^63), then recomputes the
// bounding box. So an image_index of 2^32 + 1.5 floors to 1 and stays with sExplosion's frames; and at 2^63 the
// write traps: is the sprite stored, and image_index kept? (A step probe.)
global.probe_c = 0;
// step:
var p, c;
p = 1; repeat (63) p = p * 2;
if (global.probe_step == 1) {
    c = instance_create(0, 0, core);
    global.probe_c = c;
    out("sprite_floor_frames", sprite_get_number(sExplosion));
    with (c) {
        image_speed = 0;
        image_index = p / 2147483648 + 1.5;
        sprite_index = sExplosion;
    }
    out("sprite_floor_wrapped", c.image_index);
    with (c) {
        image_index = p;
        sprite_index = sMetalBlock;
    }
    out("sprite_floor_after", c.image_index);
}
c = global.probe_c;
if (global.probe_step == 2) {
    out("sprite_floor_sprite", c.sprite_index == sMetalBlock);
    out("sprite_floor_index", c.image_index);
    c.image_index = 0;
}
if (global.probe_step == 3) {
    out("end", "ok");
    game_end();
}
// script SS_GetSoundBytesPerSecond:
return 1;
// script SS_GetSoundLength:
return 1;
