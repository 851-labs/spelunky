// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if (!instance_exists(enemyID))
{
    enemyID = 0;
}

if (enemyID != 0)
{
    x = enemyID.x-stickyXDiff;
    y = enemyID.y-stickyYDiff;
}
