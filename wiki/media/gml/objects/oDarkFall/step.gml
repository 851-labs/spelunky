// ==== action 0: lib 1 id 604: action_inherited() ====
// ==== action 1: lib 1 id 603: code ====
if isCollisionCharacterTop(1)
{
    timeFall -= 1
    if (timeFall <= 0) yAcc = grav;
}
else if (timeFall < timeFallMax) timeFall += 1;

if (yVel > 10) yVel = 10;
