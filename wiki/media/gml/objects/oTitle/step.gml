if (keyboard_check_pressed(vk_enter) or
    keyboard_check_pressed(vk_escape) or
    checkAttackPressed() or
    checkStartPressed())
{
    if (instance_exists(oScreen))
    {
        if (not oScreen.canPause)
        {
            if (gamepad.attackPressed) gamepad.attackPressed = false;
            if (gamepad.startPressed) gamepad.startPressed = false;
            global.titleStart = 2;
            room_restart();
        }
    }
    else
    {
        if (gamepad.attackPressed) gamepad.attackPressed = false;
        if (gamepad.startPressed) gamepad.startPressed = false;
        global.titleStart = 2;
        room_restart();
    }
}

if (instance_exists(oPlayer1))
{
    if (oPlayer1.x <= 320 and view_xview[0] > 0)
    {
        view_xview[0] -= 8;
    }
    
    if (oPlayer1.x > 320 and view_xview[0] < 320)
    {
        view_xview[0] += 8;
    }
}

if (fadeOut)
{
    if (darkness < 1) darkness += 0.1;
    else
    {
        room_goto(rCredits1);
    }
}
