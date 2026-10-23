// Output of the transpiler prototype (crates/gml/examples/transpile), unedited. Regenerate with:
//   scripts/cargo run -p gml --example transpile -- oSnake/

use crate::rt::*;
use crate::{b, bgr, gv, k, obj, rm, scr, scr_id, spr, v};

/// GML: `oSnake/create#1`.
pub fn create_a1(g: &mut Game, cx: &mut Ctx) -> R<()> {
    // 1: makeActive();
    g.at("oSnake/create#1", 1);
    scr::makeActive(g, cx, &[])?;
    // 2: setCollisionBounds(2, 0, 14, 16);
    g.at("oSnake/create#1", 2);
    { let a = [r(2.0), r(0.0), r(14.0), r(16.0)]; scr::setCollisionBounds(g, cx, &a)? };
    // 3: xVel = 2.5;
    g.at("oSnake/create#1", 3);
    { let x = r(2.5); g.set(cx.this, v::xVel, x)?; }
    // 4: image_speed = 0.4;
    g.at("oSnake/create#1", 4);
    { let x = r(0.4); g.set(cx.this, v::image_speed, x)?; }
    // 7: type = "Snake";
    g.at("oSnake/create#1", 7);
    { let x = s("Snake"); g.set(cx.this, v::r#type, x)?; }
    // 8: hp = 1;
    g.at("oSnake/create#1", 8);
    { let x = r(1.0); g.set(cx.this, v::hp, x)?; }
    // 9: invincible = 0;
    g.at("oSnake/create#1", 9);
    { let x = r(0.0); g.set(cx.this, v::invincible, x)?; }
    // 11: IDLE = 0;
    g.at("oSnake/create#1", 11);
    { let x = r(0.0); g.set(cx.this, v::IDLE, x)?; }
    // 12: WALK = 1;
    g.at("oSnake/create#1", 12);
    { let x = r(1.0); g.set(cx.this, v::WALK, x)?; }
    // 13: ATTACK = 2;
    g.at("oSnake/create#1", 13);
    { let x = r(2.0); g.set(cx.this, v::ATTACK, x)?; }
    // 14: STUNNED = 98;
    g.at("oSnake/create#1", 14);
    { let x = r(98.0); g.set(cx.this, v::STUNNED, x)?; }
    // 15: DEAD = 99;
    g.at("oSnake/create#1", 15);
    { let x = r(99.0); g.set(cx.this, v::DEAD, x)?; }
    // 16: status = IDLE;
    g.at("oSnake/create#1", 16);
    { let x = g.get(cx.this, v::IDLE)?; g.set(cx.this, v::status, x)?; }
    // 18: bounced = false;
    g.at("oSnake/create#1", 18);
    { let x = r(0.0); g.set(cx.this, v::bounced, x)?; }
    // 19: dead = false;
    g.at("oSnake/create#1", 19);
    { let x = r(0.0); g.set(cx.this, v::dead, x)?; }
    // 20: counter = 0;
    g.at("oSnake/create#1", 20);
    { let x = r(0.0); g.set(cx.this, v::counter, x)?; }
    // 22: LEFT = 0;
    g.at("oSnake/create#1", 22);
    { let x = r(0.0); g.set(cx.this, v::LEFT, x)?; }
    // 23: RIGHT = 1;
    g.at("oSnake/create#1", 23);
    { let x = r(1.0); g.set(cx.this, v::RIGHT, x)?; }
    // 24: facing = RIGHT;
    g.at("oSnake/create#1", 24);
    { let x = g.get(cx.this, v::RIGHT)?; g.set(cx.this, v::facing, x)?; }
    // 26: shakeCounter = 0;
    g.at("oSnake/create#1", 26);
    { let x = r(0.0); g.set(cx.this, v::shakeCounter, x)?; }
    // 27: shakeToggle = 1;
    g.at("oSnake/create#1", 27);
    { let x = r(1.0); g.set(cx.this, v::shakeToggle, x)?; }
    Ok(())
}

/// GML: `oSnake/step#1`.
pub fn step_a1(g: &mut Game, cx: &mut Ctx) -> R<()> {
    // 1: if (x > view_xview[0] - 20 && x < view_xview[0] + view_wview[0] + 4 && y > view_yview[0] - 20 && y <
    g.at("oSnake/step#1", 1);
    if and(and(and(gt(&g.get(cx.this, v::x)?, &sub(g.bget(v::view_xview, 0)?, r(20.0))?)?, lt(&g.get(cx.this, v::x)?, &add(add(g.bget(v::view_xview, 0)?, g.bget(v::view_wview, 0)?)?, r(4.0))?)?), gt(&g.get(cx.this, v::y)?, &sub(g.bget(v::view_yview, 0)?, r(20.0))?)?), lt(&g.get(cx.this, v::y)?, &add(add(g.bget(v::view_yview, 0)?, g.bget(v::view_hview, 0)?)?, r(4.0))?)?) {
        // 4: moveTo(xVel, yVel);
        g.at("oSnake/step#1", 4);
        { let a = [g.get(cx.this, v::xVel)?, g.get(cx.this, v::yVel)?]; scr::moveTo(g, cx, &a)? };
        // 6: yVel += myGrav;
        g.at("oSnake/step#1", 6);
        {
            let old = g.get(cx.this, v::yVel)?;
            let x = lenient(add, old, g.get(cx.this, v::myGrav)?);
            g.set(cx.this, v::yVel, x)?;
        }
        // 7: if (yVel > yVelLimit)
        g.at("oSnake/step#1", 7);
        if gt(&g.get(cx.this, v::yVel)?, &g.get(cx.this, v::yVelLimit)?)? {
            // 7: yVel = yVelLimit;
            g.at("oSnake/step#1", 7);
            { let x = g.get(cx.this, v::yVelLimit)?; g.set(cx.this, v::yVel, x)?; }
        }
        // 9: if (collision_point(x + 8, y + 8, oSolid, 0, 0))
        g.at("oSnake/step#1", 9);
        if truthy(&{ let a = [add(g.get(cx.this, v::x)?, r(8.0))?, add(g.get(cx.this, v::y)?, r(8.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, &a)? }) {
            // 11: hp = 0;
            g.at("oSnake/step#1", 11);
            { let x = r(0.0); g.set(cx.this, v::hp, x)?; }
        }
        // 14: if (hp < 1)
        g.at("oSnake/step#1", 14);
        if lt(&g.get(cx.this, v::hp)?, &r(1.0))? {
            // 16: scrCreateBlood(x + 8, y + 8, 3);
            g.at("oSnake/step#1", 16);
            { let a = [add(g.get(cx.this, v::x)?, r(8.0))?, add(g.get(cx.this, v::y)?, r(8.0))?, r(3.0)]; scr::scrCreateBlood(g, cx, &a)? };
            // 17: if (countsAsKill)
            g.at("oSnake/step#1", 17);
            if truthy(&g.get(cx.this, v::countsAsKill)?) {
                // 19: if (isRealLevel())
                g.at("oSnake/step#1", 19);
                if truthy(&scr::isRealLevel(g, cx, &[])?) {
                    // 19: global.enemyKills[1] += 1;
                    g.at("oSnake/step#1", 19);
                    {
                        let i = 1;
                        let j = 0;
                        let old = g.gget_a(gv::enemyKills, i, j)?;
                        let x = lenient(add, old, r(1.0));
                        g.gset_a(gv::enemyKills, i, j, x)?;
                    }
                }
                // 20: global.snakes += 1;
                g.at("oSnake/step#1", 20);
                {
                    let old = g.gget(gv::snakes)?;
                    let x = lenient(add, old, r(1.0));
                    g.gset(gv::snakes, x)?;
                }
                // 21: global.kills += 1;
                g.at("oSnake/step#1", 21);
                {
                    let old = g.gget(gv::kills)?;
                    let x = lenient(add, old, r(1.0));
                    g.gset(gv::kills, x)?;
                }
            }
            // 23: instance_destroy();
            g.at("oSnake/step#1", 23);
            b::instance_destroy(g, cx, &[])?;
        }
        // 26: if (isCollisionBottom(1) && status != STUNNED)
        g.at("oSnake/step#1", 26);
        if and(truthy(&{ let a = [r(1.0)]; scr::isCollisionBottom(g, cx, &a)? }), ne(&g.get(cx.this, v::status)?, &g.get(cx.this, v::STUNNED)?)?) {
            // 26: yVel = 0;
            g.at("oSnake/step#1", 26);
            { let x = r(0.0); g.set(cx.this, v::yVel, x)?; }
        }
        // 28: if (status == IDLE)
        g.at("oSnake/step#1", 28);
        if eq(&g.get(cx.this, v::status)?, &g.get(cx.this, v::IDLE)?)? {
            // 30: if (counter > 0)
            g.at("oSnake/step#1", 30);
            if gt(&g.get(cx.this, v::counter)?, &r(0.0))? {
                // 30: counter -= 1;
                g.at("oSnake/step#1", 30);
                {
                    let old = g.get(cx.this, v::counter)?;
                    let x = lenient(sub, old, r(1.0));
                    g.set(cx.this, v::counter, x)?;
                }
            } else {
                // 33: facing = rand(0, 1);
                g.at("oSnake/step#1", 33);
                { let x = { let a = [r(0.0), r(1.0)]; scr::rand(g, cx, &a)? }; g.set(cx.this, v::facing, x)?; }
                // 34: status = WALK;
                g.at("oSnake/step#1", 34);
                { let x = g.get(cx.this, v::WALK)?; g.set(cx.this, v::status, x)?; }
            }
        } else {
            // 37: if (status == WALK)
            g.at("oSnake/step#1", 37);
            if eq(&g.get(cx.this, v::status)?, &g.get(cx.this, v::WALK)?)? {
                // 39: if (isCollisionLeft(1) || isCollisionRight(1))
                g.at("oSnake/step#1", 39);
                if or(truthy(&{ let a = [r(1.0)]; scr::isCollisionLeft(g, cx, &a)? }), truthy(&{ let a = [r(1.0)]; scr::isCollisionRight(g, cx, &a)? })) {
                    // 41: if (facing == LEFT)
                    g.at("oSnake/step#1", 41);
                    if eq(&g.get(cx.this, v::facing)?, &g.get(cx.this, v::LEFT)?)? {
                        // 41: facing = RIGHT;
                        g.at("oSnake/step#1", 41);
                        { let x = g.get(cx.this, v::RIGHT)?; g.set(cx.this, v::facing, x)?; }
                    } else {
                        // 42: facing = LEFT;
                        g.at("oSnake/step#1", 42);
                        { let x = g.get(cx.this, v::LEFT)?; g.set(cx.this, v::facing, x)?; }
                    }
                }
                // 45: if (facing == LEFT && !collision_point(x - 1, y + 16, oSolid, -1, -1))
                g.at("oSnake/step#1", 45);
                if and(eq(&g.get(cx.this, v::facing)?, &g.get(cx.this, v::LEFT)?)?, !truthy(&{ let a = [sub(g.get(cx.this, v::x)?, r(1.0))?, add(g.get(cx.this, v::y)?, r(16.0))?, obj::oSolid, r(-1.0), r(-1.0)]; b::collision_point(g, cx, &a)? })) {
                    // 47: facing = RIGHT;
                    g.at("oSnake/step#1", 47);
                    { let x = g.get(cx.this, v::RIGHT)?; g.set(cx.this, v::facing, x)?; }
                } else {
                    // 49: if (facing == RIGHT && !collision_point(x + 16, y + 16, oSolid, -1, -1))
                    g.at("oSnake/step#1", 49);
                    if and(eq(&g.get(cx.this, v::facing)?, &g.get(cx.this, v::RIGHT)?)?, !truthy(&{ let a = [add(g.get(cx.this, v::x)?, r(16.0))?, add(g.get(cx.this, v::y)?, r(16.0))?, obj::oSolid, r(-1.0), r(-1.0)]; b::collision_point(g, cx, &a)? })) {
                        // 51: facing = LEFT;
                        g.at("oSnake/step#1", 51);
                        { let x = g.get(cx.this, v::LEFT)?; g.set(cx.this, v::facing, x)?; }
                    }
                }
                // 54: if (!collision_point(x - 1, y + 16, oSolid, -1, -1) || collision_point(x - 1, y, oSolid, -1, -1) && 
                g.at("oSnake/step#1", 54);
                if and(or(!truthy(&{ let a = [sub(g.get(cx.this, v::x)?, r(1.0))?, add(g.get(cx.this, v::y)?, r(16.0))?, obj::oSolid, r(-1.0), r(-1.0)]; b::collision_point(g, cx, &a)? }), truthy(&{ let a = [sub(g.get(cx.this, v::x)?, r(1.0))?, g.get(cx.this, v::y)?, obj::oSolid, r(-1.0), r(-1.0)]; b::collision_point(g, cx, &a)? })), or(!truthy(&{ let a = [add(g.get(cx.this, v::x)?, r(16.0))?, add(g.get(cx.this, v::y)?, r(16.0))?, obj::oSolid, r(-1.0), r(-1.0)]; b::collision_point(g, cx, &a)? }), truthy(&{ let a = [add(g.get(cx.this, v::x)?, r(16.0))?, g.get(cx.this, v::y)?, obj::oSolid, r(-1.0), r(-1.0)]; b::collision_point(g, cx, &a)? }))) {
                    // 57: if (collision_point(x - 1, y, oSolid, -1, -1))
                    g.at("oSnake/step#1", 57);
                    if truthy(&{ let a = [sub(g.get(cx.this, v::x)?, r(1.0))?, g.get(cx.this, v::y)?, obj::oSolid, r(-1.0), r(-1.0)]; b::collision_point(g, cx, &a)? }) {
                        // 57: facing = RIGHT;
                        g.at("oSnake/step#1", 57);
                        { let x = g.get(cx.this, v::RIGHT)?; g.set(cx.this, v::facing, x)?; }
                    } else {
                        // 58: facing = LEFT;
                        g.at("oSnake/step#1", 58);
                        { let x = g.get(cx.this, v::LEFT)?; g.set(cx.this, v::facing, x)?; }
                    }
                    // 59: xVel = 0;
                    g.at("oSnake/step#1", 59);
                    { let x = r(0.0); g.set(cx.this, v::xVel, x)?; }
                } else {
                    // 61: if (facing == LEFT)
                    g.at("oSnake/step#1", 61);
                    if eq(&g.get(cx.this, v::facing)?, &g.get(cx.this, v::LEFT)?)? {
                        // 61: xVel = -1;
                        g.at("oSnake/step#1", 61);
                        { let x = r(-1.0); g.set(cx.this, v::xVel, x)?; }
                    } else {
                        // 62: xVel = 1;
                        g.at("oSnake/step#1", 62);
                        { let x = r(1.0); g.set(cx.this, v::xVel, x)?; }
                    }
                }
                // 64: if (rand(1, 100) == 1)
                g.at("oSnake/step#1", 64);
                if eq(&{ let a = [r(1.0), r(100.0)]; scr::rand(g, cx, &a)? }, &r(1.0))? {
                    // 66: status = IDLE;
                    g.at("oSnake/step#1", 66);
                    { let x = g.get(cx.this, v::IDLE)?; g.set(cx.this, v::status, x)?; }
                    // 67: counter = rand(20, 50);
                    g.at("oSnake/step#1", 67);
                    { let x = { let a = [r(20.0), r(50.0)]; scr::rand(g, cx, &a)? }; g.set(cx.this, v::counter, x)?; }
                    // 68: xVel = 0;
                    g.at("oSnake/step#1", 68);
                    { let x = r(0.0); g.set(cx.this, v::xVel, x)?; }
                }
            } else {
                // 71: if (status == ATTACK)
                g.at("oSnake/step#1", 71);
                if eq(&g.get(cx.this, v::status)?, &g.get(cx.this, v::ATTACK)?)? {
                }
            }
        }
        // 76: if (isCollisionSolid())
        g.at("oSnake/step#1", 76);
        if truthy(&scr::isCollisionSolid(g, cx, &[])?) {
            // 77: y -= 2;
            g.at("oSnake/step#1", 77);
            {
                let old = g.get(cx.this, v::y)?;
                let x = sub(old, r(2.0))?;
                g.set(cx.this, v::y, x)?;
            }
        }
        // 79: if (status != STUNNED)
        g.at("oSnake/step#1", 79);
        if ne(&g.get(cx.this, v::status)?, &g.get(cx.this, v::STUNNED)?)? {
            // 81: if (xVel == 0)
            g.at("oSnake/step#1", 81);
            if eq(&g.get(cx.this, v::xVel)?, &r(0.0))? {
                // 81: image_speed = 0.2;
                g.at("oSnake/step#1", 81);
                { let x = r(0.2); g.set(cx.this, v::image_speed, x)?; }
            } else {
                // 82: image_speed = 0.4;
                g.at("oSnake/step#1", 82);
                { let x = r(0.4); g.set(cx.this, v::image_speed, x)?; }
            }
            // 84: sprite_index = sSnakeWalkL;
            g.at("oSnake/step#1", 84);
            { let x = spr::sSnakeWalkL; g.set(cx.this, v::sprite_index, x)?; }
        }
    }
    Ok(())
}

/// Event `create` of `oSnake`: its actions in order.
pub fn create(g: &mut Game, cx: &mut Ctx) -> R<()> {
    g.event_inherited(cx)?;
    create_a1(g, cx)?;
    Ok(())
}

/// Event `step` of `oSnake`: its actions in order.
pub fn step(g: &mut Game, cx: &mut Ctx) -> R<()> {
    g.event_inherited(cx)?;
    step_a1(g, cx)?;
    Ok(())
}

