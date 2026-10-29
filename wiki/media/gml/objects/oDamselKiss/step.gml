// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if ((sprite_index == sDamselKissL or sprite_index = sPKissL) and image_index == 7)
{
    instance_create(x-8, y-8, oHeart);
    playSound(global.sndKiss);
}
