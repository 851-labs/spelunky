// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if (armed and instance_exists(oShopkeeper))
{
    if ((global.roomPath[scrGetRoomX(x), scrGetRoomY(y)] == 4 or
        global.roomPath[scrGetRoomX(x), scrGetRoomY(y)] == 5) and
        distance_to_object(oShopkeeper) < 96)
    {
        with oShopkeeper
        {
            scrShopkeeperAnger(2);
        }
    }
}

if (collision_point(x, y, oWaterSwim, -1, -1))
{
    if (not swimming)
    {
        instance_create(x, y, oSplash);
        swimming = true;
        playSound(global.sndSplash);
    }
}
else
{
    swimming = false;
}
