if ((x > view_xview[0]-16 and x < view_xview[0] + view_wview[0] and
        y > view_yview[0]-16 and y < view_yview[0] + view_hview[0]))
{
if (not collision_point(x+8, y+16, oSolid, 0, 0))
{
    y += yVel;
    yVel += yAcc;
}

if (collision_point(x+8, y+15, oSolid, 0, 0))
{
    y -= 1;
}

if (abs(oPlayer1.y-(y+8)) < 8 and abs(oPlayer1.x-(x+8)) < 64)
{
    sprite_index = sSkeletonCreateL;
}
}
