life = global.plife;
if (life < 0) life = 0;
draw_set_font(global.myFont);
draw_set_color(c_white);
draw_sprite(sHeart, -1, view_xview[0]+8, view_yview[0]+8);
draw_text(view_xview[0]+24, view_yview[0]+8, life);
draw_sprite(sShopkeeperIcon, -1, view_xview[0]+64, view_yview[0]+8);
draw_text(view_xview[0]+64+16, view_yview[0]+8, kills);
if (drawStatus < 3)
{
    draw_set_font(global.myFontSmall);
    draw_set_color(c_yellow);
    strLen = string_length("SHOTGUN CHALLENGE BEGINS IN 3...")*8;
    n = 320 - strLen;
    n = ceil(n / 2);
    draw_text(n, 216, "SHOTGUN CHALLENGE BEGINS IN " + string(3-drawStatus) + "...");
}
