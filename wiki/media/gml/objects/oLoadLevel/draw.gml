draw_set_font(global.myFontSmall);
draw_set_color(c_white);

if (status == GET_FILE_NAME)
{
    draw_text(view_xview[0]+32, view_yview[0]+64, "LOAD LEVEL: " + levelName);
    draw_set_color(c_yellow);
    draw_text(view_xview[0]+40, view_yview[0]+96, "LEVELS");
    draw_text(view_xview[0]+224, view_yview[0]+96, string(page) + "/" + string(numPages));
    if (msgTimer > 0)
    {
        draw_set_color(c_red);
        draw_text(view_xview[0]+32, view_yview[0]+224, "NO SUCH LEVEL EXISTS!");
    }
    else draw_text(view_xview[0]+32, view_yview[0]+224, "PRESS ESCAPE TO RETURN TO TITLE.");
    draw_set_color(c_white);
    n = 112;
    m = 0;
    i = (page-1)*10;
    while (m < 10 and i+m < numLevels)
    {
        draw_text(view_xview[0]+40, view_yview[0]+n, string_upper(levelList[i+m]));
        n += 8;
        m += 1;
    }
}

if (blackOut)
{
    draw_set_color(c_black);
    draw_rectangle(0, 0, 320, 240, false);
}
