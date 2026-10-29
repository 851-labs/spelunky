// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if ((x > view_xview[0]-20 and x < view_xview[0] + view_wview[0]+4 and
        y > view_yview[0]-20 and y < view_yview[0] + view_hview[0]+4))
{
moveTo(xVel,yVel);

yVel += myGrav;
if (yVel > yVelLimit) yVel = yVelLimit;

if (collision_point(x+8, y+8, oSolid, 0, 0)) hp = -999;

if (hp < 1)
{
    scrCreateBlood(x+8, y+8, 3);
    if (countsAsKill)
    {
        if (isRealLevel()) global.enemyKills[8] += 1;
        global.frogs += 1;
        global.kills += 1;
    }
    instance_destroy();
}

if (isCollisionRight(1))
{
    xVel = -1;
}

if (isCollisionLeft(1))
{
    xVel = 1;
}

colBot = false;
if (isCollisionBottom(1)) colBot = true;

dist = distance_to_object(oCharacter)

if (status == IDLE)
{
    xVel = 0;
    if (counter > 0) counter -= 1;
    else if (dist < 64) status = BOUNCE;
    //if (dist < 48) status = BOUNCE;
    if (status == BOUNCE) playSound(global.sndFrog);
}
else if (status == RECOVER)
{
    if (colBot)
    {
        status = IDLE;
        xVel = 0;
        yVel = 0;
        counter = rand(10,40);
    }
}
else if (status == BOUNCE)
{
    if (colBot)
    {
        yVel = -1 * rand(2,4);
        if (oCharacter.x < x)
        {
            facing = LEFT;
            xVel = -3;
        }
        else
        {
            facing = RIGHT;
            xVel = 3;
        }
    }
    else
    {
        status = RECOVER;
    }
}
else if (status != DROWNED)
{
    status = IDLE;
    xVel = 0;
}

if (isCollisionTop(1))
    yVel = 1;

//if (isCollisionSolid())
//  y -= 2;
  
if (not colBot) sprite_index = sFrogJumpL;
else sprite_index = sFrogLeft;
}
