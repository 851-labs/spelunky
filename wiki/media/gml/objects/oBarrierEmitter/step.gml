if ((x > view_xview[0]-8 and x < view_xview[0] + view_wview[0]+8 and
        y > view_yview[0]-8 and y < view_yview[0] + view_hview[0]+8))
{
    if (not collision_point(x, y-16, oSolid, 0, 0)) instance_destroy();
}
