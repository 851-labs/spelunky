// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if ((x > view_xview[0]-20 and x < view_xview[0] + view_wview[0]+4 and
        y > view_yview[0]-20 and y < view_yview[0] + view_hview[0]+4))
{
moveTo(xVel,yVel);

yVel += myGrav;
if (yVel > yVelLimit) yVel = yVelLimit;

if (collision_point(x+8, y+8, oSolid, 0, 0))
{
    hp = 0;
}

if (hp < 1)
{
    scrCreateBlood(x+8, y+8, 3);
    if (countsAsKill)
    {
        if (isRealLevel()) global.enemyKills[2] += 1;
        global.spiders += 1;
        global.kills += 1;
    }
    instance_destroy();
}

if (isCollisionRight(1))
{
    xVel = 1;
}

if (isCollisionLeft(1))
{
    xVel = -1;
}

dist = distance_to_object(oCharacter)

if (status == IDLE)
{
    alarm[0] = rand(5,20);
    status = RECOVER;
}
else if (status == RECOVER)
{
    if (isCollisionBottom(1)) xVel = 0;
}
else if (status == BOUNCE and dist < 90)
{
    if (isCollisionBottom(1))
    {
        yVel = -1 * rand(2,5);
        if (oCharacter.x < x+8)
        {
            xVel = -2.5;
        }
        else
        {
            xVel = 2.5;
        }
        
        if (rand(1,4) == 1) { status = IDLE; xVel = 0; yVel = 0; }
    }
}
else if (status != DROWNED)
{
    status = IDLE;
    //xVel = 0;
}

if (isCollisionTop(1))
    yVel = 1;
/*
if (isCollisionLeft(1) or isCollisionRight(1))
{
    xVel = -xVel;
}
*/

//if (isCollisionSolid())
//  y -= 2;
}
// ==== action 2: lib 1 id 603: code ====
if (collision_point(x+8, y+8, oWater, 0, 0) and status != DROWNED)
{
    status = DROWNED;
    sprite_index = sSpiderDrowning;
    alarm[1] = 30;
    xVel = 0;
    yVel = 0.2;
    instance_create(x+8, y, oSplash);
    playSound(global.sndSplash);
}
