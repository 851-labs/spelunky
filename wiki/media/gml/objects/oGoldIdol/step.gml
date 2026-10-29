// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if ((x > view_xview[0]-8 and x < view_xview[0] + view_wview[0]+8 and
        y > view_yview[0]-8 and y < view_yview[0] + view_hview[0]+8))
{
// get money for idols at shops
if (isLevel())
{
    if (not held and
        collision_point(x, y+4, oBrickSmooth, 0, 0) and
        instance_exists(oShopkeeper) and
        global.thiefLevel == 0 and not global.murderer)
    {
        if (oShopkeeper.status == 0 and abs(x - oShopkeeper.x) < 80)
        {
            global.collect += value*(global.levelType+1);
            global.collectCounter += 20;
            if (global.collectCounter > 100) global.collectCounter = 100;
            global.idols += 1;
            playSound(global.sndCoin);
            instance_create(x, y-8, oBigCollect);
            global.message = "PLEASURE DOING BUSINESS!";
            global.message2 = "";
            global.messageTimer = 100;
            instance_destroy();
        }
    }
}
if (not colBot and trigger)
{
    trigger = false;
    global.idolsGrabbed += 1;
}
}
