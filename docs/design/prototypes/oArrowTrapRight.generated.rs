// Output of the transpiler prototype (crates/gml/examples/transpile), unedited. Regenerate with:
//   scripts/cargo run -p gml --example transpile -- oArrowTrapRight/

use crate::rt::*;
use crate::{b, bgr, gv, k, obj, rm, scr, scr_id, spr, v};

/// GML: `oArrowTrapRight/create#1`.
pub fn create_a1(g: &mut Game, cx: &mut Ctx) -> R<()> {
    // 1: type = "Arrow Trap";
    g.at("oArrowTrapRight/create#1", 1);
    { let x = s("Arrow Trap"); g.set(cx.this, v::r#type, x)?; }
    // 2: facing = 1;
    g.at("oArrowTrapRight/create#1", 2);
    { let x = r(1.0); g.set(cx.this, v::facing, x)?; }
    // 3: fired = 0;
    g.at("oArrowTrapRight/create#1", 3);
    { let x = r(0.0); g.set(cx.this, v::fired, x)?; }
    // 4: invincible = false;
    g.at("oArrowTrapRight/create#1", 4);
    { let x = r(0.0); g.set(cx.this, v::invincible, x)?; }
    // 5: xAct = 0;
    g.at("oArrowTrapRight/create#1", 5);
    { let x = r(0.0); g.set(cx.this, v::xAct, x)?; }
    // 7: alarm[1] = 1;
    g.at("oArrowTrapRight/create#1", 7);
    {
        let i = 1;
        let j = 0;
        let x = r(1.0);
        g.set_a(cx.this, v::alarm, i, j, x)?;
    }
    Ok(())
}

/// GML: `oArrowTrapRight/destroy#0`.
pub fn destroy_a0(g: &mut Game, cx: &mut Ctx) -> R<()> {
    // 1: if (!cleanDeath && !global.cleanSolids)
    g.at("oArrowTrapRight/destroy#0", 1);
    if and(truthy(&not(g.get(cx.this, v::cleanDeath)?)?), truthy(&not(g.gget(gv::cleanSolids)?)?)) {
        // 3: rubble = instance_create(x + 8 + rand(0, 8) - rand(0, 8), y + 8 + rand(0, 8) - rand(0, 8), oRubble);
        g.at("oArrowTrapRight/destroy#0", 3);
        { let x = { let a = [sub(add(add(g.get(cx.this, v::x)?, r(8.0))?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, sub(add(add(g.get(cx.this, v::y)?, r(8.0))?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, obj::oRubble]; b::instance_create(g, cx, &a)? }; g.set(cx.this, v::rubble, x)?; }
        // 4: rubble.sprite_index = sRubbleTan;
        g.at("oArrowTrapRight/destroy#0", 4);
        {
            let t = g.get(cx.this, v::rubble)?;
            let x = spr::sRubbleTan;
            g.set_on(cx, &t, v::sprite_index, x)?;
        }
        // 5: rubble = instance_create(x + 8 + rand(0, 8) - rand(0, 8), y + 8 + rand(0, 8) - rand(0, 8), oRubbleSm
        g.at("oArrowTrapRight/destroy#0", 5);
        { let x = { let a = [sub(add(add(g.get(cx.this, v::x)?, r(8.0))?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, sub(add(add(g.get(cx.this, v::y)?, r(8.0))?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, obj::oRubbleSmall]; b::instance_create(g, cx, &a)? }; g.set(cx.this, v::rubble, x)?; }
        // 6: rubble.sprite_index = sRubbleTanSmall;
        g.at("oArrowTrapRight/destroy#0", 6);
        {
            let t = g.get(cx.this, v::rubble)?;
            let x = spr::sRubbleTanSmall;
            g.set_on(cx, &t, v::sprite_index, x)?;
        }
        // 7: rubble = instance_create(x + 8 + rand(0, 8) - rand(0, 8), y + 8 + rand(0, 8) - rand(0, 8), oRubbleSm
        g.at("oArrowTrapRight/destroy#0", 7);
        { let x = { let a = [sub(add(add(g.get(cx.this, v::x)?, r(8.0))?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, sub(add(add(g.get(cx.this, v::y)?, r(8.0))?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, { let a = [r(0.0), r(8.0)]; scr::rand(g, cx, &a)? })?, obj::oRubbleSmall]; b::instance_create(g, cx, &a)? }; g.set(cx.this, v::rubble, x)?; }
        // 8: rubble.sprite_index = sRubbleTanSmall;
        g.at("oArrowTrapRight/destroy#0", 8);
        {
            let t = g.get(cx.this, v::rubble)?;
            let x = spr::sRubbleTanSmall;
            g.set_on(cx, &t, v::sprite_index, x)?;
        }
        // 9: if (fired == 0)
        g.at("oArrowTrapRight/destroy#0", 9);
        if eq(&g.get(cx.this, v::fired)?, &r(0.0))? {
            // 9: instance_create(x + 8, y + 8, oArrow);
            g.at("oArrowTrapRight/destroy#0", 9);
            { let a = [add(g.get(cx.this, v::x)?, r(8.0))?, add(g.get(cx.this, v::y)?, r(8.0))?, obj::oArrow]; b::instance_create(g, cx, &a)? };
        }
    }
    Ok(())
}

/// GML: `oArrowTrapRight/alarm_1#0`.
pub fn alarm_1_a0(g: &mut Game, cx: &mut Ctx) -> R<()> {
    // 1: if (!isRoom("rLevelEditor"))
    g.at("oArrowTrapRight/alarm_1#0", 1);
    if truthy(&not({ let a = [s("rLevelEditor")]; scr::isRoom(g, cx, &a)? })?) {
        // 4: xAct = x + 16;
        g.at("oArrowTrapRight/alarm_1#0", 4);
        { let x = add(g.get(cx.this, v::x)?, r(16.0))?; g.set(cx.this, v::xAct, x)?; }
        // 5: n = 100;
        g.at("oArrowTrapRight/alarm_1#0", 5);
        { let x = r(100.0); g.set(cx.this, v::n, x)?; }
        // 6: while (!collision_point(xAct, y + 8, oSolid, 0, 0) && n > 0)
        g.at("oArrowTrapRight/alarm_1#0", 6);
        'while1: while and(truthy(&not({ let a = [g.get(cx.this, v::xAct)?, add(g.get(cx.this, v::y)?, r(8.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, &a)? })?), gt(&g.get(cx.this, v::n)?, &r(0.0))?) {
            // 8: if (xAct - x > 96)
            g.at("oArrowTrapRight/alarm_1#0", 8);
            if gt(&sub(g.get(cx.this, v::xAct)?, g.get(cx.this, v::x)?)?, &r(96.0))? {
                // 8: break;
                g.at("oArrowTrapRight/alarm_1#0", 8);
                break 'while1;
            }
            // 9: xAct += 1;
            g.at("oArrowTrapRight/alarm_1#0", 9);
            {
                let old = g.get(cx.this, v::xAct)?;
                let x = lenient(add, old, r(1.0));
                g.set(cx.this, v::xAct, x)?;
            }
            // 10: n -= 1;
            g.at("oArrowTrapRight/alarm_1#0", 10);
            {
                let old = g.get(cx.this, v::n)?;
                let x = lenient(sub, old, r(1.0));
                g.set(cx.this, v::n, x)?;
            }
        }
        // 12: xAct -= x + 8;
        g.at("oArrowTrapRight/alarm_1#0", 12);
        {
            let old = g.get(cx.this, v::xAct)?;
            let x = lenient(sub, old, add(g.get(cx.this, v::x)?, r(8.0))?);
            g.set(cx.this, v::xAct, x)?;
        }
        // 13: if (xAct < 32)
        g.at("oArrowTrapRight/alarm_1#0", 13);
        if lt(&g.get(cx.this, v::xAct)?, &r(32.0))? {
            // 13: xAct = 32;
            g.at("oArrowTrapRight/alarm_1#0", 13);
            { let x = r(32.0); g.set(cx.this, v::xAct, x)?; }
        }
        // 14: obj = instance_create(x + 16, y, oArrowTrapTest);
        g.at("oArrowTrapRight/alarm_1#0", 14);
        { let x = { let a = [add(g.get(cx.this, v::x)?, r(16.0))?, g.get(cx.this, v::y)?, obj::oArrowTrapTest]; b::instance_create(g, cx, &a)? }; g.set(cx.this, v::obj, x)?; }
        // 15: obj.image_xscale = ceil((xAct - 16) / 16);
        g.at("oArrowTrapRight/alarm_1#0", 15);
        {
            let t = g.get(cx.this, v::obj)?;
            let x = { let a = [div(sub(g.get(cx.this, v::xAct)?, r(16.0))?, r(16.0))?]; b::ceil(g, cx, &a)? };
            g.set_on(cx, &t, v::image_xscale, x)?;
        }
        // 16: obj.trapID = id;
        g.at("oArrowTrapRight/alarm_1#0", 16);
        {
            let t = g.get(cx.this, v::obj)?;
            let x = g.get(cx.this, v::id)?;
            g.set_on(cx, &t, v::trapID, x)?;
        }
    }
    Ok(())
}

/// GML: `oArrowTrapRight/alarm_0#0`.
pub fn alarm_0_a0(g: &mut Game, cx: &mut Ctx) -> R<()> {
    // 1: arrow = instance_create(x + 16, y + 4, oArrow);
    g.at("oArrowTrapRight/alarm_0#0", 1);
    { let x = { let a = [add(g.get(cx.this, v::x)?, r(16.0))?, add(g.get(cx.this, v::y)?, r(4.0))?, obj::oArrow]; b::instance_create(g, cx, &a)? }; g.set(cx.this, v::arrow, x)?; }
    // 2: arrow.xVel = 5;
    g.at("oArrowTrapRight/alarm_0#0", 2);
    {
        let t = g.get(cx.this, v::arrow)?;
        let x = r(5.0);
        g.set_on(cx, &t, v::xVel, x)?;
    }
    Ok(())
}

/// GML: `oArrowTrapRight/step#0`.
pub fn step_a0(g: &mut Game, cx: &mut Ctx) -> R<()> {
    // 1: firing = false;
    g.at("oArrowTrapRight/step#0", 1);
    { let x = r(0.0); g.set(cx.this, v::firing, x)?; }
    // 2: if (fired == 0)
    g.at("oArrowTrapRight/step#0", 2);
    if eq(&g.get(cx.this, v::fired)?, &r(0.0))? {
    }
    Ok(())
}

/// Event `create` of `oArrowTrapRight`: its actions in order.
pub fn create(g: &mut Game, cx: &mut Ctx) -> R<()> {
    g.event_inherited(cx)?;
    create_a1(g, cx)?;
    Ok(())
}

/// Event `destroy` of `oArrowTrapRight`: its actions in order.
pub fn destroy(g: &mut Game, cx: &mut Ctx) -> R<()> {
    destroy_a0(g, cx)?;
    Ok(())
}

/// Event `alarm_1` of `oArrowTrapRight`: its actions in order.
pub fn alarm_1(g: &mut Game, cx: &mut Ctx) -> R<()> {
    alarm_1_a0(g, cx)?;
    Ok(())
}

/// Event `alarm_0` of `oArrowTrapRight`: its actions in order.
pub fn alarm_0(g: &mut Game, cx: &mut Ctx) -> R<()> {
    alarm_0_a0(g, cx)?;
    Ok(())
}

/// Event `step` of `oArrowTrapRight`: its actions in order.
pub fn step(g: &mut Game, cx: &mut Ctx) -> R<()> {
    step_a0(g, cx)?;
    Ok(())
}
