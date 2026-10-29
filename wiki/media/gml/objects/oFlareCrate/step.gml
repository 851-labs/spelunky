// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if (collision_point(x, y, oWater, -1, -1))
{
    instance_create(x, y, oSplash);
    playSound(global.sndSplash);
    if (held)
    {
        with oPlayer1
        {
            holdItem = 0;
            pickupItemType = "";
        }
        held = false;
    }
    instance_create(x, y, oPoof);
    instance_destroy();
}
