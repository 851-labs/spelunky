// shake the screen
if (global.shake > 0)
{
    //view_xview[0] = view_xview[0] + rand(0,3) - rand(0,3);
    //view_yview[0] = view_yview[0] + rand(0,3) - rand(0,3);
    //if (view_yview[0] > 16) view_yview[0] = 16 - rand(0,8);
    //if (view_yview[0] < 0) view_yview[0] = 0 + rand(0,8);
    if (shakeToggle) view_yview[0] = view_yview[0] + rand(1,8);
    else view_yview[0] = 0;
    shakeToggle = not shakeToggle;
    global.shake -= 1;
}
else
{
    view_xview[0] = 0;
    view_yview[0] = 0;
}
