// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if (status == DEAD)
{
    if (deathTimer > 0) deathTimer -= 1;
    else
    {
        scrCreateBlood(x+8, y+8, 3);
        instance_destroy();
    }
}
