// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if (collision_point(x+8, y+16, oLava, 0, 0) and not collision_point(x+8, y+17, oSolid, 0, 0))
{
    yVel = 0;
    myGrav = 0;
    y += 0.05;
}
if (y > 576) instance_destroy();
