// ==== action 0: lib 1 id 603: code ====
if (instance_exists(oPlayer1))
{
    if (oPlayer1.dead or isRoom("rMoon"))
    {
        if (isLevel())
        {
            if (drawStatus > 0)
            {
                draw_set_font(global.myFont);
                draw_set_color(c_yellow);
                draw_text(view_xview[0]+88, view_yview[0]+32+16, "GAME OVER");
            }
            if (drawStatus > 1)
            {
                draw_set_font(global.myFontSmall);
                draw_set_color(c_yellow);
                draw_text(view_xview[0]+88, view_yview[0]+64+16, "FINAL SCORE:");
            }
            if (drawStatus > 2)
            {
                draw_set_font(global.myFont);
                draw_set_color(c_white);
                draw_text(view_xview[0]+88, view_yview[0]+72+16, "$" + string(moneyCount));
                
                draw_set_font(global.myFontSmall);
                draw_set_color(c_yellow);
                if (global.customLevel)
                {
                    if (global.testLevel != "")
                    {
                        if (global.gamepadOn) str = "PRESS " + scrGetJoy(global.joyAttackVal) + " TO EDIT LEVEL.";
                        else str = "PRESS " + scrGetKey(global.keyAttackVal) + " TO EDIT LEVEL.";
                    }
                    else
                    {
                        if (global.gamepadOn) str = "PRESS " + scrGetJoy(global.joyAttackVal) + " TO LOAD ANOTHER LEVEL.";
                        else str = "PRESS " + scrGetKey(global.keyAttackVal) + " TO LOAD ANOTHER LEVEL.";
                    }
                    strLen = string_length(str)*8;
                    n = 320 - strLen;
                    n = ceil(n / 2);
                    draw_text(view_xview[0]+n, view_yview[0]+120, str);
                }
                else
                {
                    if (global.gamepadOn) str = "PRESS " + scrGetJoy(global.joyAttackVal) + " FOR HIGH SCORES.";
                    else str = "PRESS " + scrGetKey(global.keyAttackVal) + " FOR HIGH SCORES.";
                    strLen = string_length(str)*8;
                    n = 320 - strLen;
                    n = ceil(n / 2);
                    draw_text(view_xview[0]+n, view_yview[0]+120, str);
                }
            }
        }
        else if (isRoom("rSun"))
        {
            if (drawStatus > 0)
            {
                draw_set_font(global.myFont);
                draw_set_color(c_yellow);
                draw_text(view_xview[0]+88, view_yview[0]+32+16, "FINISHED!");
            }
            if (drawStatus > 1)
            {
                draw_set_font(global.myFontSmall);
                draw_set_color(c_yellow);
                if (oSunRoom.highscore) str = "YOU SET A NEW RECORD!";
                else str = "BETTER LUCK NEXT TIME...";
                strLen = string_length(str)*8;
                n = 320 - strLen;
                n = ceil(n / 2);
                draw_text(n, view_yview[0]+64+16, str);
            }
        }
        else if (isRoom("rMoon"))
        {
            if (oMoonRoom.timer < 0)
            {
                if (drawStatus > 0)
                {
                    draw_set_font(global.myFont);
                    draw_set_color(c_yellow);
                    draw_text(view_xview[0]+88, view_yview[0]+32+16, "FINISHED!");
                }
                if (drawStatus > 1)
                {
                    draw_set_font(global.myFontSmall);
                    draw_set_color(c_yellow);
                    if (oMoonRoom.highscore) str = "YOU SET A NEW RECORD!";
                    else str = "BETTER LUCK NEXT TIME...";
                    strLen = string_length(str)*8;
                    n = 320 - strLen;
                    n = ceil(n / 2);
                    draw_text(n, view_yview[0]+64+16, str);
                }
            }
        }
        else if (isRoom("rStars"))
        {
            if (drawStatus > 0)
            {
                draw_set_font(global.myFont);
                draw_set_color(c_yellow);
                draw_text(view_xview[0]+88, view_yview[0]+32+16, "FINISHED!");
            }
            if (drawStatus > 1)
            {
                draw_set_font(global.myFontSmall);
                draw_set_color(c_yellow);
                if (oStarsRoom.highscore) str = "YOU SET A NEW RECORD!";
                else str = "BETTER LUCK NEXT TIME...";
                strLen = string_length(str)*8;
                n = 320 - strLen;
                n = ceil(n / 2);
                draw_text(n, view_yview[0]+64+16, str);
            }
        }
    }
}
// ==== action 1: lib 1 id 524: action_color(16777215) ====
// ==== action 2: lib 1 id 603: code ====
// DEBUGGING TEXT

// Display level path
if (false and room == rLevel)
{
    for (j = 0; j < 4; j += 1)
    {
        for (i = 0; i < 4; i += 1)
        {
            draw_text(oPlayer1.x+48+i*16, oPlayer1.y-64+16+j*16, string(global.roomPath[i,j]));
        }
    }
}

if (false and instance_exists(oPlayer1))
{
    file = file_text_open_read("stats.txt");

    if (file)
    {
    file_text_readln(file);
    file_text_readln(file);
    file_text_readln(file);
    file_text_readln(file);
    str = file_text_read_string(file);
    str = string_delete(str, 1, 20);
    global.totalCrates = real(str);
    file_text_readln(file);
    str = file_text_read_string(file);
    // str = string_delete(str, 1, 20);
    // global.totalChests = real(str);
        
    draw_text(oPlayer1.x+16, oPlayer1.y-16, str);
        
    file_text_close(file);
    }
}

if (false and instance_exists(oPlayer1))
{
    draw_set_font(global.myFontSmall);
    draw_set_color(c_white);
    draw_text(oPlayer1.x+16, oPlayer1.y-16, string(oPlayer1.stunTimer));
    // if (instance_exists(oLockedChest)) { with oPlayer1 { draw_text(x+16, y-32, distance_to_object(oLockedChest)); } }
    // else { draw_text(oPlayer1.x+16, oPlayer1.y-16, "0"); }
    //if (global.madeMoai == true) draw_text(oPlayer1.x+16, oPlayer1.y-16, "TRUE");
    //else draw_text(oPlayer1.x+16, oPlayer1.y-16, "FALSE");
    //draw_text(oPlayer1.x+16, oPlayer1.y-16, string_upper(global.pickupItem));
}
// ==== action 3: lib 1 id 524: action_color(65535) ====
// ==== action 4: lib 1 id 603: code ====
// draw_text(oPlayer1.x-24, oPlayer1.y-48, string(radtodeg(arctan(1))));

// DEBUG

if (false and room == rLevel and instance_exists(oXMarket))
{
draw_text(oPlayer1.x-24, oPlayer1.y-48, string(oPlayer1.x-oXMarket.x));
draw_text(oPlayer1.x, oPlayer1.y-48, string(oPlayer1.y-oXMarket.y));
}
