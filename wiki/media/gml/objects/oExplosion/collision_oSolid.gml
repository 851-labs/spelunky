if (isLevel("rTutorial") or (x > view_xview[0]-16 and x < view_xview[0] + view_wview[0]+16 and
     y > view_yview[0]-16 and y < view_yview[0] + view_hview[0]+16))
{
    with other
    {
        repeat (2)
        {
            tile = tile_layer_find(3, x+1, y-1);
            if (tile) tile_delete(tile);
        }
        tile = tile_layer_find(3, x+1, y+16);
        if (tile) tile_delete(tile);
    
        if (not invincible) instance_destroy();
    }

    with oTreasure
    {
        state = 1;
    }

    with oSpikes
    {
        if (not collision_point(x, y+16, oSolid, 0, 0))
        {
            instance_destroy();
        }
    }

//global.checkWater = true;
}
