/*
if (global.darkLevel)
{
    draw_set_alpha(darkness);
    draw_set_color(c_black);
    draw_rectangle(view_xview[0], view_yview[0], view_xview[0]+320, view_yview[0]+240, false);
    draw_set_alpha(1);

}
*/
if (not global.darkLevel and global.messageTimer > 0)
{
    draw_set_font(global.myFontSmall);
    draw_set_color(c_white);
    strLen = string_length(global.message)*8;
    n = 320 - strLen;
    n = ceil(n / 2);
    draw_text(view_xview[0]+n, view_yview[0]+216, string(global.message));
    
    if (not isLevel("rTutorial")) draw_set_color(c_yellow);
    strLen = string_length(global.message2)*8;
    n = 320 - strLen;
    n = ceil(n / 2);
    draw_text(view_xview[0]+n, view_yview[0]+224, string(global.message2));
    
    global.messageTimer -= 1;
}
