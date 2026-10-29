// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if (xVel > 0 and yVel < 0)
{
    direction = radtodeg(arctan(-yVel/xVel));
    // direction = 45;
}
else if (xVel < 0 and yVel < 0)
{
    direction = 180 - radtodeg(arctan(-yVel/-xVel));

}
else if (xVel > 0 and yVel > 0)
{
    direction = radtodeg(arctan(yVel/xVel));

}
else if (xVel < 0 and yVel > 0)
{
    direction = 180 + radtodeg(arctan(yVel/-xVel));
}
else if (xVel < 0) direction = 180;
else direction = 0;

image_angle = direction;
