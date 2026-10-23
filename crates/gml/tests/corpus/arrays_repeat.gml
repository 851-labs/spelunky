// From scripts/scrLevelGen and scripts/scrCreateBlood: 2-D global arrays, `and`/`not` chains, `repeat`
// with a script argument.
global.startRoomX = rand(0,3);
roomX = global.startRoomX;
roomY = 0;
global.roomPath[roomX, roomY] = 1;
if (global.levelType == 1 and not global.madeBlackMarket and global.genBlackMarket)
{
    global.roomPath[0, 0] = 2;
    global.roomPath[0, 1] = 2;
}
if (bloodless) return 0;
repeat(argument2)
{
    if (instance_number(oDetritus) < 16)
    {
        instance_create(argument0, argument1, oBlood);
    }
}
