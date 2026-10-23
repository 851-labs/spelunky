// From oExplosion's collision event with oEnemy: `other` as the collision partner, `with other`, and
// `other` inside `with` meaning the outer instance.
if (other.type == "Magma Man")
{
    with other
    {
        flame = instance_create(x+8, y-4, oMagma);
        flame.yVel = -rand(1,3);
        flame = instance_create(x+8, y-4, oMagma);
        flame.yVel = -rand(1,3);
        instance_destroy();
    }
}
else if (not other.invincible)
{
    other.hp -= 30;
    if (x < other.x) other.xVel = rand(4,6);
    else other.xVel = -rand(4,6);
    other.yVel = -6;
    other.burning = 50;
}
