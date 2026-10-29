if (not spriteSet)
{
    if ((collision_point(x-16, y, oTree, 0, 0) or collision_point(x-16, y, oLeaves, 0, 0))and
    (collision_point(x+16, y, oTree, 0, 0) or collision_point(x+16, y, oLeaves, 0, 0)))
    {
        sprite_index = sLeavesTop;
    }
}

if (x > view_xview[0]-16 and x < view_xview[0] + view_wview[0]+16 and
        y > view_yview[0]-16 and y < view_yview[0] + view_hview[0]+16)
{
    if (sprite_index == sLeavesTop)
    {
        if ((not collision_point(x-16, y, oTree, 0, 0) and not collision_point(x-16, y, oLeaves, 0, 0)) or
            (not collision_point(x+16, y, oTree, 0, 0) and not collision_point(x+16, y, oLeaves, 0, 0)))
        {
            instance_destroy();
        }
    }
    else if (sprite_index == sLeaves or sprite_index == sLeavesDead)
    {
        if (not collision_point(x+16, y, oTree, 0, 0) and not collision_point(x+16, y, oLeaves, 0, 0))
        {
            instance_destroy();
        }
    }
    else if (sprite_index == sLeavesRight or sprite_index == sLeavesDeadR)
    {
        if (not collision_point(x-16, y, oTree, 0, 0) and not collision_point(x-16, y, oLeaves, 0, 0))
        {
            instance_destroy();
        }
    }
}
