if (status == EDIT) { oCursObj.visible = true; window_set_cursor(cr_default); }
else { oCursObj.visible = false; window_set_cursor(cr_none); }

if (status == EDIT and mouse_x > view_xview[0] and mouse_x < view_xview[0]+320 and
    mouse_y > view_yview[0] and mouse_y < view_yview[0]+240)
{
    if (mouse_x > view_xview[0]+320-16 and view_xview[0] < room_width-320) view_xview[0] += 4;
    else if (mouse_x < view_xview[0]+16 and view_xview[0] > 0) view_xview[0] -= 4;

    if (mouse_y > view_yview[0]+240-16 and view_yview[0] < room_height-240) view_yview[0] += 4;
    else if (mouse_y < view_yview[0]+16 and view_yview[0] > 0) view_yview[0] -= 4;

if (checkAttackPressed())
{
    scrTestLevel();
    room_goto(rLoadLevel);
}    
else if (mouse_x > 16 and
    mouse_x < room_width-16 and
    mouse_y > 16 and
    mouse_y < room_height-16 and
    oCursObj.x >= 16 and
    oCursObj.x < room_width-16 and
    oCursObj.y >= 16 and
    oCursObj.y < room_height-16)
{
    if (mouse_check_button(mb_left) and
        not collision_point(mouse_x, mouse_y, oEditButton, 0, 0) and
        not collision_point(mouse_x, mouse_y, oNewButton, 0, 0) and
        not collision_point(mouse_x, mouse_y, oTestButton, 0, 0))
    {
        if (dropSelect == BLOCKS)
        {
            scrCreateTile(blockArray[dropVal], oCursObj.x, oCursObj.y);
            levelArray[floor(oCursObj.x/16)-1, floor(oCursObj.y/16)-1] = blockArray[dropVal];
        }
        else if (dropSelect == ENEMIES)
        {
            scrCreateTile(enemyArray[dropVal], oCursObj.x, oCursObj.y);
            levelArray[floor(oCursObj.x/16)-1, floor(oCursObj.y/16)-1] = enemyArray[dropVal];
        }
        else if (dropSelect == TRAPS)
        {
            scrCreateTile(trapArray[dropVal], oCursObj.x, oCursObj.y);
            levelArray[floor(oCursObj.x/16)-1, floor(oCursObj.y/16)-1] = trapArray[dropVal];
        }
        else if (dropSelect == ITEMS)
        {
            scrCreateTile(lootArray[dropVal], oCursObj.x, oCursObj.y);
            levelArray[floor(oCursObj.x/16)-1, floor(oCursObj.y/16)-1] = lootArray[dropVal];
        }
    }
    else if (mouse_check_button(mb_right))
    {
        obj = collision_rectangle(oCursObj.x, oCursObj.y, oCursObj.x+15, oCursObj.y+15, oDrawnSprite, 0, 0); // instance_position(oCursObj.x+8, oCursObj.y+12, oDrawnSprite);
        // if (not obj) obj = instance_position(oCursObj.x+8, oCursObj.y+4, oDrawnSprite);
        if (obj)
        {
            with obj
            {
                if (type == "Giant Spider" or type == "Tomb Lord")
                {
                    if (x == oCursObj.x and y == oCursObj.y)
                    {
                        instance_destroy();
                    }
                }
                else instance_destroy();
            }
        }
        levelArray[floor(oCursObj.x/16)-1, floor(oCursObj.y/16)-1] = "0";
    }
}

}
