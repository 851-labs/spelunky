// ==== action 0: lib 1 id 408: action_if(global.gamepadOn) ====
// ==== action 1: lib 1 id 603: code ====
startReleased = false;
startPressed = false;
if (start)
{
    if (global.joyStartVal > 0)
        start = joystick_check_button(joyid,global.joyStartVal);
    else if (global.joyStartVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) start = true;
        else start = false;
    }
    else if (global.joyStartVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) start = true;
        else start = false;
    }
        
    if (not start) startReleased = true;
}
else
{
    if (global.joyStartVal > 0)
        start = joystick_check_button(joyid,global.joyStartVal);
    else if (global.joyStartVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) start = true;
        else start = false;
    }
    else if (global.joyStartVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) start = true;
        else start = false;
    }
    
    if (start) startPressed = true;
}

/////////////
// LEFT
/////////////

leftReleased = false;
leftPressed = false;
if (left)
{
    if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 270) or joystick_direction(joyid) == 100) left = true;
    else left = false;
    if (not left) leftReleased = true;
}
else
{
    if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 270) or joystick_direction(joyid) == 100) left = true;
    else left = false;
    if (left) leftPressed = true;
}

/////////////
// RIGHT
/////////////

rightReleased = false;
rightPressed = false;
if (right)
{
    if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 90) or joystick_direction(joyid) == 102) right = true;
    else right = false;
    if (not right) rightReleased = true;
}
else
{
    if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 90) or joystick_direction(joyid) == 102) right = true;
    else right = false;
    if (right) rightPressed = true;
}

/////////////
// UP
/////////////

upReleased = false;
upPressed = false;
if (up)
{
    if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 0) or joystick_direction(joyid) == 104) up = true;
    else up = false;
    if (not up) upReleased = true;
}
else
{
    if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 0) or joystick_direction(joyid) == 104) up = true;
    else up = false;
    if (up) upPressed = true;
}

/////////////
// DOWN
/////////////

downReleased = false;
downPressed = false;
if (down)
{
    if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 180) or joystick_direction(joyid) == 98) down = true;
    else down = false;
    if (not down) downReleased = true;
}
else
{
    if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 180) or joystick_direction(joyid) == 98) down = true;
    else down = false;
    if (down) downPressed = true;
}

/////////////
// DIAGONAL
/////////////

if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 45) or joystick_direction(joyid) == 105)
{
    right = true;
    up = true;
}

if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 135) or joystick_direction(joyid) == 99)
{
    right = true;
    down = true;
}

if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 225) or joystick_direction(joyid) == 97)
{
    left = true;
    down = true;
}

if ((joystick_has_pov(joyid) == 1 and joystick_pov(joyid) == 315) or joystick_direction(joyid) == 103)
{
    left = true;
    up = true;
}

/////////////
// JUMP
/////////////

jumpReleased = false;
jumpPressed = false;
if (jump)
{
    if (global.joyJumpVal > 0)
        jump = joystick_check_button(joyid,global.joyJumpVal);
    else if (global.joyJumpVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) jump = true;
        else jump = false;
    }
    else if (global.joyJumpVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) jump = true;
        else jump = false;
    }
    
    if (not jump) jumpReleased = true;
}
else
{
    if (global.joyJumpVal > 0)
        jump = joystick_check_button(joyid,global.joyJumpVal);
    else if (global.joyJumpVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) jump = true;
        else jump = false;
    }
    else if (global.joyJumpVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) jump = true;
        else jump = false;
    }
    
    if (jump) jumpPressed = true;
}

/////////////
// ATTACK
/////////////

attackReleased = false;
attackPressed = false;
if (attack)
{
    if (global.joyAttackVal > 0)
        attack = joystick_check_button(joyid,global.joyAttackVal);
    else if (global.joyAttackVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) attack = true;
        else attack = false;
    }
    else if (global.joyAttackVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) attack = true;
        else attack = false;
    }
    
    if (not attack) attackReleased = true;
}
else
{
    if (global.joyAttackVal > 0)
        attack = joystick_check_button(joyid,global.joyAttackVal);
    else if (global.joyAttackVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) attack = true;
        else attack = false;
    }
    else if (global.joyAttackVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) attack = true;
        else attack = false;
    }
    
    if (attack) attackPressed = true;
}

////////////
// ITEM
////////////

itemReleased = false;
itemPressed = false;
if (item)
{
    if (global.joyItemVal > 0)
        item = joystick_check_button(joyid,global.joyItemVal);
    else if (global.joyItemVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) item = true;
        else item = false;
    }
    else if (global.joyItemVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) item = true;
        else item = false;
    }
    
    if (not item) itemReleased = true;
}
else
{
    if (global.joyItemVal > 0)
        item = joystick_check_button(joyid,global.joyItemVal);
    else if (global.joyItemVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) item = true;
        else item = false;
    }
    else if (global.joyItemVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) item = true;
        else item = false;
    }
    
    if (item) itemPressed = true;
}

////////////
// RUN
////////////

if (global.joyRunVal > 0)
    run = joystick_check_button(joyid,global.joyRunVal);
else if (global.joyRunVal == -1)
{
    if (joystick_zpos(joyid) > 0.1) run = true;
    else run = false;
}
else if (global.joyRunVal == -2)
{
    if (joystick_zpos(joyid) < -0.1) run = true;
    else run = false;
}

////////////
// BOMB
////////////

bombReleased = false;
bombPressed = false;
if (bomb)
{
    if (global.joyBombVal > 0)
        bomb = joystick_check_button(joyid,global.joyBombVal);
    else if (global.joyBombVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) bomb = true;
        else bomb = false;
    }
    else if (global.joyBombVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) bomb = true;
        else bomb = false;
    }
    
    if (not bomb) bombReleased = true;
}
else
{
    if (global.joyBombVal > 0)
        bomb = joystick_check_button(joyid,global.joyBombVal);
    else if (global.joyBombVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) bomb = true;
        else bomb = false;
    }
    else if (global.joyBombVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) bomb = true;
        else bomb = false;
    }
    
    if (bomb) bombPressed = true;
}

////////////
// ROPE
////////////

ropeReleased = false;
ropePressed = false;
if (rope)
{
    if (global.joyRopeVal > 0)
        rope = joystick_check_button(joyid,global.joyRopeVal);
    else if (global.joyRopeVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) rope = true;
        else rope = false;
    }
    else if (global.joyRopeVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) rope = true;
        else rope = false;
    }
    
    if (not rope) ropeReleased = true;
}
else
{
    if (global.joyRopeVal > 0)
        rope = joystick_check_button(joyid,global.joyRopeVal);
    else if (global.joyRopeVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) rope = true;
        else rope = false;
    }
    else if (global.joyRopeVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) rope = true;
        else rope = false;
    }
    
    if (rope) ropePressed = true;
}

////////////
// FLARE
////////////

flareReleased = false;
flarePressed = false;
if (flare)
{
    if (global.joyFlareVal > 0)
        flare = joystick_check_button(joyid,global.joyFlareVal);
    else if (global.joyFlareVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) flare = true;
        else flare = false;
    }
    else if (global.joyFlareVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) flare = true;
        else flare = false;
    }
    
    if (not flare) flareReleased = true;
}
else
{
    if (global.joyFlareVal > 0)
        flare = joystick_check_button(joyid,global.joyFlareVal);
    else if (global.joyFlareVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) flare = true;
        else flare = false;
    }
    else if (global.joyFlareVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) flare = true;
        else flare = false;
    }
    
    if (flare) flarePressed = true;
}

////////////
// PAY
////////////

payReleased = false;
payPressed = false;
if (pay)
{
    if (global.joyPayVal > 0)
        pay = joystick_check_button(joyid,global.joyPayVal);
    else if (global.joyPayVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) pay = true;
        else pay = false;
    }
    else if (global.joyPayVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) pay = true;
        else pay = false;
    }
    
    if (not pay) payReleased = true;
}
else
{
   if (global.joyPayVal > 0)
        pay = joystick_check_button(joyid,global.joyPayVal);
    else if (global.joyPayVal == -1)
    {
        if (joystick_zpos(joyid) > 0.1) pay = true;
        else pay = false;
    }
    else if (global.joyPayVal == -2)
    {
        if (joystick_zpos(joyid) < -0.1) pay = true;
        else pay = false;
    }
    
    if (pay) payPressed = true;
}
