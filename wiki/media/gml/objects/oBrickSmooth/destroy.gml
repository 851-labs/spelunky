// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if (not cleanDeath and not global.cleanSolids)
{
    rubble = instance_create(x+8+rand(0,8)-rand(0,8), y+8+rand(0,8)-rand(0,8), oRubble);
    rubble.sprite_index = sRubbleTan;
    rubble = instance_create(x+8+rand(0,8)-rand(0,8), y+8+rand(0,8)-rand(0,8), oRubbleSmall);
    rubble.sprite_index = sRubbleTanSmall;
    rubble = instance_create(x+8+rand(0,8)-rand(0,8), y+8+rand(0,8)-rand(0,8), oRubbleSmall);
    rubble.sprite_index = sRubbleTanSmall;
}
