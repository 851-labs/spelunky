life = global.plife;
if (life < 0) life = 0;
draw_set_font(global.myFont);
draw_set_color(c_white);
draw_sprite(sHoopsIcon, -1, view_xview[0]+8, view_yview[0]+8);
draw_text(view_xview[0]+24, view_yview[0]+8, baskets);
draw_sprite(sTimerIcon, -1, view_xview[0]+64, view_yview[0]+8);
if (timer >= 0) draw_text(view_xview[0]+64+16, view_yview[0]+8, timer);
else draw_text(view_xview[0]+64+16, view_yview[0]+8, "0");
if (drawStatus < 3)
{
    draw_set_font(global.myFontSmall);
    draw_set_color(c_yellow);
    strLen = string_length("ARCHERY CHALLENGE BEGINS IN 3...")*8;
    n = 320 - strLen;
    n = ceil(n / 2);
    draw_text(n, 216, "ARCHERY CHALLENGE BEGINS IN " + string(3-drawStatus) + "...");
}
