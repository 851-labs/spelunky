// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if (sprite_index = sBombArmed) depth = 49;
if (sticky) depth = 1;

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
