if (fired > 0) fired -= 1;

range = 64;

if (fired == 0 and abs(oPlayer1.y-y-8) < prox and oPlayer1.x < x and
    point_distance(x+8, y+8, oPlayer1.x, oPlayer1.y) < range)
{
    instance_create(x-16, y, oSpearsLeft);
    fired = firedMax;
}

obj = instance_nearest(x, y, oEnemy);
if (obj) {
if (fired == 0 and abs(obj.y-y) < prox and obj.x < x and
    point_distance(x, y, obj.x, obj.y) < range)
{
    instance_create(x-16, y, oSpearsLeft);
    fired = firedMax;
}}

obj = instance_nearest(x, y, oMoveableSolid);
if (obj) {
if (fired == 0 and abs(obj.y-y) < prox and obj.x < x and
    point_distance(x, y, obj.x, obj.y) < range)
{
    instance_create(x-16, y, oSpearsLeft);
    fired = firedMax;
}}

obj = instance_nearest(x, y, oItem);
if (obj) {
if (fired == 0 and abs(obj.y-y-8) < prox and obj.x < x+8 and
    point_distance(x+8, y+8, obj.x, obj.y) < range)
{
    instance_create(x-16, y, oSpearsLeft);
    fired = firedMax;
}}

// right

if (fired == 0 and abs(oPlayer1.y-y-8) < prox and oPlayer1.x > x+8 and
    point_distance(x+8, y+8, oPlayer1.x, oPlayer1.y) < range)
{
    spears = instance_create(x+16, y, oSpearsLeft);
    spears.sprite_index = sSpearsRight;
    fired = firedMax;
}

obj = instance_nearest(x, y, oEnemy);
if (obj) {
if (fired == 0 and abs(obj.y-y) < prox and obj.x > x and
    point_distance(x, y, obj.x, obj.y) < range)
{
    spears = instance_create(x+16, y, oSpearsLeft);
    spears.sprite_index = sSpearsRight;
    fired = firedMax;
}}

obj = instance_nearest(x, y, oMoveableSolid);
if (obj) {
if (fired == 0 and abs(obj.y-y) < prox and obj.x > x and
    point_distance(x, y, obj.x, obj.y) < range)
{
    spears = instance_create(x+16, y, oSpearsLeft);
    spears.sprite_index = sSpearsRight;
    fired = firedMax;
}}

obj = instance_nearest(x, y, oItem);
if (obj) {
if (fired == 0 and abs(obj.y-y-8) < prox and obj.x > x+8 and
    point_distance(x+8, y+8, obj.x, obj.y) < range)
{
    spears = instance_create(x+16, y, oSpearsLeft);
    spears.sprite_index = sSpearsRight;
    fired = firedMax;
}}

if ((x > view_xview[0]-8 and x < view_xview[0] + view_wview[0]+8 and
        y > view_yview[0]-8 and y < view_yview[0] + view_hview[0]+8))
{
if (not collision_point(x, y+16, oSolid, 0, 0))
{
    instance_destroy();
}
}
