/// GML: `oPlayer1/step#0`.
pub fn step_a0(g: &mut World, cx: &mut Ctx) -> R<()> {
    let cx = &mut cx.clone();
    // 2: if (isRoom("rTitle") || isRoom("rHighscores"))
    g.at("oPlayer1/step#0", 2);
    if or(scr::isRoom(g, cx, &[s!("rTitle")])?.is_truthy(), scr::isRoom(g, cx, &[s!("rHighscores")])?.is_truthy()) {
        // 4: if (global.isTunnelMan)
        g.at("oPlayer1/step#0", 4);
        if g.gget(gv::isTunnelMan)?.is_truthy() {
            // 4: global.plife = 2;
            g.at("oPlayer1/step#0", 4);
            g.gset(gv::plife, r(2.0))?;
        } else {
            // 5: global.plife = 4;
            g.at("oPlayer1/step#0", 5);
            g.gset(gv::plife, r(4.0))?;
        }
    }
    // 8: if (global.plife > 99)
    g.at("oPlayer1/step#0", 8);
    if g.gget(gv::plife)?.gml_gt(&r(99.0))? {
        // 8: global.plife = 99;
        g.at("oPlayer1/step#0", 8);
        g.gset(gv::plife, r(99.0))?;
    }
    // 9: if (global.bombs > 99)
    g.at("oPlayer1/step#0", 9);
    if g.gget(gv::bombs)?.gml_gt(&r(99.0))? {
        // 9: global.bombs = 99;
        g.at("oPlayer1/step#0", 9);
        g.gset(gv::bombs, r(99.0))?;
    }
    // 10: if (global.rope > 99)
    g.at("oPlayer1/step#0", 10);
    if g.gget(gv::rope)?.gml_gt(&r(99.0))? {
        // 10: global.rope = 99;
        g.at("oPlayer1/step#0", 10);
        g.gset(gv::rope, r(99.0))?;
    }
    // 12: if (global.hasCape && !instance_exists(oCape))
    g.at("oPlayer1/step#0", 12);
    if and(g.gget(gv::hasCape)?.is_truthy(), b::instance_exists(g, cx, [obj::oCape])?.not()?.is_truthy()) {
        // 12: instance_create(x, y, oCape);
        g.at("oPlayer1/step#0", 12);
        { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oCape]; b::instance_create(g, cx, a)? };
    }
    // 14: if (instance_exists(oCape))
    g.at("oPlayer1/step#0", 14);
    if b::instance_exists(g, cx, [obj::oCape])?.is_truthy() {
        // 16: if (oCape.open)
        g.at("oPlayer1/step#0", 16);
        if g.get_on(cx, &obj::oCape, v::open)?.is_truthy() {
            // 16: fallTimer = 0;
            g.at("oPlayer1/step#0", 16);
            g.set(cx.this, v::fallTimer, r(0.0))?;
        }
    }
    // 20: if (redColor > 0)
    g.at("oPlayer1/step#0", 20);
    if g.get(cx.this, v::redColor)?.gml_gt(&r(0.0))? {
        // 22: if (redToggle)
        g.at("oPlayer1/step#0", 22);
        if g.get(cx.this, v::redToggle)?.is_truthy() {
            // 22: redColor -= 5;
            g.at("oPlayer1/step#0", 22);
            g.set(cx.this, v::redColor, lenient(Value::sub, g.get(cx.this, v::redColor)?, &r(5.0)))?;
        } else {
            // 23: if (redColor < 20)
            g.at("oPlayer1/step#0", 23);
            if g.get(cx.this, v::redColor)?.gml_lt(&r(20.0))? {
                // 23: redColor += 5;
                g.at("oPlayer1/step#0", 23);
                g.set(cx.this, v::redColor, lenient(Value::add, g.get(cx.this, v::redColor)?, &r(5.0)))?;
            } else {
                // 24: redToggle = true;
                g.at("oPlayer1/step#0", 24);
                g.set(cx.this, v::redToggle, r(1.0))?;
            }
        }
    } else {
        // 26: redColor = 0;
        g.at("oPlayer1/step#0", 26);
        g.set(cx.this, v::redColor, r(0.0))?;
    }
    // 28: if (holdArrow == ARROW_BOMB)
    g.at("oPlayer1/step#0", 28);
    if g.get(cx.this, v::holdArrow)?.gml_eq(&g.get(cx.this, v::ARROW_BOMB)?)? {
        // 30: if (bombArrowCounter > 0)
        g.at("oPlayer1/step#0", 30);
        if g.get(cx.this, v::bombArrowCounter)?.gml_gt(&r(0.0))? {
            // 30: bombArrowCounter -= 1;
            g.at("oPlayer1/step#0", 30);
            g.set(cx.this, v::bombArrowCounter, lenient(Value::sub, g.get(cx.this, v::bombArrowCounter)?, &r(1.0)))?;
        } else {
            // 33: instance_create(x, y, oExplosion);
            g.at("oPlayer1/step#0", 33);
            { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oExplosion]; b::instance_create(g, cx, a)? };
            // 34: if (global.graphicsHigh)
            g.at("oPlayer1/step#0", 34);
            if g.gget(gv::graphicsHigh)?.is_truthy() {
                // 36: scrCreateFlame(x, y, 3);
                g.at("oPlayer1/step#0", 36);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(3.0)]; scr::scrCreateFlame(g, cx, &a)? };
            }
            // 38: bombArrowCounter = 80;
            g.at("oPlayer1/step#0", 38);
            g.set(cx.this, v::bombArrowCounter, r(80.0))?;
            // 39: holdArrow = 0;
            g.at("oPlayer1/step#0", 39);
            g.set(cx.this, v::holdArrow, r(0.0))?;
        }
        // 42: if (isInShop(x, y))
        g.at("oPlayer1/step#0", 42);
        if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?]; scr::isInShop(g, cx, &a)? }).is_truthy() {
            // 44: scrShopkeeperAnger(2);
            g.at("oPlayer1/step#0", 44);
            scr::scrShopkeeperAnger(g, cx, &[r(2.0)])?;
        }
    }
    // 49: if (isRoom("rTitle") && state == CLIMBING && y < 32)
    g.at("oPlayer1/step#0", 49);
    if and(and(scr::isRoom(g, cx, &[s!("rTitle")])?.is_truthy(), g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::CLIMBING)?)?), g.get(cx.this, v::y)?.gml_lt(&r(32.0))?) {
        // 51: if (holdItem)
        g.at("oPlayer1/step#0", 51);
        if g.get(cx.this, v::holdItem)?.is_truthy() {
            // 53: holdItem.held = false;
            g.at("oPlayer1/step#0", 53);
            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
            // 54: holdItem = 0;
            g.at("oPlayer1/step#0", 54);
            g.set(cx.this, v::holdItem, r(0.0))?;
            // 55: pickupItemType = "";
            g.at("oPlayer1/step#0", 55);
            g.set(cx.this, v::pickupItemType, s!(""))?;
        }
        // 57: instance_create(x, y, oPDummy5);
        g.at("oPlayer1/step#0", 57);
        { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oPDummy5]; b::instance_create(g, cx, a)? };
        // 58: instance_destroy();
        g.at("oPlayer1/step#0", 58);
        b::instance_destroy(g, cx, [])?;
    }
    // 63: if (dead && !visible)
    g.at("oPlayer1/step#0", 63);
    if and(g.get(cx.this, v::dead)?.is_truthy(), g.get(cx.this, v::visible)?.not()?.is_truthy()) {
        // 65: xVel = 0;
        g.at("oPlayer1/step#0", 65);
        g.set(cx.this, v::xVel, r(0.0))?;
        // 66: yVel = 0;
        g.at("oPlayer1/step#0", 66);
        g.set(cx.this, v::yVel, r(0.0))?;
        // 67: grav = 0;
        g.at("oPlayer1/step#0", 67);
        g.set(cx.this, v::grav, r(0.0))?;
        // 68: myGrav = 0;
        g.at("oPlayer1/step#0", 68);
        g.set(cx.this, v::myGrav, r(0.0))?;
        // 69: bounced = true;
        g.at("oPlayer1/step#0", 69);
        g.set(cx.this, v::bounced, r(1.0))?;
    }
    // 74: distToNearestLightSource = 999;
    g.at("oPlayer1/step#0", 74);
    g.set(cx.this, v::distToNearestLightSource, r(999.0))?;
    // 75: if (instance_exists(oExplosion))
    g.at("oPlayer1/step#0", 75);
    if b::instance_exists(g, cx, [obj::oExplosion])?.is_truthy() {
        // 77: source = instance_nearest(x, y, oExplosion);
        g.at("oPlayer1/step#0", 77);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oExplosion]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 78: distToNearestLightSource = distance_to_object(source);
        g.at("oPlayer1/step#0", 78);
        {
            let x = { let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? };
            g.set(cx.this, v::distToNearestLightSource, x)?;
        }
        // 79: if (source.image_index <= 3)
        g.at("oPlayer1/step#0", 79);
        if g.get_on(cx, &g.get(cx.this, v::source)?, v::image_index)?.gml_le(&r(3.0))? {
            // 79: distToNearestLightSource -= source.image_index * 16;
            g.at("oPlayer1/step#0", 79);
            g.set(cx.this, v::distToNearestLightSource, lenient(Value::sub, g.get(cx.this, v::distToNearestLightSource)?, &g.get_on(cx, &g.get(cx.this, v::source)?, v::image_index)?.mul(&r(16.0))?))?;
        } else {
            // 80: distToNearestLightSource += (source.image_index - 3) * 16;
            g.at("oPlayer1/step#0", 80);
            g.set(cx.this, v::distToNearestLightSource, lenient(Value::add, g.get(cx.this, v::distToNearestLightSource)?, &g.get_on(cx, &g.get(cx.this, v::source)?, v::image_index)?.sub(&r(3.0))?.mul(&r(16.0))?))?;
        }
    }
    // 82: if (instance_exists(oLava))
    g.at("oPlayer1/step#0", 82);
    if b::instance_exists(g, cx, [obj::oLava])?.is_truthy() {
        // 84: source = instance_nearest(x, y, oLava);
        g.at("oPlayer1/step#0", 84);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLava]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 85: if (distance_to_object(source) < distToNearestLightSource)
        g.at("oPlayer1/step#0", 85);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 86: distToNearestLightSource = distance_to_object(source);
            g.at("oPlayer1/step#0", 86);
            {
                let x = { let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? };
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 88: if (instance_exists(oLamp))
    g.at("oPlayer1/step#0", 88);
    if b::instance_exists(g, cx, [obj::oLamp])?.is_truthy() {
        // 90: source = instance_nearest(x, y, oLamp);
        g.at("oPlayer1/step#0", 90);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLamp]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 91: if (distance_to_object(source) < distToNearestLightSource)
        g.at("oPlayer1/step#0", 91);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 92: distToNearestLightSource = distance_to_object(source);
            g.at("oPlayer1/step#0", 92);
            {
                let x = { let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? };
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 94: if (instance_exists(oLampItem))
    g.at("oPlayer1/step#0", 94);
    if b::instance_exists(g, cx, [obj::oLampItem])?.is_truthy() {
        // 96: source = instance_nearest(x, y, oLampItem);
        g.at("oPlayer1/step#0", 96);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLampItem]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 97: if (distance_to_object(source) < distToNearestLightSource)
        g.at("oPlayer1/step#0", 97);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 98: distToNearestLightSource = distance_to_object(source);
            g.at("oPlayer1/step#0", 98);
            {
                let x = { let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? };
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 100: if (instance_exists(oFlareCrate))
    g.at("oPlayer1/step#0", 100);
    if b::instance_exists(g, cx, [obj::oFlareCrate])?.is_truthy() {
        // 102: source = instance_nearest(x, y, oFlareCrate);
        g.at("oPlayer1/step#0", 102);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oFlareCrate]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 103: if (distance_to_object(source) < distToNearestLightSource)
        g.at("oPlayer1/step#0", 103);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 104: distToNearestLightSource = distance_to_object(source);
            g.at("oPlayer1/step#0", 104);
            {
                let x = { let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? };
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 106: if (instance_exists(oTikiTorch))
    g.at("oPlayer1/step#0", 106);
    if b::instance_exists(g, cx, [obj::oTikiTorch])?.is_truthy() {
        // 108: source = instance_nearest(x, y, oTikiTorch);
        g.at("oPlayer1/step#0", 108);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oTikiTorch]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 109: if (distance_to_object(source) + 48 < distToNearestLightSource)
        g.at("oPlayer1/step#0", 109);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).add(&r(48.0))?.gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 110: distToNearestLightSource = distance_to_object(source) + 48;
            g.at("oPlayer1/step#0", 110);
            {
                let x = ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).add(&r(48.0))?;
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 112: if (instance_exists(oArrowTrapLeftLit))
    g.at("oPlayer1/step#0", 112);
    if b::instance_exists(g, cx, [obj::oArrowTrapLeftLit])?.is_truthy() {
        // 114: source = instance_nearest(x, y, oArrowTrapLeftLit);
        g.at("oPlayer1/step#0", 114);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oArrowTrapLeftLit]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 115: if (distance_to_object(source) + 48 < distToNearestLightSource)
        g.at("oPlayer1/step#0", 115);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).add(&r(48.0))?.gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 116: distToNearestLightSource = distance_to_object(source) + 48;
            g.at("oPlayer1/step#0", 116);
            {
                let x = ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).add(&r(48.0))?;
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 118: if (instance_exists(oArrowTrapRightLit))
    g.at("oPlayer1/step#0", 118);
    if b::instance_exists(g, cx, [obj::oArrowTrapRightLit])?.is_truthy() {
        // 120: source = instance_nearest(x, y, oArrowTrapRightLit);
        g.at("oPlayer1/step#0", 120);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oArrowTrapRightLit]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 121: if (distance_to_object(source) + 48 < distToNearestLightSource)
        g.at("oPlayer1/step#0", 121);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).add(&r(48.0))?.gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 122: distToNearestLightSource = distance_to_object(source) + 48;
            g.at("oPlayer1/step#0", 122);
            {
                let x = ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).add(&r(48.0))?;
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 124: if (instance_exists(oSpearTrapLit))
    g.at("oPlayer1/step#0", 124);
    if b::instance_exists(g, cx, [obj::oSpearTrapLit])?.is_truthy() {
        // 126: source = instance_nearest(x, y, oSpearTrapLit);
        g.at("oPlayer1/step#0", 126);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oSpearTrapLit]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 127: if (distance_to_object(source) + 48 < distToNearestLightSource)
        g.at("oPlayer1/step#0", 127);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).add(&r(48.0))?.gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 128: distToNearestLightSource = distance_to_object(source) + 48;
            g.at("oPlayer1/step#0", 128);
            {
                let x = ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).add(&r(48.0))?;
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 130: if (instance_exists(oSmashTrapLit))
    g.at("oPlayer1/step#0", 130);
    if b::instance_exists(g, cx, [obj::oSmashTrapLit])?.is_truthy() {
        // 132: source = instance_nearest(x, y, oSmashTrapLit);
        g.at("oPlayer1/step#0", 132);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oSmashTrapLit]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 133: if (distance_to_object(source) + 48 < distToNearestLightSource)
        g.at("oPlayer1/step#0", 133);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).add(&r(48.0))?.gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 134: distToNearestLightSource = distance_to_object(source) + 48;
            g.at("oPlayer1/step#0", 134);
            {
                let x = ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).add(&r(48.0))?;
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 136: if (instance_exists(oShotgunBlastLeft))
    g.at("oPlayer1/step#0", 136);
    if b::instance_exists(g, cx, [obj::oShotgunBlastLeft])?.is_truthy() {
        // 138: source = instance_nearest(x, y, oShotgunBlastLeft);
        g.at("oPlayer1/step#0", 138);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oShotgunBlastLeft]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 139: if (distance_to_object(source) < distToNearestLightSource)
        g.at("oPlayer1/step#0", 139);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 140: distToNearestLightSource = distance_to_object(source);
            g.at("oPlayer1/step#0", 140);
            {
                let x = { let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? };
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 142: if (instance_exists(oShotgunBlastRight))
    g.at("oPlayer1/step#0", 142);
    if b::instance_exists(g, cx, [obj::oShotgunBlastRight])?.is_truthy() {
        // 144: source = instance_nearest(x, y, oShotgunBlastRight);
        g.at("oPlayer1/step#0", 144);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oShotgunBlastRight]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::source, x)?;
        }
        // 145: if (distance_to_object(source) < distToNearestLightSource)
        g.at("oPlayer1/step#0", 145);
        if ({ let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? }).gml_lt(&g.get(cx.this, v::distToNearestLightSource)?)? {
            // 146: distToNearestLightSource = distance_to_object(source);
            g.at("oPlayer1/step#0", 146);
            {
                let x = { let a = [g.get(cx.this, v::source)?]; b::distance_to_object(g, cx, a)? };
                g.set(cx.this, v::distToNearestLightSource, x)?;
            }
        }
    }
    // 150: if (sprite_index == sWhoaLeft || sprite_index == sDamselWhoaL || sprite_index == sTunnelWhoaL)
    g.at("oPlayer1/step#0", 150);
    if or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sWhoaLeft)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselWhoaL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelWhoaL)?) {
        // 152: if (whoaTimer > 0)
        g.at("oPlayer1/step#0", 152);
        if g.get(cx.this, v::whoaTimer)?.gml_gt(&r(0.0))? {
            // 152: whoaTimer -= 1;
            g.at("oPlayer1/step#0", 152);
            g.set(cx.this, v::whoaTimer, lenient(Value::sub, g.get(cx.this, v::whoaTimer)?, &r(1.0)))?;
        } else {
            // 153: if (holdItem)
            g.at("oPlayer1/step#0", 153);
            if g.get(cx.this, v::holdItem)?.is_truthy() {
                // 155: holdItem.held = false;
                g.at("oPlayer1/step#0", 155);
                { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
                // 156: if (facing == LEFT)
                g.at("oPlayer1/step#0", 156);
                if g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)? {
                    // 156: holdItem.xVel = -2;
                    g.at("oPlayer1/step#0", 156);
                    { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::xVel, r(-2.0))?; }
                } else {
                    // 157: holdItem.xVel = 2;
                    g.at("oPlayer1/step#0", 157);
                    { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::xVel, r(2.0))?; }
                }
                // 158: if (holdItem.type == "Damsel")
                g.at("oPlayer1/step#0", 158);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Damsel"))? {
                    // 158: playSound(global.sndDamsel);
                    g.at("oPlayer1/step#0", 158);
                    { let a = [g.gget(gv::sndDamsel)?]; scr::playSound(g, cx, &a)? };
                }
                // 159: if (holdItem.type == "Bow" && bowArmed)
                g.at("oPlayer1/step#0", 159);
                if and(g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Bow"))?, g.get(cx.this, v::bowArmed)?.is_truthy()) {
                    // 161: scrFireBow();
                    g.at("oPlayer1/step#0", 161);
                    scr::scrFireBow(g, cx, &[])?;
                }
                // 163: if (holdItem.type == pickupItemType)
                g.at("oPlayer1/step#0", 163);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&g.get(cx.this, v::pickupItemType)?)? {
                    // 165: holdItem = 0;
                    g.at("oPlayer1/step#0", 165);
                    g.set(cx.this, v::holdItem, r(0.0))?;
                    // 166: pickupItemType = "";
                    g.at("oPlayer1/step#0", 166);
                    g.set(cx.this, v::pickupItemType, s!(""))?;
                } else {
                    // 168: scrHoldItem(pickupItemType);
                    g.at("oPlayer1/step#0", 168);
                    { let a = [g.get(cx.this, v::pickupItemType)?]; scr::scrHoldItem(g, cx, &a)? };
                }
            }
        }
    } else {
        // 171: whoaTimer = whoaTimerMax;
        g.at("oPlayer1/step#0", 171);
        g.set(cx.this, v::whoaTimer, g.get(cx.this, v::whoaTimerMax)?)?;
    }
    // 174: if (firing > 0)
    g.at("oPlayer1/step#0", 174);
    if g.get(cx.this, v::firing)?.gml_gt(&r(0.0))? {
        // 174: firing -= 1;
        g.at("oPlayer1/step#0", 174);
        g.set(cx.this, v::firing, lenient(Value::sub, g.get(cx.this, v::firing)?, &r(1.0)))?;
    }
    // 177: if (collision_point(x, y, oWaterSwim, -1, -1))
    g.at("oPlayer1/step#0", 177);
    if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oWaterSwim, r(-1.0), r(-1.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
        // 179: if (!swimming)
        g.at("oPlayer1/step#0", 179);
        if g.get(cx.this, v::swimming)?.not()?.is_truthy() {
            // 181: instance_create(x, y - 8, oSplash);
            g.at("oPlayer1/step#0", 181);
            { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(8.0))?, obj::oSplash]; b::instance_create(g, cx, a)? };
            // 182: swimming = true;
            g.at("oPlayer1/step#0", 182);
            g.set(cx.this, v::swimming, r(1.0))?;
            // 183: playSound(global.sndSplash);
            g.at("oPlayer1/step#0", 183);
            { let a = [g.gget(gv::sndSplash)?]; scr::playSound(g, cx, &a)? };
        }
    }
    // 188: if (burning > 0)
    g.at("oPlayer1/step#0", 188);
    if g.get(cx.this, v::burning)?.gml_gt(&r(0.0))? {
        // 190: if (rand(1, 5) == 1)
        g.at("oPlayer1/step#0", 190);
        if scr::rand(g, cx, &[r(1.0), r(5.0)])?.gml_eq(&r(1.0))? {
            // 190: instance_create(x - 8 + rand(4, 12), y - 8 + rand(4, 12), oBurn);
            g.at("oPlayer1/step#0", 190);
            { let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?.add(&scr::rand(g, cx, &[r(4.0), r(12.0)])?)?, g.get(cx.this, v::y)?.sub(&r(8.0))?.add(&scr::rand(g, cx, &[r(4.0), r(12.0)])?)?, obj::oBurn]; b::instance_create(g, cx, a)? };
        }
        // 191: burning -= 1;
        g.at("oPlayer1/step#0", 191);
        g.set(cx.this, v::burning, lenient(Value::sub, g.get(cx.this, v::burning)?, &r(1.0)))?;
    }
    // 195: if (collision_point(x, y + 6, oLava, 0, 0))
    g.at("oPlayer1/step#0", 195);
    if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(6.0))?, obj::oLava, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
        // 197: if (!dead)
        g.at("oPlayer1/step#0", 197);
        if g.get(cx.this, v::dead)?.not()?.is_truthy() {
            // 199: if (isRealLevel())
            g.at("oPlayer1/step#0", 199);
            if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                // 199: global.miscDeaths[11] += 1;
                g.at("oPlayer1/step#0", 199);
                g.gset_a(gv::miscDeaths, 0, 11, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 11)?, &r(1.0)))?;
            }
            // 200: playSound(global.sndFlame);
            g.at("oPlayer1/step#0", 200);
            { let a = [g.gget(gv::sndFlame)?]; scr::playSound(g, cx, &a)? };
        }
        // 202: global.plife -= 99;
        g.at("oPlayer1/step#0", 202);
        g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(99.0)))?;
        // 204: xVel = 0;
        g.at("oPlayer1/step#0", 204);
        g.set(cx.this, v::xVel, r(0.0))?;
        // 205: yVel = 0.1;
        g.at("oPlayer1/step#0", 205);
        g.set(cx.this, v::yVel, r(0.1))?;
        // 206: grav = 0;
        g.at("oPlayer1/step#0", 206);
        g.set(cx.this, v::grav, r(0.0))?;
        // 207: myGrav = 0;
        g.at("oPlayer1/step#0", 207);
        g.set(cx.this, v::myGrav, r(0.0))?;
        // 208: bounced = true;
        g.at("oPlayer1/step#0", 208);
        g.set(cx.this, v::bounced, r(1.0))?;
        // 209: burning = 100;
        g.at("oPlayer1/step#0", 209);
        g.set(cx.this, v::burning, r(100.0))?;
        // 210: depth = 999;
        g.at("oPlayer1/step#0", 210);
        g.set(cx.this, v::depth, r(999.0))?;
    }
    // 214: if (global.hasJetpack && platformCharacterIs(ON_GROUND))
    g.at("oPlayer1/step#0", 214);
    if and(g.gget(gv::hasJetpack)?.is_truthy(), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()) {
        // 216: jetpackFuel = 50;
        g.at("oPlayer1/step#0", 216);
        g.set(cx.this, v::jetpackFuel, r(50.0))?;
    }
    // 220: if (y > room_height + 16 && !dead)
    g.at("oPlayer1/step#0", 220);
    if and(g.get(cx.this, v::y)?.gml_gt(&g.bget(v::room_height, 0)?.add(&r(16.0))?)?, g.get(cx.this, v::dead)?.not()?.is_truthy()) {
        // 222: if (isRealLevel())
        g.at("oPlayer1/step#0", 222);
        if scr::isRealLevel(g, cx, &[])?.is_truthy() {
            // 222: global.miscDeaths[10] += 1;
            g.at("oPlayer1/step#0", 222);
            g.gset_a(gv::miscDeaths, 0, 10, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 10)?, &r(1.0)))?;
        }
        // 223: global.plife -= 99;
        g.at("oPlayer1/step#0", 223);
        g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(99.0)))?;
        // 224: xVel = 0;
        g.at("oPlayer1/step#0", 224);
        g.set(cx.this, v::xVel, r(0.0))?;
        // 225: yVel = 0;
        g.at("oPlayer1/step#0", 225);
        g.set(cx.this, v::yVel, r(0.0))?;
        // 226: grav = 0;
        g.at("oPlayer1/step#0", 226);
        g.set(cx.this, v::grav, r(0.0))?;
        // 227: myGrav = 0;
        g.at("oPlayer1/step#0", 227);
        g.set(cx.this, v::myGrav, r(0.0))?;
        // 228: bounced = true;
        g.at("oPlayer1/step#0", 228);
        g.set(cx.this, v::bounced, r(1.0))?;
        // 229: if (holdItem)
        g.at("oPlayer1/step#0", 229);
        if g.get(cx.this, v::holdItem)?.is_truthy() {
            // 231: holdItem.visible = true;
            g.at("oPlayer1/step#0", 231);
            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::visible, r(1.0))?; }
            // 232: holdItem.held = false;
            g.at("oPlayer1/step#0", 232);
            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
            // 233: holdItem = 0;
            g.at("oPlayer1/step#0", 233);
            g.set(cx.this, v::holdItem, r(0.0))?;
            // 234: pickupItemType = "";
            g.at("oPlayer1/step#0", 234);
            g.set(cx.this, v::pickupItemType, s!(""))?;
        }
        // 236: playSound(global.sndThud);
        g.at("oPlayer1/step#0", 236);
        { let a = [g.gget(gv::sndThud)?]; scr::playSound(g, cx, &a)? };
        // 237: playSound(global.sndDie);
        g.at("oPlayer1/step#0", 237);
        { let a = [g.gget(gv::sndDie)?]; scr::playSound(g, cx, &a)? };
    }
    // 240: if (active)
    g.at("oPlayer1/step#0", 240);
    if g.get(cx.this, v::active)?.is_truthy() {
        // 242: if (stunTimer > 0 && (sprite_index == sStunL || sprite_index == sDamselStunL || sprite_index == sTun
        g.at("oPlayer1/step#0", 242);
        if and(g.get(cx.this, v::stunTimer)?.gml_gt(&r(0.0))?, or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sStunL)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselStunL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelStunL)?)) {
            // 244: image_speed = 0.4;
            g.at("oPlayer1/step#0", 244);
            g.set(cx.this, v::image_speed, r(0.4))?;
            // 245: stunTimer -= 1;
            g.at("oPlayer1/step#0", 245);
            g.set(cx.this, v::stunTimer, lenient(Value::sub, g.get(cx.this, v::stunTimer)?, &r(1.0)))?;
        }
        // 248: if (stunTimer < 1 && (sprite_index == sStunL || sprite_index == sDamselStunL || sprite_index == sTun
        g.at("oPlayer1/step#0", 248);
        if and(g.get(cx.this, v::stunTimer)?.gml_lt(&r(1.0))?, or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sStunL)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselStunL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelStunL)?)) {
            // 249: stunned = false;
            g.at("oPlayer1/step#0", 249);
            g.set(cx.this, v::stunned, r(0.0))?;
        }
        // 251: if (instance_exists(oParachute))
        g.at("oPlayer1/step#0", 251);
        if b::instance_exists(g, cx, [obj::oParachute])?.is_truthy() {
            // 251: fallTimer = 0;
            g.at("oPlayer1/step#0", 251);
            g.set(cx.this, v::fallTimer, r(0.0))?;
        }
        // 253: if (yVel > 0 && state != CLIMBING)
        g.at("oPlayer1/step#0", 253);
        if and(g.get(cx.this, v::yVel)?.gml_gt(&r(0.0))?, g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::CLIMBING)?)?) {
            // 255: fallTimer += 1;
            g.at("oPlayer1/step#0", 255);
            g.set(cx.this, v::fallTimer, lenient(Value::add, g.get(cx.this, v::fallTimer)?, &r(1.0)))?;
            // 256: if (fallTimer > 16)
            g.at("oPlayer1/step#0", 256);
            if g.get(cx.this, v::fallTimer)?.gml_gt(&r(16.0))? {
                // 256: wallHurt = 0;
                g.at("oPlayer1/step#0", 256);
                g.set(cx.this, v::wallHurt, r(0.0))?;
            }
            // 257: if (global.hasParachute && !stunned && fallTimer > 14)
            g.at("oPlayer1/step#0", 257);
            if and(and(g.gget(gv::hasParachute)?.is_truthy(), g.get(cx.this, v::stunned)?.not()?.is_truthy()), g.get(cx.this, v::fallTimer)?.gml_gt(&r(14.0))?) {
                // 259: if (!collision_point(x, y + 32, oSolid, 0, 0))
                g.at("oPlayer1/step#0", 259);
                if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(32.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy() {
                    // 261: instance_create(x - 8, y - 16, oParachute);
                    g.at("oPlayer1/step#0", 261);
                    { let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(16.0))?, obj::oParachute]; b::instance_create(g, cx, a)? };
                    // 262: fallTimer = 0;
                    g.at("oPlayer1/step#0", 262);
                    g.set(cx.this, v::fallTimer, r(0.0))?;
                    // 263: global.hasParachute = false;
                    g.at("oPlayer1/step#0", 263);
                    g.gset(gv::hasParachute, r(0.0))?;
                }
            }
        } else {
            // 267: if (platformCharacterIs(ON_GROUND) && fallTimer > 16 && !collision_rectangle(x - 8, y - 8, x + 8, y 
            g.at("oPlayer1/step#0", 267);
            if and(and(({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy(), g.get(cx.this, v::fallTimer)?.gml_gt(&r(16.0))?), ({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oSpringTrap, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).not()?.is_truthy()) {
                // 270: stunned = true;
                g.at("oPlayer1/step#0", 270);
                g.set(cx.this, v::stunned, r(1.0))?;
                // 271: if (fallTimer > 48)
                g.at("oPlayer1/step#0", 271);
                if g.get(cx.this, v::fallTimer)?.gml_gt(&r(48.0))? {
                    // 271: global.plife -= 10;
                    g.at("oPlayer1/step#0", 271);
                    g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(10.0)))?;
                } else {
                    // 272: if (fallTimer > 32)
                    g.at("oPlayer1/step#0", 272);
                    if g.get(cx.this, v::fallTimer)?.gml_gt(&r(32.0))? {
                        // 272: global.plife -= 2;
                        g.at("oPlayer1/step#0", 272);
                        g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(2.0)))?;
                    } else {
                        // 273: global.plife -= 1;
                        g.at("oPlayer1/step#0", 273);
                        g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(1.0)))?;
                    }
                }
                // 274: if (global.plife < 1)
                g.at("oPlayer1/step#0", 274);
                if g.gget(gv::plife)?.gml_lt(&r(1.0))? {
                    // 276: scrCreateBlood(x, y, 3);
                    g.at("oPlayer1/step#0", 276);
                    { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(3.0)]; scr::scrCreateBlood(g, cx, &a)? };
                    // 277: if (isRealLevel())
                    g.at("oPlayer1/step#0", 277);
                    if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                        // 277: global.miscDeaths[3] += 1;
                        g.at("oPlayer1/step#0", 277);
                        g.gset_a(gv::miscDeaths, 0, 3, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 3)?, &r(1.0)))?;
                    }
                }
                // 279: bounced = true;
                g.at("oPlayer1/step#0", 279);
                g.set(cx.this, v::bounced, r(1.0))?;
                // 280: stunTimer += 60;
                g.at("oPlayer1/step#0", 280);
                g.set(cx.this, v::stunTimer, lenient(Value::add, g.get(cx.this, v::stunTimer)?, &r(60.0)))?;
                // 281: yVel = -3;
                g.at("oPlayer1/step#0", 281);
                g.set(cx.this, v::yVel, r(-3.0))?;
                // 282: fallTimer = 0;
                g.at("oPlayer1/step#0", 282);
                g.set(cx.this, v::fallTimer, r(0.0))?;
                // 283: obj = instance_create(x - 4, y + 6, oPoof);
                g.at("oPlayer1/step#0", 283);
                {
                    let x = { let a = [g.get(cx.this, v::x)?.sub(&r(4.0))?, g.get(cx.this, v::y)?.add(&r(6.0))?, obj::oPoof]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 284: with (obj)
                g.at("oPlayer1/step#0", 284);
                {
                    let t = g.get(cx.this, v::obj)?;
                    let mut it1 = g.with_iter(cx, &t)?;
                    let outer3 = (cx.this, cx.other);
                    'with1: while let Some(h2) = it1.next(g) {
                        (cx.this, cx.other) = (h2, outer3.0);
                        // 284: xVel = -0.4;
                        g.at("oPlayer1/step#0", 284);
                        g.set(cx.this, v::xVel, r(-0.4))?;
                    }
                    (cx.this, cx.other) = outer3;
                }
                // 285: obj = instance_create(x + 4, y + 6, oPoof);
                g.at("oPlayer1/step#0", 285);
                {
                    let x = { let a = [g.get(cx.this, v::x)?.add(&r(4.0))?, g.get(cx.this, v::y)?.add(&r(6.0))?, obj::oPoof]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 286: with (obj)
                g.at("oPlayer1/step#0", 286);
                {
                    let t = g.get(cx.this, v::obj)?;
                    let mut it4 = g.with_iter(cx, &t)?;
                    let outer6 = (cx.this, cx.other);
                    'with2: while let Some(h5) = it4.next(g) {
                        (cx.this, cx.other) = (h5, outer6.0);
                        // 286: xVel = 0.4;
                        g.at("oPlayer1/step#0", 286);
                        g.set(cx.this, v::xVel, r(0.4))?;
                    }
                    (cx.this, cx.other) = outer6;
                }
                // 287: playSound(global.sndThud);
                g.at("oPlayer1/step#0", 287);
                { let a = [g.gget(gv::sndThud)?]; scr::playSound(g, cx, &a)? };
            } else {
                // 291: fallTimer = 0;
                g.at("oPlayer1/step#0", 291);
                g.set(cx.this, v::fallTimer, r(0.0))?;
                // 292: if (instance_exists(oParachute))
                g.at("oPlayer1/step#0", 292);
                if b::instance_exists(g, cx, [obj::oParachute])?.is_truthy() {
                    // 294: instance_create(x - 8, y - 16 - 8, oParaUsed);
                    g.at("oPlayer1/step#0", 294);
                    { let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(16.0))?.sub(&r(8.0))?, obj::oParaUsed]; b::instance_create(g, cx, a)? };
                    // 295: with (oParachute)
                    g.at("oPlayer1/step#0", 295);
                    {
                        let t = obj::oParachute;
                        let mut it7 = g.with_iter(cx, &t)?;
                        let outer9 = (cx.this, cx.other);
                        'with3: while let Some(h8) = it7.next(g) {
                            (cx.this, cx.other) = (h8, outer9.0);
                            // 295: instance_destroy();
                            g.at("oPlayer1/step#0", 295);
                            b::instance_destroy(g, cx, [])?;
                        }
                        (cx.this, cx.other) = outer9;
                    }
                }
            }
        }
        // 301: if (swimming && !collision_point(x, y, oLava, 0, 0))
        g.at("oPlayer1/step#0", 301);
        if and(g.get(cx.this, v::swimming)?.is_truthy(), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLava, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()) {
            // 303: fallTimer = 0;
            g.at("oPlayer1/step#0", 303);
            g.set(cx.this, v::fallTimer, r(0.0))?;
            // 304: if (bubbleTimer > 0)
            g.at("oPlayer1/step#0", 304);
            if g.get(cx.this, v::bubbleTimer)?.gml_gt(&r(0.0))? {
                // 304: bubbleTimer -= 1;
                g.at("oPlayer1/step#0", 304);
                g.set(cx.this, v::bubbleTimer, lenient(Value::sub, g.get(cx.this, v::bubbleTimer)?, &r(1.0)))?;
            } else {
                // 307: instance_create(x, y - 4, oBubble);
                g.at("oPlayer1/step#0", 307);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(4.0))?, obj::oBubble]; b::instance_create(g, cx, a)? };
                // 308: bubbleTimer = bubbleTimerMax;
                g.at("oPlayer1/step#0", 308);
                g.set(cx.this, v::bubbleTimer, g.get(cx.this, v::bubbleTimerMax)?)?;
            }
        } else {
            // 313: bubbleTimer = bubbleTimerMax;
            g.at("oPlayer1/step#0", 313);
            g.set(cx.this, v::bubbleTimer, g.get(cx.this, v::bubbleTimerMax)?)?;
        }
        // 316: if (state != DUCKTOHANG && !stunned && !dead && sprite_index != sPExit && sprite_index != sDamselExi
        g.at("oPlayer1/step#0", 316);
        if and(and(and(and(and(g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::DUCKTOHANG)?)?, g.get(cx.this, v::stunned)?.not()?.is_truthy()), g.get(cx.this, v::dead)?.not()?.is_truthy()), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sPExit)?), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sDamselExit)?), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sTunnelExit)?) {
            // 318: bounced = false;
            g.at("oPlayer1/step#0", 318);
            g.set(cx.this, v::bounced, r(0.0))?;
            // 319: characterStepEvent();
            g.at("oPlayer1/step#0", 319);
            scr::characterStepEvent(g, cx, &[])?;
        } else {
            // 321: if (state != DUCKING && state != DUCKTOHANG)
            g.at("oPlayer1/step#0", 321);
            if and(g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::DUCKING)?)?, g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::DUCKTOHANG)?)?) {
                // 323: state = STANDING;
                g.at("oPlayer1/step#0", 323);
                g.set(cx.this, v::state, g.get(cx.this, v::STANDING)?)?;
            }
        }
    }
    // 328: if (dead || stunned)
    g.at("oPlayer1/step#0", 328);
    if or(g.get(cx.this, v::dead)?.is_truthy(), g.get(cx.this, v::stunned)?.is_truthy()) {
        // 330: if (holdItem)
        g.at("oPlayer1/step#0", 330);
        if g.get(cx.this, v::holdItem)?.is_truthy() {
            // 332: if (holdItem.type == "Bow" && bowArmed)
            g.at("oPlayer1/step#0", 332);
            if and(g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Bow"))?, g.get(cx.this, v::bowArmed)?.is_truthy()) {
                // 334: scrFireBow();
                g.at("oPlayer1/step#0", 334);
                scr::scrFireBow(g, cx, &[])?;
            }
            // 336: holdItem.visible = true;
            g.at("oPlayer1/step#0", 336);
            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::visible, r(1.0))?; }
            // 337: holdItem.held = false;
            g.at("oPlayer1/step#0", 337);
            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
            // 338: if (holdItem.type == pickupItemType)
            g.at("oPlayer1/step#0", 338);
            if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&g.get(cx.this, v::pickupItemType)?)? {
                // 340: holdItem = 0;
                g.at("oPlayer1/step#0", 340);
                g.set(cx.this, v::holdItem, r(0.0))?;
                // 341: pickupItemType = "";
                g.at("oPlayer1/step#0", 341);
                g.set(cx.this, v::pickupItemType, s!(""))?;
            } else {
                // 343: scrHoldItem(pickupItemType);
                g.at("oPlayer1/step#0", 343);
                { let a = [g.get(cx.this, v::pickupItemType)?]; scr::scrHoldItem(g, cx, &a)? };
            }
        }
        // 348: if (bounced)
        g.at("oPlayer1/step#0", 348);
        if g.get(cx.this, v::bounced)?.is_truthy() {
            // 348: yVel += 1;
            g.at("oPlayer1/step#0", 348);
            g.set(cx.this, v::yVel, lenient(Value::add, g.get(cx.this, v::yVel)?, &r(1.0)))?;
        } else {
            // 349: yVel += 0.6;
            g.at("oPlayer1/step#0", 349);
            g.set(cx.this, v::yVel, lenient(Value::add, g.get(cx.this, v::yVel)?, &r(0.6000000000000001)))?;
        }
        // 351: if (isCollisionTop(1) && yVel < 0)
        g.at("oPlayer1/step#0", 351);
        if and(scr::isCollisionTop(g, cx, &[r(1.0)])?.is_truthy(), g.get(cx.this, v::yVel)?.gml_lt(&r(0.0))?) {
            // 353: yVel = -yVel * 0.8;
            g.at("oPlayer1/step#0", 353);
            g.set(cx.this, v::yVel, g.get(cx.this, v::yVel)?.neg()?.mul(&r(0.8))?)?;
        }
        // 356: if (isCollisionLeft(1) || isCollisionRight(1))
        g.at("oPlayer1/step#0", 356);
        if or(scr::isCollisionLeft(g, cx, &[r(1.0)])?.is_truthy(), scr::isCollisionRight(g, cx, &[r(1.0)])?.is_truthy()) {
            // 358: xVel = -xVel * 0.5;
            g.at("oPlayer1/step#0", 358);
            g.set(cx.this, v::xVel, g.get(cx.this, v::xVel)?.neg()?.mul(&r(0.5))?)?;
        }
        // 361: if (isCollisionBottom(1) || isCollisionPlatformBottom(1))
        g.at("oPlayer1/step#0", 361);
        if or(scr::isCollisionBottom(g, cx, &[r(1.0)])?.is_truthy(), scr::isCollisionPlatformBottom(g, cx, &[r(1.0)])?.is_truthy()) {
            // 364: if (yVel > 1)
            g.at("oPlayer1/step#0", 364);
            if g.get(cx.this, v::yVel)?.gml_gt(&r(1.0))? {
                // 364: yVel = -yVel * 0.5;
                g.at("oPlayer1/step#0", 364);
                g.set(cx.this, v::yVel, g.get(cx.this, v::yVel)?.neg()?.mul(&r(0.5))?)?;
            } else {
                // 365: yVel = 0;
                g.at("oPlayer1/step#0", 365);
                g.set(cx.this, v::yVel, r(0.0))?;
            }
            // 368: if (abs(xVel) < 0.1)
            g.at("oPlayer1/step#0", 368);
            if ({ let a = [g.get(cx.this, v::xVel)?]; b::abs(g, cx, a)? }).gml_lt(&r(0.1))? {
                // 368: xVel = 0;
                g.at("oPlayer1/step#0", 368);
                g.set(cx.this, v::xVel, r(0.0))?;
            } else {
                // 369: if (abs(xVel) != 0 && collision_point(x, y + 16, oIce, 0, 0))
                g.at("oPlayer1/step#0", 369);
                if and(({ let a = [g.get(cx.this, v::xVel)?]; b::abs(g, cx, a)? }).gml_ne(&r(0.0))?, ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(16.0))?, obj::oIce, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()) {
                    // 369: xVel *= 0.8;
                    g.at("oPlayer1/step#0", 369);
                    g.set(cx.this, v::xVel, lenient(Value::mul, g.get(cx.this, v::xVel)?, &r(0.8)))?;
                } else {
                    // 370: if (abs(xVel) != 0)
                    g.at("oPlayer1/step#0", 370);
                    if ({ let a = [g.get(cx.this, v::xVel)?]; b::abs(g, cx, a)? }).gml_ne(&r(0.0))? {
                        // 370: xVel *= 0.3;
                        g.at("oPlayer1/step#0", 370);
                        g.set(cx.this, v::xVel, lenient(Value::mul, g.get(cx.this, v::xVel)?, &r(0.30000000000000004)))?;
                    }
                }
            }
            // 372: bounced = true;
            g.at("oPlayer1/step#0", 372);
            g.set(cx.this, v::bounced, r(1.0))?;
        }
        // 376: xVelLimit = 10;
        g.at("oPlayer1/step#0", 376);
        g.set(cx.this, v::xVelLimit, r(10.0))?;
        // 377: if (xVel > xVelLimit)
        g.at("oPlayer1/step#0", 377);
        if g.get(cx.this, v::xVel)?.gml_gt(&g.get(cx.this, v::xVelLimit)?)? {
            // 377: xVel = xVelLimit;
            g.at("oPlayer1/step#0", 377);
            g.set(cx.this, v::xVel, g.get(cx.this, v::xVelLimit)?)?;
        } else {
            // 378: if (xVel < -xVelLimit)
            g.at("oPlayer1/step#0", 378);
            if g.get(cx.this, v::xVel)?.gml_lt(&g.get(cx.this, v::xVelLimit)?.neg()?)? {
                // 378: xVel = -xVelLimit;
                g.at("oPlayer1/step#0", 378);
                g.set(cx.this, v::xVel, g.get(cx.this, v::xVelLimit)?.neg()?)?;
            }
        }
        // 379: if (yVel > yVelLimit)
        g.at("oPlayer1/step#0", 379);
        if g.get(cx.this, v::yVel)?.gml_gt(&g.get(cx.this, v::yVelLimit)?)? {
            // 379: yVel = yVelLimit;
            g.at("oPlayer1/step#0", 379);
            g.set(cx.this, v::yVel, g.get(cx.this, v::yVelLimit)?)?;
        } else {
            // 380: if (yVel < -yVelLimit)
            g.at("oPlayer1/step#0", 380);
            if g.get(cx.this, v::yVel)?.gml_lt(&g.get(cx.this, v::yVelLimit)?.neg()?)? {
                // 380: yVel = -yVelLimit;
                g.at("oPlayer1/step#0", 380);
                g.set(cx.this, v::yVel, g.get(cx.this, v::yVelLimit)?.neg()?)?;
            }
        }
        // 382: moveTo(xVel, yVel);
        g.at("oPlayer1/step#0", 382);
        { let a = [g.get(cx.this, v::xVel)?, g.get(cx.this, v::yVel)?]; scr::moveTo(g, cx, &a)? };
    } else {
        // 384: if (isLevel())
        g.at("oPlayer1/step#0", 384);
        if scr::isLevel(g, cx, &[])?.is_truthy() {
            // 386: if (kDown && (platformCharacterIs(ON_GROUND) || state == HANGING) && !kRight && !kLeft)
            g.at("oPlayer1/step#0", 386);
            if and(and(and(g.get(cx.this, v::kDown)?.is_truthy(), or(({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy(), g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::HANGING)?)?)), g.get(cx.this, v::kRight)?.not()?.is_truthy()), g.get(cx.this, v::kLeft)?.not()?.is_truthy()) {
                // 388: if (viewCount <= 30)
                g.at("oPlayer1/step#0", 388);
                if g.get(cx.this, v::viewCount)?.gml_le(&r(30.0))? {
                    // 388: viewCount += 1;
                    g.at("oPlayer1/step#0", 388);
                    g.set(cx.this, v::viewCount, lenient(Value::add, g.get(cx.this, v::viewCount)?, &r(1.0)))?;
                } else {
                    // 389: view_yview[0] += 4;
                    g.at("oPlayer1/step#0", 389);
                    g.bset(v::view_yview, 0, g.bget(v::view_yview, 0)?.add(&r(4.0))?)?;
                }
            } else {
                // 391: if (kUp && (platformCharacterIs(ON_GROUND) || state == HANGING) && !kRight && !kLeft)
                g.at("oPlayer1/step#0", 391);
                if and(and(and(g.get(cx.this, v::kUp)?.is_truthy(), or(({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy(), g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::HANGING)?)?)), g.get(cx.this, v::kRight)?.not()?.is_truthy()), g.get(cx.this, v::kLeft)?.not()?.is_truthy()) {
                    // 393: if (viewCount <= 30)
                    g.at("oPlayer1/step#0", 393);
                    if g.get(cx.this, v::viewCount)?.gml_le(&r(30.0))? {
                        // 393: viewCount += 1;
                        g.at("oPlayer1/step#0", 393);
                        g.set(cx.this, v::viewCount, lenient(Value::add, g.get(cx.this, v::viewCount)?, &r(1.0)))?;
                    } else {
                        // 394: view_yview[0] -= 4;
                        g.at("oPlayer1/step#0", 394);
                        g.bset(v::view_yview, 0, g.bget(v::view_yview, 0)?.sub(&r(4.0))?)?;
                    }
                } else {
                    // 396: viewCount = 0;
                    g.at("oPlayer1/step#0", 396);
                    g.set(cx.this, v::viewCount, r(0.0))?;
                }
            }
        }
    }
    Ok(())
}

/// GML: `oPlayer1/step#2`.
pub fn step_a2(g: &mut World, cx: &mut Ctx) -> R<()> {
    let cx = &mut cx.clone();
    // 1: if (dead)
    g.at("oPlayer1/step#2", 1);
    if g.get(cx.this, v::dead)?.is_truthy() {
        // 3: kAttackPressed = checkAttackPressed();
        g.at("oPlayer1/step#2", 3);
        { let x = scr::checkAttackPressed(g, cx, &[])?; g.set(cx.this, v::kAttackPressed, x)?; }
    }
    // 6: if (global.plife > 0)
    g.at("oPlayer1/step#2", 6);
    if g.gget(gv::plife)?.gml_gt(&r(0.0))? {
        // 6: kBombPressed = checkBombPressed();
        g.at("oPlayer1/step#2", 6);
        { let x = scr::checkBombPressed(g, cx, &[])?; g.set(cx.this, v::kBombPressed, x)?; }
    } else {
        // 7: kBombPressed = false;
        g.at("oPlayer1/step#2", 7);
        g.set(cx.this, v::kBombPressed, r(0.0))?;
    }
    // 9: if (global.plife > 0)
    g.at("oPlayer1/step#2", 9);
    if g.gget(gv::plife)?.gml_gt(&r(0.0))? {
        // 9: kRopePressed = checkRopePressed();
        g.at("oPlayer1/step#2", 9);
        { let x = scr::checkRopePressed(g, cx, &[])?; g.set(cx.this, v::kRopePressed, x)?; }
    } else {
        // 10: kRopePressed = false;
        g.at("oPlayer1/step#2", 10);
        g.set(cx.this, v::kRopePressed, r(0.0))?;
    }
    // 12: kPayPressed = checkPayPressed();
    g.at("oPlayer1/step#2", 12);
    { let x = scr::checkPayPressed(g, cx, &[])?; g.set(cx.this, v::kPayPressed, x)?; }
    // 15: if (sprite_index == sAttackLeft || sprite_index == sDamselAttackL || sprite_index == sTunnelAttackL 
    g.at("oPlayer1/step#2", 15);
    if and(and(and(or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sAttackLeft)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselAttackL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelAttackL)?), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)?), g.get(cx.this, v::image_index)?.gml_gt(&r(4.0))?), b::instance_number(g, cx, [obj::oWhip])?.gml_eq(&r(0.0))?) {
        // 17: if (holdItem)
        g.at("oPlayer1/step#2", 17);
        if g.get(cx.this, v::holdItem)?.is_truthy() {
            // 19: if (holdItem.type == "Machete")
            g.at("oPlayer1/step#2", 19);
            if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Machete"))? {
                // 21: obj = instance_create(x - 16, y, oSlash);
                g.at("oPlayer1/step#2", 21);
                {
                    let x = { let a = [g.get(cx.this, v::x)?.sub(&r(16.0))?, g.get(cx.this, v::y)?, obj::oSlash]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 22: obj.sprite_index = sSlashLeft;
                g.at("oPlayer1/step#2", 22);
                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sSlashLeft)?; }
                // 23: playSound(global.sndWhip);
                g.at("oPlayer1/step#2", 23);
                { let a = [g.gget(gv::sndWhip)?]; scr::playSound(g, cx, &a)? };
            } else {
                // 25: if (holdItem.type == "Mattock")
                g.at("oPlayer1/step#2", 25);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Mattock"))? {
                    // 27: obj = instance_create(x - 16, y, oMattockHit);
                    g.at("oPlayer1/step#2", 27);
                    {
                        let x = { let a = [g.get(cx.this, v::x)?.sub(&r(16.0))?, g.get(cx.this, v::y)?, obj::oMattockHit]; b::instance_create(g, cx, a)? };
                        g.set(cx.this, v::obj, x)?;
                    }
                    // 28: obj.sprite_index = sMattockHitL;
                    g.at("oPlayer1/step#2", 28);
                    { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sMattockHitL)?; }
                    // 29: playSound(global.sndWhip);
                    g.at("oPlayer1/step#2", 29);
                    { let a = [g.gget(gv::sndWhip)?]; scr::playSound(g, cx, &a)? };
                }
            }
        } else {
            // 34: if (global.isTunnelMan)
            g.at("oPlayer1/step#2", 34);
            if g.gget(gv::isTunnelMan)?.is_truthy() {
                // 36: obj = instance_create(x - 16, y, oMattockHit);
                g.at("oPlayer1/step#2", 36);
                {
                    let x = { let a = [g.get(cx.this, v::x)?.sub(&r(16.0))?, g.get(cx.this, v::y)?, obj::oMattockHit]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 37: obj.sprite_index = sMattockHitL;
                g.at("oPlayer1/step#2", 37);
                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sMattockHitL)?; }
                // 38: playSound(global.sndWhip);
                g.at("oPlayer1/step#2", 38);
                { let a = [g.gget(gv::sndWhip)?]; scr::playSound(g, cx, &a)? };
            } else {
                // 42: obj = instance_create(x - 16, y, oWhip);
                g.at("oPlayer1/step#2", 42);
                {
                    let x = { let a = [g.get(cx.this, v::x)?.sub(&r(16.0))?, g.get(cx.this, v::y)?, obj::oWhip]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 43: obj.sprite_index = sWhipLeft;
                g.at("oPlayer1/step#2", 43);
                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sWhipLeft)?; }
                // 44: playSound(global.sndWhip);
                g.at("oPlayer1/step#2", 44);
                { let a = [g.gget(gv::sndWhip)?]; scr::playSound(g, cx, &a)? };
            }
        }
    } else {
        // 48: if (sprite_index == sAttackLeft || sprite_index == sDamselAttackL || sprite_index == sTunnelAttackL 
        g.at("oPlayer1/step#2", 48);
        if and(and(and(or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sAttackLeft)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselAttackL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelAttackL)?), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)?), g.get(cx.this, v::image_index)?.gml_gt(&r(4.0))?), b::instance_number(g, cx, [obj::oWhip])?.gml_eq(&r(0.0))?) {
            // 50: if (holdItem)
            g.at("oPlayer1/step#2", 50);
            if g.get(cx.this, v::holdItem)?.is_truthy() {
                // 52: if (holdItem.type == "Machete")
                g.at("oPlayer1/step#2", 52);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Machete"))? {
                    // 54: obj = instance_create(x + 16, y, oSlash);
                    g.at("oPlayer1/step#2", 54);
                    {
                        let x = { let a = [g.get(cx.this, v::x)?.add(&r(16.0))?, g.get(cx.this, v::y)?, obj::oSlash]; b::instance_create(g, cx, a)? };
                        g.set(cx.this, v::obj, x)?;
                    }
                    // 55: obj.sprite_index = sSlashRight;
                    g.at("oPlayer1/step#2", 55);
                    { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sSlashRight)?; }
                    // 56: playSound(global.sndWhip);
                    g.at("oPlayer1/step#2", 56);
                    { let a = [g.gget(gv::sndWhip)?]; scr::playSound(g, cx, &a)? };
                } else {
                    // 58: if (holdItem.type == "Mattock")
                    g.at("oPlayer1/step#2", 58);
                    if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Mattock"))? {
                        // 60: obj = instance_create(x + 16, y, oMattockHit);
                        g.at("oPlayer1/step#2", 60);
                        {
                            let x = { let a = [g.get(cx.this, v::x)?.add(&r(16.0))?, g.get(cx.this, v::y)?, obj::oMattockHit]; b::instance_create(g, cx, a)? };
                            g.set(cx.this, v::obj, x)?;
                        }
                        // 61: obj.sprite_index = sMattockHitR;
                        g.at("oPlayer1/step#2", 61);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sMattockHitR)?; }
                        // 62: playSound(global.sndWhip);
                        g.at("oPlayer1/step#2", 62);
                        { let a = [g.gget(gv::sndWhip)?]; scr::playSound(g, cx, &a)? };
                    }
                }
            } else {
                // 67: if (global.isTunnelMan)
                g.at("oPlayer1/step#2", 67);
                if g.gget(gv::isTunnelMan)?.is_truthy() {
                    // 69: obj = instance_create(x + 16, y, oMattockHit);
                    g.at("oPlayer1/step#2", 69);
                    {
                        let x = { let a = [g.get(cx.this, v::x)?.add(&r(16.0))?, g.get(cx.this, v::y)?, obj::oMattockHit]; b::instance_create(g, cx, a)? };
                        g.set(cx.this, v::obj, x)?;
                    }
                    // 70: obj.sprite_index = sMattockHitR;
                    g.at("oPlayer1/step#2", 70);
                    { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sMattockHitR)?; }
                    // 71: playSound(global.sndWhip);
                    g.at("oPlayer1/step#2", 71);
                    { let a = [g.gget(gv::sndWhip)?]; scr::playSound(g, cx, &a)? };
                } else {
                    // 75: obj = instance_create(x + 16, y, oWhip);
                    g.at("oPlayer1/step#2", 75);
                    {
                        let x = { let a = [g.get(cx.this, v::x)?.add(&r(16.0))?, g.get(cx.this, v::y)?, obj::oWhip]; b::instance_create(g, cx, a)? };
                        g.set(cx.this, v::obj, x)?;
                    }
                    // 76: obj.sprite_index = sWhipRight;
                    g.at("oPlayer1/step#2", 76);
                    { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sWhipRight)?; }
                    // 77: playSound(global.sndWhip);
                    g.at("oPlayer1/step#2", 77);
                    { let a = [g.gget(gv::sndWhip)?]; scr::playSound(g, cx, &a)? };
                }
            }
        }
    }
    // 82: if (holdItem)
    g.at("oPlayer1/step#2", 82);
    if g.get(cx.this, v::holdItem)?.is_truthy() {
        // 84: if (holdItem.type == "Machete")
        g.at("oPlayer1/step#2", 84);
        if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Machete"))? {
            // 86: if (sprite_index == sAttackLeft || sprite_index == sDamselAttackL || sprite_index == sTunnelAttackL 
            g.at("oPlayer1/step#2", 86);
            if and(and(and(or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sAttackLeft)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselAttackL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelAttackL)?), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)?), g.get(cx.this, v::image_index)?.gml_lt(&r(2.0))?), b::instance_number(g, cx, [obj::oMachetePre])?.gml_eq(&r(0.0))?) {
                // 88: obj = instance_create(x + 16, y, oMachetePre);
                g.at("oPlayer1/step#2", 88);
                {
                    let x = { let a = [g.get(cx.this, v::x)?.add(&r(16.0))?, g.get(cx.this, v::y)?, obj::oMachetePre]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 89: obj.sprite_index = sMachetePreL;
                g.at("oPlayer1/step#2", 89);
                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sMachetePreL)?; }
            } else {
                // 91: if (sprite_index == sAttackLeft || sprite_index == sDamselAttackL || sprite_index == sTunnelAttackL 
                g.at("oPlayer1/step#2", 91);
                if and(and(and(or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sAttackLeft)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselAttackL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelAttackL)?), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)?), g.get(cx.this, v::image_index)?.gml_lt(&r(2.0))?), b::instance_number(g, cx, [obj::oMachetePre])?.gml_eq(&r(0.0))?) {
                    // 93: obj = instance_create(x - 16, y, oMachetePre);
                    g.at("oPlayer1/step#2", 93);
                    {
                        let x = { let a = [g.get(cx.this, v::x)?.sub(&r(16.0))?, g.get(cx.this, v::y)?, obj::oMachetePre]; b::instance_create(g, cx, a)? };
                        g.set(cx.this, v::obj, x)?;
                    }
                    // 94: obj.sprite_index = sMachetePreR;
                    g.at("oPlayer1/step#2", 94);
                    { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sMachetePreR)?; }
                }
            }
        } else {
            // 97: if (holdItem.type == "Mattock")
            g.at("oPlayer1/step#2", 97);
            if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Mattock"))? {
                // 99: if (sprite_index == sAttackLeft || sprite_index == sDamselAttackL || sprite_index == sTunnelAttackL 
                g.at("oPlayer1/step#2", 99);
                if and(and(and(or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sAttackLeft)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselAttackL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelAttackL)?), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)?), g.get(cx.this, v::image_index)?.gml_lt(&r(2.0))?), b::instance_number(g, cx, [obj::oMachetePre])?.gml_eq(&r(0.0))?) {
                    // 101: obj = instance_create(x + 16, y, oMattockPre);
                    g.at("oPlayer1/step#2", 101);
                    {
                        let x = { let a = [g.get(cx.this, v::x)?.add(&r(16.0))?, g.get(cx.this, v::y)?, obj::oMattockPre]; b::instance_create(g, cx, a)? };
                        g.set(cx.this, v::obj, x)?;
                    }
                    // 102: obj.sprite_index = sMattockPreL;
                    g.at("oPlayer1/step#2", 102);
                    { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sMattockPreL)?; }
                } else {
                    // 104: if (sprite_index == sAttackLeft || sprite_index == sDamselAttackL || sprite_index == sTunnelAttackL 
                    g.at("oPlayer1/step#2", 104);
                    if and(and(and(or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sAttackLeft)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselAttackL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelAttackL)?), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)?), g.get(cx.this, v::image_index)?.gml_lt(&r(2.0))?), b::instance_number(g, cx, [obj::oMachetePre])?.gml_eq(&r(0.0))?) {
                        // 106: obj = instance_create(x - 16, y, oMattockPre);
                        g.at("oPlayer1/step#2", 106);
                        {
                            let x = { let a = [g.get(cx.this, v::x)?.sub(&r(16.0))?, g.get(cx.this, v::y)?, obj::oMattockPre]; b::instance_create(g, cx, a)? };
                            g.set(cx.this, v::obj, x)?;
                        }
                        // 107: obj.sprite_index = sMattockPreR;
                        g.at("oPlayer1/step#2", 107);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sMattockPreR)?; }
                    }
                }
            }
        }
    } else {
        // 111: if (sprite_index == sTunnelAttackL && image_index < 2 && instance_number(oMattockPre) == 0)
        g.at("oPlayer1/step#2", 111);
        if and(and(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelAttackL)?, g.get(cx.this, v::image_index)?.gml_lt(&r(2.0))?), b::instance_number(g, cx, [obj::oMattockPre])?.gml_eq(&r(0.0))?) {
            // 113: if (facing == LEFT)
            g.at("oPlayer1/step#2", 113);
            if g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)? {
                // 115: obj = instance_create(x + 16, y, oMattockPre);
                g.at("oPlayer1/step#2", 115);
                {
                    let x = { let a = [g.get(cx.this, v::x)?.add(&r(16.0))?, g.get(cx.this, v::y)?, obj::oMattockPre]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 116: obj.sprite_index = sMattockPreL;
                g.at("oPlayer1/step#2", 116);
                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sMattockPreL)?; }
            } else {
                // 120: obj = instance_create(x - 16, y, oMattockPre);
                g.at("oPlayer1/step#2", 120);
                {
                    let x = { let a = [g.get(cx.this, v::x)?.sub(&r(16.0))?, g.get(cx.this, v::y)?, obj::oMattockPre]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 121: obj.sprite_index = sMattockPreR;
                g.at("oPlayer1/step#2", 121);
                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sMattockPreR)?; }
            }
        } else {
            // 124: if (sprite_index == sAttackLeft || sprite_index == sDamselAttackL || sprite_index == sTunnelAttackL 
            g.at("oPlayer1/step#2", 124);
            if and(and(and(or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sAttackLeft)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselAttackL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelAttackL)?), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)?), g.get(cx.this, v::image_index)?.gml_lt(&r(2.0))?), b::instance_number(g, cx, [obj::oWhipPre])?.gml_eq(&r(0.0))?) {
                // 126: obj = instance_create(x + 16, y, oWhipPre);
                g.at("oPlayer1/step#2", 126);
                {
                    let x = { let a = [g.get(cx.this, v::x)?.add(&r(16.0))?, g.get(cx.this, v::y)?, obj::oWhipPre]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 127: obj.sprite_index = sWhipPreL;
                g.at("oPlayer1/step#2", 127);
                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sWhipPreL)?; }
            } else {
                // 129: if (sprite_index == sAttackLeft || sprite_index == sDamselAttackL || sprite_index == sTunnelAttackL 
                g.at("oPlayer1/step#2", 129);
                if and(and(and(or(or(g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sAttackLeft)?, g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sDamselAttackL)?), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelAttackL)?), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)?), g.get(cx.this, v::image_index)?.gml_lt(&r(2.0))?), b::instance_number(g, cx, [obj::oWhipPre])?.gml_eq(&r(0.0))?) {
                    // 131: obj = instance_create(x - 16, y, oWhipPre);
                    g.at("oPlayer1/step#2", 131);
                    {
                        let x = { let a = [g.get(cx.this, v::x)?.sub(&r(16.0))?, g.get(cx.this, v::y)?, obj::oWhipPre]; b::instance_create(g, cx, a)? };
                        g.set(cx.this, v::obj, x)?;
                    }
                    // 132: obj.sprite_index = sWhipPreR;
                    g.at("oPlayer1/step#2", 132);
                    { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sWhipPreR)?; }
                }
            }
        }
    }
    // 135: if (!whipping)
    g.at("oPlayer1/step#2", 135);
    if g.get(cx.this, v::whipping)?.not()?.is_truthy() {
        // 137: with (oWhip)
        g.at("oPlayer1/step#2", 137);
        {
            let t = obj::oWhip;
            let mut it1 = g.with_iter(cx, &t)?;
            let outer3 = (cx.this, cx.other);
            'with1: while let Some(h2) = it1.next(g) {
                (cx.this, cx.other) = (h2, outer3.0);
                // 137: instance_destroy();
                g.at("oPlayer1/step#2", 137);
                b::instance_destroy(g, cx, [])?;
            }
            (cx.this, cx.other) = outer3;
        }
        // 138: with (oWhipPre)
        g.at("oPlayer1/step#2", 138);
        {
            let t = obj::oWhipPre;
            let mut it4 = g.with_iter(cx, &t)?;
            let outer6 = (cx.this, cx.other);
            'with2: while let Some(h5) = it4.next(g) {
                (cx.this, cx.other) = (h5, outer6.0);
                // 138: instance_destroy();
                g.at("oPlayer1/step#2", 138);
                b::instance_destroy(g, cx, [])?;
            }
            (cx.this, cx.other) = outer6;
        }
    }
    // 141: if (holdItem > 0)
    g.at("oPlayer1/step#2", 141);
    if g.get(cx.this, v::holdItem)?.gml_gt(&r(0.0))? {
        // 143: if (holdItem.cost > 0 && isLevel())
        g.at("oPlayer1/step#2", 143);
        if and(g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::cost)?.gml_gt(&r(0.0))?, scr::isLevel(g, cx, &[])?.is_truthy()) {
            // 145: if (global.roomPath[scrGetRoomX(x), scrGetRoomY(y)] != 4 && global.roomPath[scrGetRoomX(x), scrGetRo
            g.at("oPlayer1/step#2", 145);
            if and(({ let i = ix(&{ let a = [g.get(cx.this, v::x)?]; scr::scrGetRoomX(g, cx, &a)? })?; let j = ix(&{ let a = [g.get(cx.this, v::y)?]; scr::scrGetRoomY(g, cx, &a)? })?; g.gget_a(gv::roomPath, i, j)? }).gml_ne(&r(4.0))?, ({ let i = ix(&{ let a = [g.get(cx.this, v::x)?]; scr::scrGetRoomX(g, cx, &a)? })?; let j = ix(&{ let a = [g.get(cx.this, v::y)?]; scr::scrGetRoomY(g, cx, &a)? })?; g.gget_a(gv::roomPath, i, j)? }).gml_ne(&r(5.0))?) {
                // 147: scrStealItem();
                g.at("oPlayer1/step#2", 147);
                scr::scrStealItem(g, cx, &[])?;
                // 148: if (instance_exists(oShopkeeper))
                g.at("oPlayer1/step#2", 148);
                if b::instance_exists(g, cx, [obj::oShopkeeper])?.is_truthy() {
                    // 150: scrShopkeeperAnger(0);
                    g.at("oPlayer1/step#2", 150);
                    scr::scrShopkeeperAnger(g, cx, &[r(0.0)])?;
                }
            }
        } else {
            // 154: if (holdItem.cost > 0)
            g.at("oPlayer1/step#2", 154);
            if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::cost)?.gml_gt(&r(0.0))? {
                // 156: scrStealItem();
                g.at("oPlayer1/step#2", 156);
                scr::scrStealItem(g, cx, &[])?;
            }
        }
    }
    // 161: if (kUp && kAttackPressed && collision_point(x, y, oChest, 0, 0))
    g.at("oPlayer1/step#2", 161);
    if and(and(g.get(cx.this, v::kUp)?.is_truthy(), g.get(cx.this, v::kAttackPressed)?.is_truthy()), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oChest, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()) {
        // 163: if (isRealLevel())
        g.at("oPlayer1/step#2", 163);
        if scr::isRealLevel(g, cx, &[])?.is_truthy() {
            // 163: global.totalChests += 1;
            g.at("oPlayer1/step#2", 163);
            g.gset(gv::totalChests, lenient(Value::add, g.gget(gv::totalChests)?, &r(1.0)))?;
        }
        // 164: chest = instance_place(x, y, oChest);
        g.at("oPlayer1/step#2", 164);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oChest]; b::instance_place(g, cx, a)? };
            g.set(cx.this, v::chest, x)?;
        }
        // 165: if (chest.sprite_index == sChest)
        g.at("oPlayer1/step#2", 165);
        if g.get_on(cx, &g.get(cx.this, v::chest)?, v::sprite_index)?.gml_eq(&spr::sChest)? {
            // 167: chest.sprite_index = sChestOpen;
            g.at("oPlayer1/step#2", 167);
            { let t = g.get(cx.this, v::chest)?; g.set_on(cx, &t, v::sprite_index, spr::sChestOpen)?; }
            // 168: if (rand(1, 12) == 1 && global.currLevel > 0)
            g.at("oPlayer1/step#2", 168);
            if and(scr::rand(g, cx, &[r(1.0), r(12.0)])?.gml_eq(&r(1.0))?, g.gget(gv::currLevel)?.gml_gt(&r(0.0))?) {
                // 170: obj = instance_create(chest.x, chest.y, oBomb);
                g.at("oPlayer1/step#2", 170);
                {
                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oBomb]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 171: obj.xVel = rand(0, 3) - rand(0, 3);
                g.at("oPlayer1/step#2", 171);
                {
                    let t = g.get(cx.this, v::obj)?;
                    let x = scr::rand(g, cx, &[r(0.0), r(3.0)])?.sub(&scr::rand(g, cx, &[r(0.0), r(3.0)])?)?;
                    g.set_on(cx, &t, v::xVel, x)?;
                }
                // 172: obj.yVel = -2;
                g.at("oPlayer1/step#2", 172);
                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::yVel, r(-2.0))?; }
                // 173: with (obj)
                g.at("oPlayer1/step#2", 173);
                {
                    let t = g.get(cx.this, v::obj)?;
                    let mut it7 = g.with_iter(cx, &t)?;
                    let outer9 = (cx.this, cx.other);
                    'with3: while let Some(h8) = it7.next(g) {
                        (cx.this, cx.other) = (h8, outer9.0);
                        // 175: sprite_index = sBombArmed;
                        g.at("oPlayer1/step#2", 175);
                        g.set(cx.this, v::sprite_index, spr::sBombArmed)?;
                        // 176: alarm[1] = 40;
                        g.at("oPlayer1/step#2", 176);
                        g.set_a(cx.this, v::alarm, 0, 1, r(40.0))?;
                    }
                    (cx.this, cx.other) = outer9;
                }
                // 178: playSound(global.sndTrap);
                g.at("oPlayer1/step#2", 178);
                { let a = [g.gget(gv::sndTrap)?]; scr::playSound(g, cx, &a)? };
            } else {
                // 181: playSound(global.sndChestOpen);
                g.at("oPlayer1/step#2", 181);
                { let a = [g.gget(gv::sndChestOpen)?]; scr::playSound(g, cx, &a)? };
                // 182: repeat (rand(3, 4))
                g.at("oPlayer1/step#2", 182);
                'repeat4: for _ in 0..repeat_count(&scr::rand(g, cx, &[r(3.0), r(4.0)])?) {
                    // 184: n = rand(1, 3);
                    g.at("oPlayer1/step#2", 184);
                    { let x = scr::rand(g, cx, &[r(1.0), r(3.0)])?; g.set(cx.this, v::n, x)?; }
                    // 185: switch (n)
                    g.at("oPlayer1/step#2", 185);
                    'switch5: {
                        let sw = g.get(cx.this, v::n)?;
                        let entry: usize = if case_eq(&sw, &r(1.0)) { 0 } else if case_eq(&sw, &r(2.0)) { 1 } else if case_eq(&sw, &r(3.0)) { 2 } else { 3 };
                        if entry <= 0 {
                            {
                                // 187: obj = instance_create(chest.x, chest.y, oEmerald);
                                g.at("oPlayer1/step#2", 187);
                                {
                                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oEmerald]; b::instance_create(g, cx, a)? };
                                    g.set(cx.this, v::obj, x)?;
                                }
                                // 187: break;
                                g.at("oPlayer1/step#2", 187);
                                break 'switch5;
                            }
                        }
                        if entry <= 1 {
                            {
                                // 188: obj = instance_create(chest.x, chest.y, oSapphire);
                                g.at("oPlayer1/step#2", 188);
                                {
                                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oSapphire]; b::instance_create(g, cx, a)? };
                                    g.set(cx.this, v::obj, x)?;
                                }
                                // 188: break;
                                g.at("oPlayer1/step#2", 188);
                                break 'switch5;
                            }
                        }
                        if entry <= 2 {
                            {
                                // 189: obj = instance_create(chest.x, chest.y, oRuby);
                                g.at("oPlayer1/step#2", 189);
                                {
                                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oRuby]; b::instance_create(g, cx, a)? };
                                    g.set(cx.this, v::obj, x)?;
                                }
                                // 189: break;
                                g.at("oPlayer1/step#2", 189);
                                break 'switch5;
                            }
                        }
                    }
                    // 191: obj.xVel = rand(0, 3) - rand(0, 3);
                    g.at("oPlayer1/step#2", 191);
                    {
                        let t = g.get(cx.this, v::obj)?;
                        let x = scr::rand(g, cx, &[r(0.0), r(3.0)])?.sub(&scr::rand(g, cx, &[r(0.0), r(3.0)])?)?;
                        g.set_on(cx, &t, v::xVel, x)?;
                    }
                    // 192: obj.yVel = -2;
                    g.at("oPlayer1/step#2", 192);
                    { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::yVel, r(-2.0))?; }
                }
                // 194: if (rand(1, 4) == 1)
                g.at("oPlayer1/step#2", 194);
                if scr::rand(g, cx, &[r(1.0), r(4.0)])?.gml_eq(&r(1.0))? {
                    // 196: n = rand(1, 3);
                    g.at("oPlayer1/step#2", 196);
                    { let x = scr::rand(g, cx, &[r(1.0), r(3.0)])?; g.set(cx.this, v::n, x)?; }
                    // 197: switch (n)
                    g.at("oPlayer1/step#2", 197);
                    'switch6: {
                        let sw = g.get(cx.this, v::n)?;
                        let entry: usize = if case_eq(&sw, &r(1.0)) { 0 } else if case_eq(&sw, &r(2.0)) { 1 } else if case_eq(&sw, &r(3.0)) { 2 } else { 3 };
                        if entry <= 0 {
                            {
                                // 199: obj = instance_create(chest.x, chest.y, oEmeraldBig);
                                g.at("oPlayer1/step#2", 199);
                                {
                                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oEmeraldBig]; b::instance_create(g, cx, a)? };
                                    g.set(cx.this, v::obj, x)?;
                                }
                                // 199: break;
                                g.at("oPlayer1/step#2", 199);
                                break 'switch6;
                            }
                        }
                        if entry <= 1 {
                            {
                                // 200: obj = instance_create(chest.x, chest.y, oSapphireBig);
                                g.at("oPlayer1/step#2", 200);
                                {
                                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oSapphireBig]; b::instance_create(g, cx, a)? };
                                    g.set(cx.this, v::obj, x)?;
                                }
                                // 200: break;
                                g.at("oPlayer1/step#2", 200);
                                break 'switch6;
                            }
                        }
                        if entry <= 2 {
                            {
                                // 201: obj = instance_create(chest.x, chest.y, oRubyBig);
                                g.at("oPlayer1/step#2", 201);
                                {
                                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oRubyBig]; b::instance_create(g, cx, a)? };
                                    g.set(cx.this, v::obj, x)?;
                                }
                                // 201: break;
                                g.at("oPlayer1/step#2", 201);
                                break 'switch6;
                            }
                        }
                    }
                    // 203: obj.xVel = rand(0, 3) - rand(0, 3);
                    g.at("oPlayer1/step#2", 203);
                    {
                        let t = g.get(cx.this, v::obj)?;
                        let x = scr::rand(g, cx, &[r(0.0), r(3.0)])?.sub(&scr::rand(g, cx, &[r(0.0), r(3.0)])?)?;
                        g.set_on(cx, &t, v::xVel, x)?;
                    }
                    // 204: obj.yVel = -2;
                    g.at("oPlayer1/step#2", 204);
                    { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::yVel, r(-2.0))?; }
                }
            }
            // 207: kAttackPressed = false;
            g.at("oPlayer1/step#2", 207);
            g.set(cx.this, v::kAttackPressed, r(0.0))?;
        }
    }
    // 212: if (kUp && kAttackPressed && collision_point(x, y, oCrate, 0, 0))
    g.at("oPlayer1/step#2", 212);
    if and(and(g.get(cx.this, v::kUp)?.is_truthy(), g.get(cx.this, v::kAttackPressed)?.is_truthy()), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oCrate, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()) {
        // 214: if (isRealLevel())
        g.at("oPlayer1/step#2", 214);
        if scr::isRealLevel(g, cx, &[])?.is_truthy() {
            // 214: global.totalCrates += 1;
            g.at("oPlayer1/step#2", 214);
            g.gset(gv::totalCrates, lenient(Value::add, g.gget(gv::totalCrates)?, &r(1.0)))?;
        }
        // 215: chest = instance_place(x, y, oCrate);
        g.at("oPlayer1/step#2", 215);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oCrate]; b::instance_place(g, cx, a)? };
            g.set(cx.this, v::chest, x)?;
        }
        // 216: if (isRoom("rTutorial"))
        g.at("oPlayer1/step#2", 216);
        if scr::isRoom(g, cx, &[s!("rTutorial")])?.is_truthy() {
            // 216: obj = instance_create(chest.x, chest.y, oBombBag);
            g.at("oPlayer1/step#2", 216);
            {
                let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oBombBag]; b::instance_create(g, cx, a)? };
                g.set(cx.this, v::obj, x)?;
            }
        } else {
            // 217: if (rand(1, 500) == 1)
            g.at("oPlayer1/step#2", 217);
            if scr::rand(g, cx, &[r(1.0), r(500.0)])?.gml_eq(&r(1.0))? {
                // 217: obj = instance_create(chest.x, chest.y, oJetpack);
                g.at("oPlayer1/step#2", 217);
                {
                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oJetpack]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
            } else {
                // 218: if (rand(1, 200) == 1)
                g.at("oPlayer1/step#2", 218);
                if scr::rand(g, cx, &[r(1.0), r(200.0)])?.gml_eq(&r(1.0))? {
                    // 218: obj = instance_create(chest.x, chest.y, oCapePickup);
                    g.at("oPlayer1/step#2", 218);
                    {
                        let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oCapePickup]; b::instance_create(g, cx, a)? };
                        g.set(cx.this, v::obj, x)?;
                    }
                } else {
                    // 219: if (rand(1, 100) == 1)
                    g.at("oPlayer1/step#2", 219);
                    if scr::rand(g, cx, &[r(1.0), r(100.0)])?.gml_eq(&r(1.0))? {
                        // 219: obj = instance_create(chest.x, chest.y, oShotgun);
                        g.at("oPlayer1/step#2", 219);
                        {
                            let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oShotgun]; b::instance_create(g, cx, a)? };
                            g.set(cx.this, v::obj, x)?;
                        }
                    } else {
                        // 220: if (rand(1, 100) == 1)
                        g.at("oPlayer1/step#2", 220);
                        if scr::rand(g, cx, &[r(1.0), r(100.0)])?.gml_eq(&r(1.0))? {
                            // 220: obj = instance_create(chest.x, chest.y, oMattock);
                            g.at("oPlayer1/step#2", 220);
                            {
                                let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oMattock]; b::instance_create(g, cx, a)? };
                                g.set(cx.this, v::obj, x)?;
                            }
                        } else {
                            // 221: if (rand(1, 100) == 1)
                            g.at("oPlayer1/step#2", 221);
                            if scr::rand(g, cx, &[r(1.0), r(100.0)])?.gml_eq(&r(1.0))? {
                                // 221: obj = instance_create(chest.x, chest.y, oTeleporter);
                                g.at("oPlayer1/step#2", 221);
                                {
                                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oTeleporter]; b::instance_create(g, cx, a)? };
                                    g.set(cx.this, v::obj, x)?;
                                }
                            } else {
                                // 222: if (rand(1, 90) == 1)
                                g.at("oPlayer1/step#2", 222);
                                if scr::rand(g, cx, &[r(1.0), r(90.0)])?.gml_eq(&r(1.0))? {
                                    // 222: obj = instance_create(chest.x, chest.y, oGloves);
                                    g.at("oPlayer1/step#2", 222);
                                    {
                                        let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oGloves]; b::instance_create(g, cx, a)? };
                                        g.set(cx.this, v::obj, x)?;
                                    }
                                } else {
                                    // 223: if (rand(1, 90) == 1)
                                    g.at("oPlayer1/step#2", 223);
                                    if scr::rand(g, cx, &[r(1.0), r(90.0)])?.gml_eq(&r(1.0))? {
                                        // 223: obj = instance_create(chest.x, chest.y, oSpectacles);
                                        g.at("oPlayer1/step#2", 223);
                                        {
                                            let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oSpectacles]; b::instance_create(g, cx, a)? };
                                            g.set(cx.this, v::obj, x)?;
                                        }
                                    } else {
                                        // 224: if (rand(1, 80) == 1)
                                        g.at("oPlayer1/step#2", 224);
                                        if scr::rand(g, cx, &[r(1.0), r(80.0)])?.gml_eq(&r(1.0))? {
                                            // 224: obj = instance_create(chest.x, chest.y, oWebCannon);
                                            g.at("oPlayer1/step#2", 224);
                                            {
                                                let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oWebCannon]; b::instance_create(g, cx, a)? };
                                                g.set(cx.this, v::obj, x)?;
                                            }
                                        } else {
                                            // 225: if (rand(1, 80) == 1)
                                            g.at("oPlayer1/step#2", 225);
                                            if scr::rand(g, cx, &[r(1.0), r(80.0)])?.gml_eq(&r(1.0))? {
                                                // 225: obj = instance_create(chest.x, chest.y, oPistol);
                                                g.at("oPlayer1/step#2", 225);
                                                {
                                                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oPistol]; b::instance_create(g, cx, a)? };
                                                    g.set(cx.this, v::obj, x)?;
                                                }
                                            } else {
                                                // 226: if (rand(1, 80) == 1)
                                                g.at("oPlayer1/step#2", 226);
                                                if scr::rand(g, cx, &[r(1.0), r(80.0)])?.gml_eq(&r(1.0))? {
                                                    // 226: obj = instance_create(chest.x, chest.y, oMitt);
                                                    g.at("oPlayer1/step#2", 226);
                                                    {
                                                        let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oMitt]; b::instance_create(g, cx, a)? };
                                                        g.set(cx.this, v::obj, x)?;
                                                    }
                                                } else {
                                                    // 227: if (rand(1, 60) == 1)
                                                    g.at("oPlayer1/step#2", 227);
                                                    if scr::rand(g, cx, &[r(1.0), r(60.0)])?.gml_eq(&r(1.0))? {
                                                        // 227: obj = instance_create(chest.x, chest.y, oPaste);
                                                        g.at("oPlayer1/step#2", 227);
                                                        {
                                                            let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oPaste]; b::instance_create(g, cx, a)? };
                                                            g.set(cx.this, v::obj, x)?;
                                                        }
                                                    } else {
                                                        // 228: if (rand(1, 60) == 1)
                                                        g.at("oPlayer1/step#2", 228);
                                                        if scr::rand(g, cx, &[r(1.0), r(60.0)])?.gml_eq(&r(1.0))? {
                                                            // 228: obj = instance_create(chest.x, chest.y, oSpringShoes);
                                                            g.at("oPlayer1/step#2", 228);
                                                            {
                                                                let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oSpringShoes]; b::instance_create(g, cx, a)? };
                                                                g.set(cx.this, v::obj, x)?;
                                                            }
                                                        } else {
                                                            // 229: if (rand(1, 60) == 1)
                                                            g.at("oPlayer1/step#2", 229);
                                                            if scr::rand(g, cx, &[r(1.0), r(60.0)])?.gml_eq(&r(1.0))? {
                                                                // 229: obj = instance_create(chest.x, chest.y, oSpikeShoes);
                                                                g.at("oPlayer1/step#2", 229);
                                                                {
                                                                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oSpikeShoes]; b::instance_create(g, cx, a)? };
                                                                    g.set(cx.this, v::obj, x)?;
                                                                }
                                                            } else {
                                                                // 230: if (rand(1, 60) == 1)
                                                                g.at("oPlayer1/step#2", 230);
                                                                if scr::rand(g, cx, &[r(1.0), r(60.0)])?.gml_eq(&r(1.0))? {
                                                                    // 230: obj = instance_create(chest.x, chest.y, oMachete);
                                                                    g.at("oPlayer1/step#2", 230);
                                                                    {
                                                                        let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oMachete]; b::instance_create(g, cx, a)? };
                                                                        g.set(cx.this, v::obj, x)?;
                                                                    }
                                                                } else {
                                                                    // 231: if (rand(1, 40) == 1)
                                                                    g.at("oPlayer1/step#2", 231);
                                                                    if scr::rand(g, cx, &[r(1.0), r(40.0)])?.gml_eq(&r(1.0))? {
                                                                        // 231: obj = instance_create(chest.x, chest.y, oBombBox);
                                                                        g.at("oPlayer1/step#2", 231);
                                                                        {
                                                                            let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oBombBox]; b::instance_create(g, cx, a)? };
                                                                            g.set(cx.this, v::obj, x)?;
                                                                        }
                                                                    } else {
                                                                        // 232: if (rand(1, 40) == 1)
                                                                        g.at("oPlayer1/step#2", 232);
                                                                        if scr::rand(g, cx, &[r(1.0), r(40.0)])?.gml_eq(&r(1.0))? {
                                                                            // 232: obj = instance_create(chest.x, chest.y, oBow);
                                                                            g.at("oPlayer1/step#2", 232);
                                                                            {
                                                                                let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oBow]; b::instance_create(g, cx, a)? };
                                                                                g.set(cx.this, v::obj, x)?;
                                                                            }
                                                                        } else {
                                                                            // 233: if (rand(1, 20) == 1)
                                                                            g.at("oPlayer1/step#2", 233);
                                                                            if scr::rand(g, cx, &[r(1.0), r(20.0)])?.gml_eq(&r(1.0))? {
                                                                                // 233: obj = instance_create(chest.x, chest.y, oCompass);
                                                                                g.at("oPlayer1/step#2", 233);
                                                                                {
                                                                                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oCompass]; b::instance_create(g, cx, a)? };
                                                                                    g.set(cx.this, v::obj, x)?;
                                                                                }
                                                                            } else {
                                                                                // 234: if (rand(1, 10) == 1)
                                                                                g.at("oPlayer1/step#2", 234);
                                                                                if scr::rand(g, cx, &[r(1.0), r(10.0)])?.gml_eq(&r(1.0))? {
                                                                                    // 234: obj = instance_create(chest.x, chest.y, oParaPickup);
                                                                                    g.at("oPlayer1/step#2", 234);
                                                                                    {
                                                                                        let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oParaPickup]; b::instance_create(g, cx, a)? };
                                                                                        g.set(cx.this, v::obj, x)?;
                                                                                    }
                                                                                } else {
                                                                                    // 235: if (rand(1, 2) == 1)
                                                                                    g.at("oPlayer1/step#2", 235);
                                                                                    if scr::rand(g, cx, &[r(1.0), r(2.0)])?.gml_eq(&r(1.0))? {
                                                                                        // 235: obj = instance_create(chest.x, chest.y, oRopePile);
                                                                                        g.at("oPlayer1/step#2", 235);
                                                                                        {
                                                                                            let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oRopePile]; b::instance_create(g, cx, a)? };
                                                                                            g.set(cx.this, v::obj, x)?;
                                                                                        }
                                                                                    } else {
                                                                                        // 236: obj = instance_create(chest.x, chest.y, oBombBag);
                                                                                        g.at("oPlayer1/step#2", 236);
                                                                                        {
                                                                                            let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oBombBag]; b::instance_create(g, cx, a)? };
                                                                                            g.set(cx.this, v::obj, x)?;
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // 237: obj.cost = 0;
        g.at("oPlayer1/step#2", 237);
        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::cost, r(0.0))?; }
        // 238: playSound(global.sndPickup);
        g.at("oPlayer1/step#2", 238);
        { let a = [g.gget(gv::sndPickup)?]; scr::playSound(g, cx, &a)? };
        // 239: if (chest == holdItem)
        g.at("oPlayer1/step#2", 239);
        if g.get(cx.this, v::chest)?.gml_eq(&g.get(cx.this, v::holdItem)?)? {
            // 241: holdItem = 0;
            g.at("oPlayer1/step#2", 241);
            g.set(cx.this, v::holdItem, r(0.0))?;
            // 242: pickupItemType = "";
            g.at("oPlayer1/step#2", 242);
            g.set(cx.this, v::pickupItemType, s!(""))?;
        }
        // 244: with (chest)
        g.at("oPlayer1/step#2", 244);
        {
            let t = g.get(cx.this, v::chest)?;
            let mut it10 = g.with_iter(cx, &t)?;
            let outer12 = (cx.this, cx.other);
            'with7: while let Some(h11) = it10.next(g) {
                (cx.this, cx.other) = (h11, outer12.0);
                // 244: instance_create(x, y, oPoof);
                g.at("oPlayer1/step#2", 244);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oPoof]; b::instance_create(g, cx, a)? };
                // 244: instance_destroy();
                g.at("oPlayer1/step#2", 244);
                b::instance_destroy(g, cx, [])?;
            }
            (cx.this, cx.other) = outer12;
        }
        // 246: kAttackPressed = false;
        g.at("oPlayer1/step#2", 246);
        g.set(cx.this, v::kAttackPressed, r(0.0))?;
    }
    // 251: if (kUp && kAttackPressed && collision_point(x, y, oFlareCrate, 0, 0))
    g.at("oPlayer1/step#2", 251);
    if and(and(g.get(cx.this, v::kUp)?.is_truthy(), g.get(cx.this, v::kAttackPressed)?.is_truthy()), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oFlareCrate, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()) {
        // 253: chest = instance_place(x, y, oFlareCrate);
        g.at("oPlayer1/step#2", 253);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oFlareCrate]; b::instance_place(g, cx, a)? };
            g.set(cx.this, v::chest, x)?;
        }
        // 254: for (...; i < 3; ...)
        g.at("oPlayer1/step#2", 254);
        // 254: i = 0;
        g.at("oPlayer1/step#2", 254);
        g.set(cx.this, v::i, r(0.0))?;
        'for8: loop {
            if !(g.get(cx.this, v::i)?.gml_lt(&r(3.0))?) { break 'for8; }
            'body9: {
                // 256: obj = instance_create(chest.x, chest.y, oFlare);
                g.at("oPlayer1/step#2", 256);
                {
                    let x = { let a = [g.get_on(cx, &g.get(cx.this, v::chest)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::chest)?, v::y)?, obj::oFlare]; b::instance_create(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 257: obj.xVel = rand(0, 3) - rand(0, 3);
                g.at("oPlayer1/step#2", 257);
                {
                    let t = g.get(cx.this, v::obj)?;
                    let x = scr::rand(g, cx, &[r(0.0), r(3.0)])?.sub(&scr::rand(g, cx, &[r(0.0), r(3.0)])?)?;
                    g.set_on(cx, &t, v::xVel, x)?;
                }
                // 258: obj.yVel = rand(1, 3) * -1;
                g.at("oPlayer1/step#2", 258);
                {
                    let t = g.get(cx.this, v::obj)?;
                    let x = scr::rand(g, cx, &[r(1.0), r(3.0)])?.mul(&r(-1.0))?;
                    g.set_on(cx, &t, v::yVel, x)?;
                }
            }
            // 254: i += 1;
            g.at("oPlayer1/step#2", 254);
            g.set(cx.this, v::i, lenient(Value::add, g.get(cx.this, v::i)?, &r(1.0)))?;
        }
        // 260: playSound(global.sndPickup);
        g.at("oPlayer1/step#2", 260);
        { let a = [g.gget(gv::sndPickup)?]; scr::playSound(g, cx, &a)? };
        // 261: if (chest == holdItem)
        g.at("oPlayer1/step#2", 261);
        if g.get(cx.this, v::chest)?.gml_eq(&g.get(cx.this, v::holdItem)?)? {
            // 263: holdItem = 0;
            g.at("oPlayer1/step#2", 263);
            g.set(cx.this, v::holdItem, r(0.0))?;
            // 264: pickupItemType = "";
            g.at("oPlayer1/step#2", 264);
            g.set(cx.this, v::pickupItemType, s!(""))?;
        }
        // 266: with (chest)
        g.at("oPlayer1/step#2", 266);
        {
            let t = g.get(cx.this, v::chest)?;
            let mut it13 = g.with_iter(cx, &t)?;
            let outer15 = (cx.this, cx.other);
            'with10: while let Some(h14) = it13.next(g) {
                (cx.this, cx.other) = (h14, outer15.0);
                // 266: instance_create(x, y, oPoof);
                g.at("oPlayer1/step#2", 266);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oPoof]; b::instance_create(g, cx, a)? };
                // 266: instance_destroy();
                g.at("oPlayer1/step#2", 266);
                b::instance_destroy(g, cx, [])?;
            }
            (cx.this, cx.other) = outer15;
        }
        // 268: kAttackPressed = false;
        g.at("oPlayer1/step#2", 268);
        g.set(cx.this, v::kAttackPressed, r(0.0))?;
    }
    // 274: if (!dead && !stunned && !whipping && collision_point(x, y, oXStart, 0, 0) && kUp && platformCharact
    g.at("oPlayer1/step#2", 274);
    if and(and(and(and(and(and(and(and(g.get(cx.this, v::dead)?.not()?.is_truthy(), g.get(cx.this, v::stunned)?.not()?.is_truthy()), g.get(cx.this, v::whipping)?.not()?.is_truthy()), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXStart, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()), g.get(cx.this, v::kUp)?.is_truthy()), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sPExit)?), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sDamselExit)?), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sTunnelExit)?) {
        // 283: if (isRoom("rOlmec") && holdItem)
        g.at("oPlayer1/step#2", 283);
        if and(scr::isRoom(g, cx, &[s!("rOlmec")])?.is_truthy(), g.get(cx.this, v::holdItem)?.is_truthy()) {
            // 285: if (holdItem.heavy)
            g.at("oPlayer1/step#2", 285);
            if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::heavy)?.is_truthy() {
                // 287: holdItem.held = false;
                g.at("oPlayer1/step#2", 287);
                { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
                // 288: holdItem = 0;
                g.at("oPlayer1/step#2", 288);
                g.set(cx.this, v::holdItem, r(0.0))?;
                // 289: pickupItemType = "";
                g.at("oPlayer1/step#2", 289);
                g.set(cx.this, v::pickupItemType, s!(""))?;
            } else {
                // 291: if (holdItem.type == "Bomb")
                g.at("oPlayer1/step#2", 291);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Bomb"))? {
                    // 293: if (holdItem.armed)
                    g.at("oPlayer1/step#2", 293);
                    if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::armed)?.is_truthy() {
                        // 295: holdItem.held = false;
                        g.at("oPlayer1/step#2", 295);
                        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
                    } else {
                        // 299: global.bombs += 1;
                        g.at("oPlayer1/step#2", 299);
                        g.gset(gv::bombs, lenient(Value::add, g.gget(gv::bombs)?, &r(1.0)))?;
                        // 300: with (holdItem)
                        g.at("oPlayer1/step#2", 300);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            let mut it16 = g.with_iter(cx, &t)?;
                            let outer18 = (cx.this, cx.other);
                            'with11: while let Some(h17) = it16.next(g) {
                                (cx.this, cx.other) = (h17, outer18.0);
                                // 300: instance_destroy();
                                g.at("oPlayer1/step#2", 300);
                                b::instance_destroy(g, cx, [])?;
                            }
                            (cx.this, cx.other) = outer18;
                        }
                    }
                    // 303: global.pickupItem = pickupItemType;
                    g.at("oPlayer1/step#2", 303);
                    g.gset(gv::pickupItem, g.get(cx.this, v::pickupItemType)?)?;
                } else {
                    // 305: if (holdItem.type == "Rope")
                    g.at("oPlayer1/step#2", 305);
                    if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Rope"))? {
                        // 307: global.rope += 1;
                        g.at("oPlayer1/step#2", 307);
                        g.gset(gv::rope, lenient(Value::add, g.gget(gv::rope)?, &r(1.0)))?;
                        // 308: with (holdItem)
                        g.at("oPlayer1/step#2", 308);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            let mut it19 = g.with_iter(cx, &t)?;
                            let outer21 = (cx.this, cx.other);
                            'with12: while let Some(h20) = it19.next(g) {
                                (cx.this, cx.other) = (h20, outer21.0);
                                // 308: instance_destroy();
                                g.at("oPlayer1/step#2", 308);
                                b::instance_destroy(g, cx, [])?;
                            }
                            (cx.this, cx.other) = outer21;
                        }
                        // 310: global.pickupItem = pickupItemType;
                        g.at("oPlayer1/step#2", 310);
                        g.gset(gv::pickupItem, g.get(cx.this, v::pickupItemType)?)?;
                    } else {
                        // 314: global.pickupItem = holdItem.type;
                        g.at("oPlayer1/step#2", 314);
                        g.gset(gv::pickupItem, g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?)?;
                        // 315: with (holdItem)
                        g.at("oPlayer1/step#2", 315);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            let mut it22 = g.with_iter(cx, &t)?;
                            let outer24 = (cx.this, cx.other);
                            'with13: while let Some(h23) = it22.next(g) {
                                (cx.this, cx.other) = (h23, outer24.0);
                                // 317: breakPieces = false;
                                g.at("oPlayer1/step#2", 317);
                                g.set(cx.this, v::breakPieces, r(0.0))?;
                                // 318: instance_destroy();
                                g.at("oPlayer1/step#2", 318);
                                b::instance_destroy(g, cx, [])?;
                            }
                            (cx.this, cx.other) = outer24;
                        }
                    }
                }
            }
        } else {
            // 322: if (isRoom("rOlmec"))
            g.at("oPlayer1/step#2", 322);
            if scr::isRoom(g, cx, &[s!("rOlmec")])?.is_truthy() {
                // 322: global.pickupItem = "";
                g.at("oPlayer1/step#2", 322);
                g.gset(gv::pickupItem, s!(""))?;
            } else {
                // 323: if (holdItem)
                g.at("oPlayer1/step#2", 323);
                if g.get(cx.this, v::holdItem)?.is_truthy() {
                    // 323: holdItem.held = false;
                    g.at("oPlayer1/step#2", 323);
                    { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
                }
            }
        }
        // 324: holdItem = 0;
        g.at("oPlayer1/step#2", 324);
        g.set(cx.this, v::holdItem, r(0.0))?;
        // 325: pickupItemType = "";
        g.at("oPlayer1/step#2", 325);
        g.set(cx.this, v::pickupItemType, s!(""))?;
        // 327: door = instance_place(x, y, oXStart);
        g.at("oPlayer1/step#2", 327);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXStart]; b::instance_place(g, cx, a)? };
            g.set(cx.this, v::door, x)?;
        }
        // 328: if (door)
        g.at("oPlayer1/step#2", 328);
        if g.get(cx.this, v::door)?.is_truthy() {
            // 328: x = door.x + 8;
            g.at("oPlayer1/step#2", 328);
            g.set(cx.this, v::x, g.get_on(cx, &g.get(cx.this, v::door)?, v::x)?.add(&r(8.0))?)?;
        }
        // 329: if (global.isDamsel)
        g.at("oPlayer1/step#2", 329);
        if g.gget(gv::isDamsel)?.is_truthy() {
            // 329: sprite_index = sDamselExit;
            g.at("oPlayer1/step#2", 329);
            g.set(cx.this, v::sprite_index, spr::sDamselExit)?;
        } else {
            // 330: if (global.isTunnelMan)
            g.at("oPlayer1/step#2", 330);
            if g.gget(gv::isTunnelMan)?.is_truthy() {
                // 330: sprite_index = sTunnelExit;
                g.at("oPlayer1/step#2", 330);
                g.set(cx.this, v::sprite_index, spr::sTunnelExit)?;
            } else {
                // 331: sprite_index = sPExit;
                g.at("oPlayer1/step#2", 331);
                g.set(cx.this, v::sprite_index, spr::sPExit)?;
            }
        }
        // 332: image_speed = 0.5;
        g.at("oPlayer1/step#2", 332);
        g.set(cx.this, v::image_speed, r(0.5))?;
        // 333: active = false;
        g.at("oPlayer1/step#2", 333);
        g.set(cx.this, v::active, r(0.0))?;
        // 334: depth = 999;
        g.at("oPlayer1/step#2", 334);
        g.set(cx.this, v::depth, r(999.0))?;
        // 335: invincible = 999;
        g.at("oPlayer1/step#2", 335);
        g.set(cx.this, v::invincible, r(999.0))?;
        // 337: pExit = xSTART;
        g.at("oPlayer1/step#2", 337);
        g.set(cx.this, v::pExit, g.get(cx.this, v::xSTART)?)?;
        // 338: if (collision_point(x, y, oXScores, 0, 0))
        g.at("oPlayer1/step#2", 338);
        if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXScores, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
            // 338: pExit = xSCORES;
            g.at("oPlayer1/step#2", 338);
            g.set(cx.this, v::pExit, g.get(cx.this, v::xSCORES)?)?;
        } else {
            // 339: if (collision_point(x, y, oXTutorial, 0, 0))
            g.at("oPlayer1/step#2", 339);
            if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXTutorial, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                // 339: pExit = xTUTORIAL;
                g.at("oPlayer1/step#2", 339);
                g.set(cx.this, v::pExit, g.get(cx.this, v::xTUTORIAL)?)?;
            } else {
                // 340: if (collision_point(x, y, oXTitle, 0, 0))
                g.at("oPlayer1/step#2", 340);
                if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXTitle, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                    // 340: pExit = xTITLE;
                    g.at("oPlayer1/step#2", 340);
                    g.set(cx.this, v::pExit, g.get(cx.this, v::xTITLE)?)?;
                } else {
                    // 341: if (collision_point(x, y, oXEnd, 0, 0))
                    g.at("oPlayer1/step#2", 341);
                    if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXEnd, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                        // 341: pExit = xEND;
                        g.at("oPlayer1/step#2", 341);
                        g.set(cx.this, v::pExit, g.get(cx.this, v::xEND)?)?;
                    } else {
                        // 342: if (collision_point(x, y, oXShortcut5, 0, 0))
                        g.at("oPlayer1/step#2", 342);
                        if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXShortcut5, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                            // 342: pExit = xSHORTCUT5;
                            g.at("oPlayer1/step#2", 342);
                            g.set(cx.this, v::pExit, g.get(cx.this, v::xSHORTCUT5)?)?;
                        } else {
                            // 343: if (collision_point(x, y, oXShortcut9, 0, 0))
                            g.at("oPlayer1/step#2", 343);
                            if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXShortcut9, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                                // 343: pExit = xSHORTCUT9;
                                g.at("oPlayer1/step#2", 343);
                                g.set(cx.this, v::pExit, g.get(cx.this, v::xSHORTCUT9)?)?;
                            } else {
                                // 344: if (collision_point(x, y, oXShortcut13, 0, 0))
                                g.at("oPlayer1/step#2", 344);
                                if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXShortcut13, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                                    // 344: pExit = xSHORTCUT13;
                                    g.at("oPlayer1/step#2", 344);
                                    g.set(cx.this, v::pExit, g.get(cx.this, v::xSHORTCUT13)?)?;
                                } else {
                                    // 345: if (collision_point(x, y, oXSun, 0, 0))
                                    g.at("oPlayer1/step#2", 345);
                                    if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXSun, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                                        // 345: pExit = xSUN;
                                        g.at("oPlayer1/step#2", 345);
                                        g.set(cx.this, v::pExit, g.get(cx.this, v::xSUN)?)?;
                                    } else {
                                        // 346: if (collision_point(x, y, oXMoon, 0, 0))
                                        g.at("oPlayer1/step#2", 346);
                                        if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXMoon, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                                            // 346: pExit = xMOON;
                                            g.at("oPlayer1/step#2", 346);
                                            g.set(cx.this, v::pExit, g.get(cx.this, v::xMOON)?)?;
                                        } else {
                                            // 347: if (collision_point(x, y, oXStars, 0, 0))
                                            g.at("oPlayer1/step#2", 347);
                                            if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXStars, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                                                // 347: pExit = xSTARS;
                                                g.at("oPlayer1/step#2", 347);
                                                g.set(cx.this, v::pExit, g.get(cx.this, v::xSTARS)?)?;
                                            } else {
                                                // 348: if (collision_point(x, y, oXChange, 0, 0))
                                                g.at("oPlayer1/step#2", 348);
                                                if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXChange, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                                                    // 348: pExit = xCHANGE;
                                                    g.at("oPlayer1/step#2", 348);
                                                    g.set(cx.this, v::pExit, g.get(cx.this, v::xCHANGE)?)?;
                                                } else {
                                                    // 349: if (collision_point(x, y, oXChange2, 0, 0))
                                                    g.at("oPlayer1/step#2", 349);
                                                    if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXChange2, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                                                        // 349: pExit = xCHANGE2;
                                                        g.at("oPlayer1/step#2", 349);
                                                        g.set(cx.this, v::pExit, g.get(cx.this, v::xCHANGE2)?)?;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // 351: if (pExit != xCHANGE2)
        g.at("oPlayer1/step#2", 351);
        if g.get(cx.this, v::pExit)?.gml_ne(&g.get(cx.this, v::xCHANGE2)?)? {
            // 351: stopAllMusic();
            g.at("oPlayer1/step#2", 351);
            scr::stopAllMusic(g, cx, &[])?;
        }
        // 353: playSound(global.sndSteps);
        g.at("oPlayer1/step#2", 353);
        { let a = [g.gget(gv::sndSteps)?]; scr::playSound(g, cx, &a)? };
    }
    // 359: if (!dead && !stunned && !whipping && collision_point(x, y, oExit, 0, 0) && kUp && platformCharacter
    g.at("oPlayer1/step#2", 359);
    if and(and(and(and(and(and(and(and(g.get(cx.this, v::dead)?.not()?.is_truthy(), g.get(cx.this, v::stunned)?.not()?.is_truthy()), g.get(cx.this, v::whipping)?.not()?.is_truthy()), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oExit, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()), g.get(cx.this, v::kUp)?.is_truthy()), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sPExit)?), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sDamselExit)?), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sTunnelExit)?) {
        // 367: holdArrow = 0;
        g.at("oPlayer1/step#2", 367);
        g.set(cx.this, v::holdArrow, r(0.0))?;
        // 368: global.pickupItem = "";
        g.at("oPlayer1/step#2", 368);
        g.gset(gv::pickupItem, s!(""))?;
        // 369: if (holdItem)
        g.at("oPlayer1/step#2", 369);
        if g.get(cx.this, v::holdItem)?.is_truthy() {
            // 371: if (holdItem.type == "Gold Idol")
            g.at("oPlayer1/step#2", 371);
            if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Gold Idol"))? {
                // 373: if (isRealLevel())
                g.at("oPlayer1/step#2", 373);
                if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                    // 373: global.idolsConverted += 1;
                    g.at("oPlayer1/step#2", 373);
                    g.gset(gv::idolsConverted, lenient(Value::add, g.gget(gv::idolsConverted)?, &r(1.0)))?;
                }
                // 374: global.money += holdItem.value * (global.levelType + 1);
                g.at("oPlayer1/step#2", 374);
                g.gset(gv::money, lenient(Value::add, g.gget(gv::money)?, &g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::value)?.mul(&g.gget(gv::levelType)?.add(&r(1.0))?)?))?;
                // 375: if (holdItem.sprite_index == sCrystalSkull)
                g.at("oPlayer1/step#2", 375);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::sprite_index)?.gml_eq(&spr::sCrystalSkull)? {
                    // 375: global.skulls += 1;
                    g.at("oPlayer1/step#2", 375);
                    g.gset(gv::skulls, lenient(Value::add, g.gget(gv::skulls)?, &r(1.0)))?;
                } else {
                    // 376: global.idols += 1;
                    g.at("oPlayer1/step#2", 376);
                    g.gset(gv::idols, lenient(Value::add, g.gget(gv::idols)?, &r(1.0)))?;
                }
                // 377: playSound(global.sndCoin);
                g.at("oPlayer1/step#2", 377);
                { let a = [g.gget(gv::sndCoin)?]; scr::playSound(g, cx, &a)? };
                // 378: instance_create(x, y - 8, oBigCollect);
                g.at("oPlayer1/step#2", 378);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(8.0))?, obj::oBigCollect]; b::instance_create(g, cx, a)? };
                // 379: with (holdItem)
                g.at("oPlayer1/step#2", 379);
                {
                    let t = g.get(cx.this, v::holdItem)?;
                    let mut it25 = g.with_iter(cx, &t)?;
                    let outer27 = (cx.this, cx.other);
                    'with14: while let Some(h26) = it25.next(g) {
                        (cx.this, cx.other) = (h26, outer27.0);
                        // 379: instance_destroy();
                        g.at("oPlayer1/step#2", 379);
                        b::instance_destroy(g, cx, [])?;
                    }
                    (cx.this, cx.other) = outer27;
                }
                // 380: holdItem = 0;
                g.at("oPlayer1/step#2", 380);
                g.set(cx.this, v::holdItem, r(0.0))?;
            } else {
                // 382: if (holdItem.type == "Damsel")
                g.at("oPlayer1/step#2", 382);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Damsel"))? {
                    // 384: if (holdItem.hp > 0)
                    g.at("oPlayer1/step#2", 384);
                    if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::hp)?.gml_gt(&r(0.0))? {
                        // 387: if (isRealLevel())
                        g.at("oPlayer1/step#2", 387);
                        if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                            // 387: global.damselsSavedTotal += 1;
                            g.at("oPlayer1/step#2", 387);
                            g.gset(gv::damselsSavedTotal, lenient(Value::add, g.gget(gv::damselsSavedTotal)?, &r(1.0)))?;
                        }
                        // 388: global.damsels += 1;
                        g.at("oPlayer1/step#2", 388);
                        g.gset(gv::damsels, lenient(Value::add, g.gget(gv::damsels)?, &r(1.0)))?;
                        // 389: global.xdamsels += 1;
                        g.at("oPlayer1/step#2", 389);
                        g.gset(gv::xdamsels, lenient(Value::add, g.gget(gv::xdamsels)?, &r(1.0)))?;
                        // 390: door = instance_place(x, y, oExit);
                        g.at("oPlayer1/step#2", 390);
                        {
                            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oExit]; b::instance_place(g, cx, a)? };
                            g.set(cx.this, v::door, x)?;
                        }
                        // 391: holdItem.x = door.x + 8;
                        g.at("oPlayer1/step#2", 391);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            g.set_on(cx, &t, v::x, g.get_on(cx, &g.get(cx.this, v::door)?, v::x)?.add(&r(8.0))?)?;
                        }
                        // 392: holdItem.y = door.y + 8;
                        g.at("oPlayer1/step#2", 392);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            g.set_on(cx, &t, v::y, g.get_on(cx, &g.get(cx.this, v::door)?, v::y)?.add(&r(8.0))?)?;
                        }
                        // 393: with (holdItem)
                        g.at("oPlayer1/step#2", 393);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            let mut it28 = g.with_iter(cx, &t)?;
                            let outer30 = (cx.this, cx.other);
                            'with15: while let Some(h29) = it28.next(g) {
                                (cx.this, cx.other) = (h29, outer30.0);
                                // 395: if (global.isDamsel)
                                g.at("oPlayer1/step#2", 395);
                                if g.gget(gv::isDamsel)?.is_truthy() {
                                    // 395: sprite_index = sPExit;
                                    g.at("oPlayer1/step#2", 395);
                                    g.set(cx.this, v::sprite_index, spr::sPExit)?;
                                } else {
                                    // 396: sprite_index = sDamselExit;
                                    g.at("oPlayer1/step#2", 396);
                                    g.set(cx.this, v::sprite_index, spr::sDamselExit)?;
                                }
                                // 397: status = 4;
                                g.at("oPlayer1/step#2", 397);
                                g.set(cx.this, v::status, r(4.0))?;
                                // 398: held = false;
                                g.at("oPlayer1/step#2", 398);
                                g.set(cx.this, v::held, r(0.0))?;
                                // 399: xVel = 0;
                                g.at("oPlayer1/step#2", 399);
                                g.set(cx.this, v::xVel, r(0.0))?;
                                // 400: yVel = 0;
                                g.at("oPlayer1/step#2", 400);
                                g.set(cx.this, v::yVel, r(0.0))?;
                                // 401: playSound(global.sndSteps);
                                g.at("oPlayer1/step#2", 401);
                                { let a = [g.gget(gv::sndSteps)?]; scr::playSound(g, cx, &a)? };
                                // 402: depth = 1000;
                                g.at("oPlayer1/step#2", 402);
                                g.set(cx.this, v::depth, r(1000.0))?;
                                // 403: active = false;
                                g.at("oPlayer1/step#2", 403);
                                g.set(cx.this, v::active, r(0.0))?;
                            }
                            (cx.this, cx.other) = outer30;
                        }
                        // 405: holdItem = 0;
                        g.at("oPlayer1/step#2", 405);
                        g.set(cx.this, v::holdItem, r(0.0))?;
                    } else {
                        // 409: holdItem.status = 2;
                        g.at("oPlayer1/step#2", 409);
                        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::status, r(2.0))?; }
                        // 410: holdItem.held = false;
                        g.at("oPlayer1/step#2", 410);
                        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
                        // 411: holdItem = 0;
                        g.at("oPlayer1/step#2", 411);
                        g.set(cx.this, v::holdItem, r(0.0))?;
                        // 412: pickupItemType = "";
                        g.at("oPlayer1/step#2", 412);
                        g.set(cx.this, v::pickupItemType, s!(""))?;
                    }
                } else {
                    // 415: if (holdItem.heavy)
                    g.at("oPlayer1/step#2", 415);
                    if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::heavy)?.is_truthy() {
                        // 417: holdItem.held = false;
                        g.at("oPlayer1/step#2", 417);
                        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
                        // 418: holdItem = 0;
                        g.at("oPlayer1/step#2", 418);
                        g.set(cx.this, v::holdItem, r(0.0))?;
                        // 419: pickupItemType = "";
                        g.at("oPlayer1/step#2", 419);
                        g.set(cx.this, v::pickupItemType, s!(""))?;
                    } else {
                        // 421: if (holdItem.type == "Bomb")
                        g.at("oPlayer1/step#2", 421);
                        if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Bomb"))? {
                            // 423: if (holdItem.armed)
                            g.at("oPlayer1/step#2", 423);
                            if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::armed)?.is_truthy() {
                                // 425: holdItem.held = false;
                                g.at("oPlayer1/step#2", 425);
                                { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
                            } else {
                                // 429: global.bombs += 1;
                                g.at("oPlayer1/step#2", 429);
                                g.gset(gv::bombs, lenient(Value::add, g.gget(gv::bombs)?, &r(1.0)))?;
                                // 430: with (holdItem)
                                g.at("oPlayer1/step#2", 430);
                                {
                                    let t = g.get(cx.this, v::holdItem)?;
                                    let mut it31 = g.with_iter(cx, &t)?;
                                    let outer33 = (cx.this, cx.other);
                                    'with16: while let Some(h32) = it31.next(g) {
                                        (cx.this, cx.other) = (h32, outer33.0);
                                        // 430: instance_destroy();
                                        g.at("oPlayer1/step#2", 430);
                                        b::instance_destroy(g, cx, [])?;
                                    }
                                    (cx.this, cx.other) = outer33;
                                }
                            }
                            // 433: global.pickupItem = pickupItemType;
                            g.at("oPlayer1/step#2", 433);
                            g.gset(gv::pickupItem, g.get(cx.this, v::pickupItemType)?)?;
                        } else {
                            // 435: if (holdItem.type == "Rope")
                            g.at("oPlayer1/step#2", 435);
                            if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Rope"))? {
                                // 437: global.rope += 1;
                                g.at("oPlayer1/step#2", 437);
                                g.gset(gv::rope, lenient(Value::add, g.gget(gv::rope)?, &r(1.0)))?;
                                // 438: with (holdItem)
                                g.at("oPlayer1/step#2", 438);
                                {
                                    let t = g.get(cx.this, v::holdItem)?;
                                    let mut it34 = g.with_iter(cx, &t)?;
                                    let outer36 = (cx.this, cx.other);
                                    'with17: while let Some(h35) = it34.next(g) {
                                        (cx.this, cx.other) = (h35, outer36.0);
                                        // 438: instance_destroy();
                                        g.at("oPlayer1/step#2", 438);
                                        b::instance_destroy(g, cx, [])?;
                                    }
                                    (cx.this, cx.other) = outer36;
                                }
                                // 440: global.pickupItem = pickupItemType;
                                g.at("oPlayer1/step#2", 440);
                                g.gset(gv::pickupItem, g.get(cx.this, v::pickupItemType)?)?;
                            } else {
                                // 444: global.pickupItem = holdItem.type;
                                g.at("oPlayer1/step#2", 444);
                                g.gset(gv::pickupItem, g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?)?;
                                // 445: with (holdItem)
                                g.at("oPlayer1/step#2", 445);
                                {
                                    let t = g.get(cx.this, v::holdItem)?;
                                    let mut it37 = g.with_iter(cx, &t)?;
                                    let outer39 = (cx.this, cx.other);
                                    'with18: while let Some(h38) = it37.next(g) {
                                        (cx.this, cx.other) = (h38, outer39.0);
                                        // 447: breakPieces = false;
                                        g.at("oPlayer1/step#2", 447);
                                        g.set(cx.this, v::breakPieces, r(0.0))?;
                                        // 448: instance_destroy();
                                        g.at("oPlayer1/step#2", 448);
                                        b::instance_destroy(g, cx, [])?;
                                    }
                                    (cx.this, cx.other) = outer39;
                                }
                            }
                        }
                    }
                }
            }
            // 451: holdItem = 0;
            g.at("oPlayer1/step#2", 451);
            g.set(cx.this, v::holdItem, r(0.0))?;
            // 452: pickupItemType = "";
            g.at("oPlayer1/step#2", 452);
            g.set(cx.this, v::pickupItemType, s!(""))?;
        }
        // 455: door = instance_place(x, y, oExit);
        g.at("oPlayer1/step#2", 455);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oExit]; b::instance_place(g, cx, a)? };
            g.set(cx.this, v::door, x)?;
        }
        // 456: if (door)
        g.at("oPlayer1/step#2", 456);
        if g.get(cx.this, v::door)?.is_truthy() {
            // 458: x = door.x + 8;
            g.at("oPlayer1/step#2", 458);
            g.set(cx.this, v::x, g.get_on(cx, &g.get(cx.this, v::door)?, v::x)?.add(&r(8.0))?)?;
            // 459: y = door.y + 8;
            g.at("oPlayer1/step#2", 459);
            g.set(cx.this, v::y, g.get_on(cx, &g.get(cx.this, v::door)?, v::y)?.add(&r(8.0))?)?;
        }
        // 463: global.money += global.collect;
        g.at("oPlayer1/step#2", 463);
        g.gset(gv::money, lenient(Value::add, g.gget(gv::money)?, &g.gget(gv::collect)?))?;
        // 464: global.xmoney += global.collect;
        g.at("oPlayer1/step#2", 464);
        g.gset(gv::xmoney, lenient(Value::add, g.gget(gv::xmoney)?, &g.gget(gv::collect)?))?;
        // 465: global.collect = 0;
        g.at("oPlayer1/step#2", 465);
        g.gset(gv::collect, r(0.0))?;
        // 467: if (global.isDamsel)
        g.at("oPlayer1/step#2", 467);
        if g.gget(gv::isDamsel)?.is_truthy() {
            // 467: sprite_index = sDamselExit;
            g.at("oPlayer1/step#2", 467);
            g.set(cx.this, v::sprite_index, spr::sDamselExit)?;
        } else {
            // 468: if (global.isTunnelMan)
            g.at("oPlayer1/step#2", 468);
            if g.gget(gv::isTunnelMan)?.is_truthy() {
                // 468: sprite_index = sTunnelExit;
                g.at("oPlayer1/step#2", 468);
                g.set(cx.this, v::sprite_index, spr::sTunnelExit)?;
            } else {
                // 469: sprite_index = sPExit;
                g.at("oPlayer1/step#2", 469);
                g.set(cx.this, v::sprite_index, spr::sPExit)?;
            }
        }
        // 470: image_speed = 0.5;
        g.at("oPlayer1/step#2", 470);
        g.set(cx.this, v::image_speed, r(0.5))?;
        // 471: active = false;
        g.at("oPlayer1/step#2", 471);
        g.set(cx.this, v::active, r(0.0))?;
        // 472: invincible = 999;
        g.at("oPlayer1/step#2", 472);
        g.set(cx.this, v::invincible, r(999.0))?;
        // 473: depth = 999;
        g.at("oPlayer1/step#2", 473);
        g.set(cx.this, v::depth, r(999.0))?;
        // 474: if (global.thiefLevel > 0)
        g.at("oPlayer1/step#2", 474);
        if g.gget(gv::thiefLevel)?.gml_gt(&r(0.0))? {
            // 474: global.thiefLevel -= 1;
            g.at("oPlayer1/step#2", 474);
            g.gset(gv::thiefLevel, lenient(Value::sub, g.gget(gv::thiefLevel)?, &r(1.0)))?;
        }
        // 475: if (global.currLevel == 1)
        g.at("oPlayer1/step#2", 475);
        if g.gget(gv::currLevel)?.gml_eq(&r(1.0))? {
            // 475: global.currLevel += firstLevelSkip;
            g.at("oPlayer1/step#2", 475);
            g.gset(gv::currLevel, lenient(Value::add, g.gget(gv::currLevel)?, &g.get(cx.this, v::firstLevelSkip)?))?;
        } else {
            // 476: global.currLevel += levelSkip;
            g.at("oPlayer1/step#2", 476);
            g.gset(gv::currLevel, lenient(Value::add, g.gget(gv::currLevel)?, &g.get(cx.this, v::levelSkip)?))?;
        }
        // 477: stopAllMusic();
        g.at("oPlayer1/step#2", 477);
        scr::stopAllMusic(g, cx, &[])?;
        // 478: playSound(global.sndSteps);
        g.at("oPlayer1/step#2", 478);
        { let a = [g.gget(gv::sndSteps)?]; scr::playSound(g, cx, &a)? };
        // 479: if (collision_point(x, y, oXMarket, 0, 0))
        g.at("oPlayer1/step#2", 479);
        if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXMarket, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
            // 479: global.genBlackMarket = true;
            g.at("oPlayer1/step#2", 479);
            g.gset(gv::genBlackMarket, r(1.0))?;
        }
        // 480: if (collision_point(x, y, oXGold, 0, 0))
        g.at("oPlayer1/step#2", 480);
        if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oXGold, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
            // 480: global.cityOfGold = true;
            g.at("oPlayer1/step#2", 480);
            g.gset(gv::cityOfGold, r(1.0))?;
        }
        // 481: obj = collision_point(x, y, oExit, 0, 0);
        g.at("oPlayer1/step#2", 481);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oExit, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? };
            g.set(cx.this, v::obj, x)?;
        }
        // 482: if (obj)
        g.at("oPlayer1/step#2", 482);
        if g.get(cx.this, v::obj)?.is_truthy() {
            // 484: if (obj.leadsTo != "")
            g.at("oPlayer1/step#2", 484);
            if g.get_on(cx, &g.get(cx.this, v::obj)?, v::leadsTo)?.gml_ne(&s!(""))? {
                // 486: global.nextCustomLevel = obj.leadsTo;
                g.at("oPlayer1/step#2", 486);
                g.gset(gv::nextCustomLevel, g.get_on(cx, &g.get(cx.this, v::obj)?, v::leadsTo)?)?;
            }
        }
        // 490: with (oMonkey)
        g.at("oPlayer1/step#2", 490);
        {
            let t = obj::oMonkey;
            let mut it40 = g.with_iter(cx, &t)?;
            let outer42 = (cx.this, cx.other);
            'with19: while let Some(h41) = it40.next(g) {
                (cx.this, cx.other) = (h41, outer42.0);
                // 493: if (status == 7)
                g.at("oPlayer1/step#2", 493);
                if g.get(cx.this, v::status)?.gml_eq(&r(7.0))? {
                    // 495: xVel = rand(0, 1) - rand(0, 1);
                    g.at("oPlayer1/step#2", 495);
                    {
                        let x = scr::rand(g, cx, &[r(0.0), r(1.0)])?.sub(&scr::rand(g, cx, &[r(0.0), r(1.0)])?)?;
                        g.set(cx.this, v::xVel, x)?;
                    }
                    // 496: yVel = -4;
                    g.at("oPlayer1/step#2", 496);
                    g.set(cx.this, v::yVel, r(-4.0))?;
                    // 497: status = 1;
                    g.at("oPlayer1/step#2", 497);
                    g.set(cx.this, v::status, r(1.0))?;
                    // 498: vineCounter = 20;
                    g.at("oPlayer1/step#2", 498);
                    g.set(cx.this, v::vineCounter, r(20.0))?;
                    // 499: grabCounter = 60;
                    g.at("oPlayer1/step#2", 499);
                    g.set(cx.this, v::grabCounter, r(60.0))?;
                }
            }
            (cx.this, cx.other) = outer42;
        }
    }
    // 507: if (checkAttackPressed() || checkStartPressed() && dead)
    g.at("oPlayer1/step#2", 507);
    if and(or(scr::checkAttackPressed(g, cx, &[])?.is_truthy(), scr::checkStartPressed(g, cx, &[])?.is_truthy()), g.get(cx.this, v::dead)?.is_truthy()) {
        // 509: if (oGame.moneyCount < global.money || oGame.drawStatus < 3)
        g.at("oPlayer1/step#2", 509);
        if or(g.get_on(cx, &obj::oGame, v::moneyCount)?.gml_lt(&g.gget(gv::money)?)?, g.get_on(cx, &obj::oGame, v::drawStatus)?.gml_lt(&r(3.0))?) {
            // 511: oGame.drawStatus = 3;
            g.at("oPlayer1/step#2", 511);
            { let t = obj::oGame; g.set_on(cx, &t, v::drawStatus, r(3.0))?; }
            // 512: oGame.moneyCount = global.money;
            g.at("oPlayer1/step#2", 512);
            { let t = obj::oGame; g.set_on(cx, &t, v::moneyCount, g.gget(gv::money)?)?; }
        } else {
            // 517: if (isRealLevel())
            g.at("oPlayer1/step#2", 517);
            if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                // 517: global.levelDeaths[global.currLevel - 1] += 1;
                g.at("oPlayer1/step#2", 517);
                {
                    let j = ix(&g.gget(gv::currLevel)?.sub(&r(1.0))?)?;
                    g.gset_a(gv::levelDeaths, 0, j, lenient(Value::add, g.gget_a(gv::levelDeaths, 0, j)?, &r(1.0)))?;
                }
            }
            // 519: if (gamepad.attackPressed)
            g.at("oPlayer1/step#2", 519);
            if g.get_on(cx, &g.gget(gv::gamepad)?, v::attackPressed)?.is_truthy() {
                // 519: gamepad.attackPressed = false;
                g.at("oPlayer1/step#2", 519);
                { let t = g.gget(gv::gamepad)?; g.set_on(cx, &t, v::attackPressed, r(0.0))?; }
            }
            // 520: if (gamepad.startPressed)
            g.at("oPlayer1/step#2", 520);
            if g.get_on(cx, &g.gget(gv::gamepad)?, v::startPressed)?.is_truthy() {
                // 520: gamepad.startPressed = false;
                g.at("oPlayer1/step#2", 520);
                { let t = g.gget(gv::gamepad)?; g.set_on(cx, &t, v::startPressed, r(0.0))?; }
            }
            // 521: global.prevCustomLevel = "";
            g.at("oPlayer1/step#2", 521);
            g.gset(gv::prevCustomLevel, s!(""))?;
            // 522: if (global.testLevel != "")
            g.at("oPlayer1/step#2", 522);
            if g.gget(gv::testLevel)?.gml_ne(&s!(""))? {
                // 524: scrClearGlobals();
                g.at("oPlayer1/step#2", 524);
                scr::scrClearGlobals(g, cx, &[])?;
                // 525: room_goto(rLevelEditor);
                g.at("oPlayer1/step#2", 525);
                b::room_goto(g, cx, [rm::rLevelEditor])?;
            } else {
                // 527: if (global.customLevel)
                g.at("oPlayer1/step#2", 527);
                if g.gget(gv::customLevel)?.is_truthy() {
                    // 529: scrClearGlobals();
                    g.at("oPlayer1/step#2", 529);
                    scr::scrClearGlobals(g, cx, &[])?;
                    // 530: global.customLevel = false;
                    g.at("oPlayer1/step#2", 530);
                    g.gset(gv::customLevel, r(0.0))?;
                    // 531: room_goto(rLoadLevel);
                    g.at("oPlayer1/step#2", 531);
                    b::room_goto(g, cx, [rm::rLoadLevel])?;
                } else {
                    // 535: scrClearGlobals();
                    g.at("oPlayer1/step#2", 535);
                    scr::scrClearGlobals(g, cx, &[])?;
                    // 536: if (isRoom("rSun"))
                    g.at("oPlayer1/step#2", 536);
                    if scr::isRoom(g, cx, &[s!("rSun")])?.is_truthy() {
                        // 536: global.scoresStart = 1;
                        g.at("oPlayer1/step#2", 536);
                        g.gset(gv::scoresStart, r(1.0))?;
                    }
                    // 537: if (isRoom("rMoon"))
                    g.at("oPlayer1/step#2", 537);
                    if scr::isRoom(g, cx, &[s!("rMoon")])?.is_truthy() {
                        // 537: global.scoresStart = 2;
                        g.at("oPlayer1/step#2", 537);
                        g.gset(gv::scoresStart, r(2.0))?;
                    }
                    // 538: if (isRoom("rStars"))
                    g.at("oPlayer1/step#2", 538);
                    if scr::isRoom(g, cx, &[s!("rStars")])?.is_truthy() {
                        // 538: global.scoresStart = 3;
                        g.at("oPlayer1/step#2", 538);
                        g.gset(gv::scoresStart, r(3.0))?;
                    }
                    // 539: room_goto(rHighscores);
                    g.at("oPlayer1/step#2", 539);
                    b::room_goto(g, cx, [rm::rHighscores])?;
                }
            }
        }
    }
    // 544: inGame = true;
    g.at("oPlayer1/step#2", 544);
    g.set(cx.this, v::inGame, r(1.0))?;
    // 545: if (!isLevel())
    g.at("oPlayer1/step#2", 545);
    if scr::isLevel(g, cx, &[])?.not()?.is_truthy() {
        // 547: inGame = false;
        g.at("oPlayer1/step#2", 547);
        g.set(cx.this, v::inGame, r(0.0))?;
    }
    // 550: if (dead || stunned || !active)
    g.at("oPlayer1/step#2", 550);
    if or(or(g.get(cx.this, v::dead)?.is_truthy(), g.get(cx.this, v::stunned)?.is_truthy()), g.get(cx.this, v::active)?.not()?.is_truthy()) {
    } else {
        // 554: if (inGame && kItemPressed && !whipping)
        g.at("oPlayer1/step#2", 554);
        if and(and(g.get(cx.this, v::inGame)?.is_truthy(), g.get(cx.this, v::kItemPressed)?.is_truthy()), g.get(cx.this, v::whipping)?.not()?.is_truthy()) {
            // 557: if (holdItem)
            g.at("oPlayer1/step#2", 557);
            if g.get(cx.this, v::holdItem)?.is_truthy() {
                // 559: if (holdItem.sprite_index == sBombArmed)
                g.at("oPlayer1/step#2", 559);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::sprite_index)?.gml_eq(&spr::sBombArmed)? {
                } else {
                    // 563: if (holdItem.sprite_index == sBomb)
                    g.at("oPlayer1/step#2", 563);
                    if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::sprite_index)?.gml_eq(&spr::sBomb)? {
                        // 565: with (holdItem)
                        g.at("oPlayer1/step#2", 565);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            let mut it43 = g.with_iter(cx, &t)?;
                            let outer45 = (cx.this, cx.other);
                            'with20: while let Some(h44) = it43.next(g) {
                                (cx.this, cx.other) = (h44, outer45.0);
                                // 567: global.bombs += 1;
                                g.at("oPlayer1/step#2", 567);
                                g.gset(gv::bombs, lenient(Value::add, g.gget(gv::bombs)?, &r(1.0)))?;
                                // 568: instance_destroy();
                                g.at("oPlayer1/step#2", 568);
                                b::instance_destroy(g, cx, [])?;
                            }
                            (cx.this, cx.other) = outer45;
                        }
                        // 571: if (global.rope > 0)
                        g.at("oPlayer1/step#2", 571);
                        if g.gget(gv::rope)?.gml_gt(&r(0.0))? {
                            // 573: holdItem = instance_create(x, y, oRopeThrow);
                            g.at("oPlayer1/step#2", 573);
                            {
                                let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oRopeThrow]; b::instance_create(g, cx, a)? };
                                g.set(cx.this, v::holdItem, x)?;
                            }
                            // 574: holdItem.held = true;
                            g.at("oPlayer1/step#2", 574);
                            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(1.0))?; }
                            // 575: global.rope -= 1;
                            g.at("oPlayer1/step#2", 575);
                            g.gset(gv::rope, lenient(Value::sub, g.gget(gv::rope)?, &r(1.0)))?;
                            // 576: whoaTimer = whoaTimerMax;
                            g.at("oPlayer1/step#2", 576);
                            g.set(cx.this, v::whoaTimer, g.get(cx.this, v::whoaTimerMax)?)?;
                        } else {
                            // 580: scrHoldItem(pickupItemType);
                            g.at("oPlayer1/step#2", 580);
                            { let a = [g.get(cx.this, v::pickupItemType)?]; scr::scrHoldItem(g, cx, &a)? };
                        }
                    } else {
                        // 583: if (holdItem.sprite_index == sRopeEnd)
                        g.at("oPlayer1/step#2", 583);
                        if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::sprite_index)?.gml_eq(&spr::sRopeEnd)? {
                            // 585: with (holdItem)
                            g.at("oPlayer1/step#2", 585);
                            {
                                let t = g.get(cx.this, v::holdItem)?;
                                let mut it46 = g.with_iter(cx, &t)?;
                                let outer48 = (cx.this, cx.other);
                                'with21: while let Some(h47) = it46.next(g) {
                                    (cx.this, cx.other) = (h47, outer48.0);
                                    // 587: global.rope += 1;
                                    g.at("oPlayer1/step#2", 587);
                                    g.gset(gv::rope, lenient(Value::add, g.gget(gv::rope)?, &r(1.0)))?;
                                    // 588: instance_destroy();
                                    g.at("oPlayer1/step#2", 588);
                                    b::instance_destroy(g, cx, [])?;
                                }
                                (cx.this, cx.other) = outer48;
                            }
                            // 591: scrHoldItem(pickupItemType);
                            g.at("oPlayer1/step#2", 591);
                            { let a = [g.get(cx.this, v::pickupItemType)?]; scr::scrHoldItem(g, cx, &a)? };
                        } else {
                            // 593: if (!holdItem.heavy && holdItem.cost == 0)
                            g.at("oPlayer1/step#2", 593);
                            if and(g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::heavy)?.not()?.is_truthy(), g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::cost)?.gml_eq(&r(0.0))?) {
                                // 595: if (global.bombs > 0 || global.rope > 0)
                                g.at("oPlayer1/step#2", 595);
                                if or(g.gget(gv::bombs)?.gml_gt(&r(0.0))?, g.gget(gv::rope)?.gml_gt(&r(0.0))?) {
                                    // 597: pickupItemType = holdItem.type;
                                    g.at("oPlayer1/step#2", 597);
                                    g.set(cx.this, v::pickupItemType, g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?)?;
                                    // 598: if (holdItem.type == "Bow" && bowArmed)
                                    g.at("oPlayer1/step#2", 598);
                                    if and(g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Bow"))?, g.get(cx.this, v::bowArmed)?.is_truthy()) {
                                        // 600: scrFireBow();
                                        g.at("oPlayer1/step#2", 600);
                                        scr::scrFireBow(g, cx, &[])?;
                                    }
                                    // 602: with (holdItem)
                                    g.at("oPlayer1/step#2", 602);
                                    {
                                        let t = g.get(cx.this, v::holdItem)?;
                                        let mut it49 = g.with_iter(cx, &t)?;
                                        let outer51 = (cx.this, cx.other);
                                        'with22: while let Some(h50) = it49.next(g) {
                                            (cx.this, cx.other) = (h50, outer51.0);
                                            // 604: breakPieces = false;
                                            g.at("oPlayer1/step#2", 604);
                                            g.set(cx.this, v::breakPieces, r(0.0))?;
                                            // 605: instance_destroy();
                                            g.at("oPlayer1/step#2", 605);
                                            b::instance_destroy(g, cx, [])?;
                                        }
                                        (cx.this, cx.other) = outer51;
                                    }
                                }
                                // 609: if (global.bombs > 0)
                                g.at("oPlayer1/step#2", 609);
                                if g.gget(gv::bombs)?.gml_gt(&r(0.0))? {
                                    // 611: holdItem = instance_create(x, y, oBomb);
                                    g.at("oPlayer1/step#2", 611);
                                    {
                                        let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oBomb]; b::instance_create(g, cx, a)? };
                                        g.set(cx.this, v::holdItem, x)?;
                                    }
                                    // 612: if (global.hasStickyBombs)
                                    g.at("oPlayer1/step#2", 612);
                                    if g.gget(gv::hasStickyBombs)?.is_truthy() {
                                        // 612: holdItem.sticky = true;
                                        g.at("oPlayer1/step#2", 612);
                                        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::sticky, r(1.0))?; }
                                    }
                                    // 613: holdItem.held = true;
                                    g.at("oPlayer1/step#2", 613);
                                    { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(1.0))?; }
                                    // 614: global.bombs -= 1;
                                    g.at("oPlayer1/step#2", 614);
                                    g.gset(gv::bombs, lenient(Value::sub, g.gget(gv::bombs)?, &r(1.0)))?;
                                    // 615: whoaTimer = whoaTimerMax;
                                    g.at("oPlayer1/step#2", 615);
                                    g.set(cx.this, v::whoaTimer, g.get(cx.this, v::whoaTimerMax)?)?;
                                } else {
                                    // 617: if (global.rope > 0)
                                    g.at("oPlayer1/step#2", 617);
                                    if g.gget(gv::rope)?.gml_gt(&r(0.0))? {
                                        // 619: holdItem = instance_create(x, y, oRopeThrow);
                                        g.at("oPlayer1/step#2", 619);
                                        {
                                            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oRopeThrow]; b::instance_create(g, cx, a)? };
                                            g.set(cx.this, v::holdItem, x)?;
                                        }
                                        // 620: holdItem.held = true;
                                        g.at("oPlayer1/step#2", 620);
                                        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(1.0))?; }
                                        // 621: global.rope -= 1;
                                        g.at("oPlayer1/step#2", 621);
                                        g.gset(gv::rope, lenient(Value::sub, g.gget(gv::rope)?, &r(1.0)))?;
                                        // 622: whoaTimer = whoaTimerMax;
                                        g.at("oPlayer1/step#2", 622);
                                        g.set(cx.this, v::whoaTimer, g.get(cx.this, v::whoaTimerMax)?)?;
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // 628: if (global.bombs > 0)
                g.at("oPlayer1/step#2", 628);
                if g.gget(gv::bombs)?.gml_gt(&r(0.0))? {
                    // 630: holdItem = instance_create(x, y, oBomb);
                    g.at("oPlayer1/step#2", 630);
                    {
                        let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oBomb]; b::instance_create(g, cx, a)? };
                        g.set(cx.this, v::holdItem, x)?;
                    }
                    // 631: if (global.hasStickyBombs)
                    g.at("oPlayer1/step#2", 631);
                    if g.gget(gv::hasStickyBombs)?.is_truthy() {
                        // 631: holdItem.sticky = true;
                        g.at("oPlayer1/step#2", 631);
                        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::sticky, r(1.0))?; }
                    }
                    // 632: holdItem.held = true;
                    g.at("oPlayer1/step#2", 632);
                    { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(1.0))?; }
                    // 633: global.bombs -= 1;
                    g.at("oPlayer1/step#2", 633);
                    g.gset(gv::bombs, lenient(Value::sub, g.gget(gv::bombs)?, &r(1.0)))?;
                    // 634: whoaTimer = whoaTimerMax;
                    g.at("oPlayer1/step#2", 634);
                    g.set(cx.this, v::whoaTimer, g.get(cx.this, v::whoaTimerMax)?)?;
                } else {
                    // 636: if (global.rope > 0)
                    g.at("oPlayer1/step#2", 636);
                    if g.gget(gv::rope)?.gml_gt(&r(0.0))? {
                        // 638: holdItem = instance_create(x, y, oRopeThrow);
                        g.at("oPlayer1/step#2", 638);
                        {
                            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oRopeThrow]; b::instance_create(g, cx, a)? };
                            g.set(cx.this, v::holdItem, x)?;
                        }
                        // 639: holdItem.held = true;
                        g.at("oPlayer1/step#2", 639);
                        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(1.0))?; }
                        // 640: global.rope -= 1;
                        g.at("oPlayer1/step#2", 640);
                        g.gset(gv::rope, lenient(Value::sub, g.gget(gv::rope)?, &r(1.0)))?;
                        // 641: whoaTimer = whoaTimerMax;
                        g.at("oPlayer1/step#2", 641);
                        g.set(cx.this, v::whoaTimer, g.get(cx.this, v::whoaTimerMax)?)?;
                    }
                }
            }
        } else {
            // 645: if (inGame && kRopePressed && global.rope > 0 && !whipping)
            g.at("oPlayer1/step#2", 645);
            if and(and(and(g.get(cx.this, v::inGame)?.is_truthy(), g.get(cx.this, v::kRopePressed)?.is_truthy()), g.gget(gv::rope)?.gml_gt(&r(0.0))?), g.get(cx.this, v::whipping)?.not()?.is_truthy()) {
                // 647: if (!kDown && colTop)
                g.at("oPlayer1/step#2", 647);
                if and(g.get(cx.this, v::kDown)?.not()?.is_truthy(), g.get(cx.this, v::colTop)?.is_truthy()) {
                } else {
                    // 653: if (kDown)
                    g.at("oPlayer1/step#2", 653);
                    if g.get(cx.this, v::kDown)?.is_truthy() {
                        // 655: if (facing == LEFT)
                        g.at("oPlayer1/step#2", 655);
                        if g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)? {
                            // 657: obj = instance_create(x - 16, y, oRopeThrow);
                            g.at("oPlayer1/step#2", 657);
                            {
                                let x = { let a = [g.get(cx.this, v::x)?.sub(&r(16.0))?, g.get(cx.this, v::y)?, obj::oRopeThrow]; b::instance_create(g, cx, a)? };
                                g.set(cx.this, v::obj, x)?;
                            }
                        } else {
                            // 661: obj = instance_create(x + 16, y, oRopeThrow);
                            g.at("oPlayer1/step#2", 661);
                            {
                                let x = { let a = [g.get(cx.this, v::x)?.add(&r(16.0))?, g.get(cx.this, v::y)?, obj::oRopeThrow]; b::instance_create(g, cx, a)? };
                                g.set(cx.this, v::obj, x)?;
                            }
                        }
                        // 664: with (obj)
                        g.at("oPlayer1/step#2", 664);
                        {
                            let t = g.get(cx.this, v::obj)?;
                            let mut it52 = g.with_iter(cx, &t)?;
                            let outer54 = (cx.this, cx.other);
                            'with23: while let Some(h53) = it52.next(g) {
                                (cx.this, cx.other) = (h53, outer54.0);
                                // 666: t = true;
                                g.at("oPlayer1/step#2", 666);
                                g.set(cx.this, v::t, r(1.0))?;
                                // 667: move_snap(16, 1);
                                g.at("oPlayer1/step#2", 667);
                                b::move_snap(g, cx, [r(16.0), r(1.0)])?;
                                // 668: if (oPlayer1.x < x)
                                g.at("oPlayer1/step#2", 668);
                                if g.get_on(cx, &obj::oPlayer1, v::x)?.gml_lt(&g.get(cx.this, v::x)?)? {
                                    // 670: if (!collision_point(oPlayer1.x + 8, oPlayer1.y, oSolid, 0, 0))
                                    g.at("oPlayer1/step#2", 670);
                                    if ({ let a = [g.get_on(cx, &obj::oPlayer1, v::x)?.add(&r(8.0))?, g.get_on(cx, &obj::oPlayer1, v::y)?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy() {
                                        // 672: if (!collision_rectangle(x - 8, y, x - 7, y + 16, oSolid, 0, 0))
                                        g.at("oPlayer1/step#2", 672);
                                        if ({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?, g.get(cx.this, v::x)?.sub(&r(7.0))?, g.get(cx.this, v::y)?.add(&r(16.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).not()?.is_truthy() {
                                            // 673: x -= 8;
                                            g.at("oPlayer1/step#2", 673);
                                            g.set(cx.this, v::x, g.get(cx.this, v::x)?.sub(&r(8.0))?)?;
                                        } else {
                                            // 674: if (!collision_rectangle(x + 7, y, x + 8, y + 16, oSolid, 0, 0))
                                            g.at("oPlayer1/step#2", 674);
                                            if ({ let a = [g.get(cx.this, v::x)?.add(&r(7.0))?, g.get(cx.this, v::y)?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(16.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).not()?.is_truthy() {
                                                // 675: x += 8;
                                                g.at("oPlayer1/step#2", 675);
                                                g.set(cx.this, v::x, g.get(cx.this, v::x)?.add(&r(8.0))?)?;
                                            } else {
                                                // 676: t = false;
                                                g.at("oPlayer1/step#2", 676);
                                                g.set(cx.this, v::t, r(0.0))?;
                                            }
                                        }
                                    } else {
                                        // 678: t = false;
                                        g.at("oPlayer1/step#2", 678);
                                        g.set(cx.this, v::t, r(0.0))?;
                                    }
                                } else {
                                    // 680: if (!collision_point(oPlayer1.x - 8, oPlayer1.y, oSolid, 0, 0))
                                    g.at("oPlayer1/step#2", 680);
                                    if ({ let a = [g.get_on(cx, &obj::oPlayer1, v::x)?.sub(&r(8.0))?, g.get_on(cx, &obj::oPlayer1, v::y)?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy() {
                                        // 682: if (!collision_rectangle(x + 7, y, x + 8, y + 16, oSolid, 0, 0))
                                        g.at("oPlayer1/step#2", 682);
                                        if ({ let a = [g.get(cx.this, v::x)?.add(&r(7.0))?, g.get(cx.this, v::y)?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(16.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).not()?.is_truthy() {
                                            // 683: x += 8;
                                            g.at("oPlayer1/step#2", 683);
                                            g.set(cx.this, v::x, g.get(cx.this, v::x)?.add(&r(8.0))?)?;
                                        } else {
                                            // 684: if (!collision_rectangle(x - 8, y, x - 7, y + 16, oSolid, 0, 0))
                                            g.at("oPlayer1/step#2", 684);
                                            if ({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?, g.get(cx.this, v::x)?.sub(&r(7.0))?, g.get(cx.this, v::y)?.add(&r(16.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).not()?.is_truthy() {
                                                // 685: x -= 8;
                                                g.at("oPlayer1/step#2", 685);
                                                g.set(cx.this, v::x, g.get(cx.this, v::x)?.sub(&r(8.0))?)?;
                                            } else {
                                                // 686: t = false;
                                                g.at("oPlayer1/step#2", 686);
                                                g.set(cx.this, v::t, r(0.0))?;
                                            }
                                        }
                                    } else {
                                        // 688: t = false;
                                        g.at("oPlayer1/step#2", 688);
                                        g.set(cx.this, v::t, r(0.0))?;
                                    }
                                }
                                // 690: if (!t)
                                g.at("oPlayer1/step#2", 690);
                                if g.get(cx.this, v::t)?.not()?.is_truthy() {
                                    // 705: instance_destroy();
                                    g.at("oPlayer1/step#2", 705);
                                    b::instance_destroy(g, cx, [])?;
                                } else {
                                    // 709: instance_create(x, y, oRopeTop);
                                    g.at("oPlayer1/step#2", 709);
                                    { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oRopeTop]; b::instance_create(g, cx, a)? };
                                    // 710: armed = false;
                                    g.at("oPlayer1/step#2", 710);
                                    g.set(cx.this, v::armed, r(0.0))?;
                                    // 711: falling = true;
                                    g.at("oPlayer1/step#2", 711);
                                    g.set(cx.this, v::falling, r(1.0))?;
                                    // 712: xVel = 0;
                                    g.at("oPlayer1/step#2", 712);
                                    g.set(cx.this, v::xVel, r(0.0))?;
                                    // 713: yVel = 0;
                                    g.at("oPlayer1/step#2", 713);
                                    g.set(cx.this, v::yVel, r(0.0))?;
                                    // 714: global.rope -= 1;
                                    g.at("oPlayer1/step#2", 714);
                                    g.gset(gv::rope, lenient(Value::sub, g.gget(gv::rope)?, &r(1.0)))?;
                                    // 715: playSound(global.sndThrow);
                                    g.at("oPlayer1/step#2", 715);
                                    { let a = [g.gget(gv::sndThrow)?]; scr::playSound(g, cx, &a)? };
                                }
                            }
                            (cx.this, cx.other) = outer54;
                        }
                    } else {
                        // 721: obj = instance_create(x, y, oRopeThrow);
                        g.at("oPlayer1/step#2", 721);
                        {
                            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oRopeThrow]; b::instance_create(g, cx, a)? };
                            g.set(cx.this, v::obj, x)?;
                        }
                        // 722: obj.armed = true;
                        g.at("oPlayer1/step#2", 722);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::armed, r(1.0))?; }
                        // 723: obj.px = x;
                        g.at("oPlayer1/step#2", 723);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::px, g.get(cx.this, v::x)?)?; }
                        // 724: obj.py = y;
                        g.at("oPlayer1/step#2", 724);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::py, g.get(cx.this, v::y)?)?; }
                        // 725: obj.xVel = 0;
                        g.at("oPlayer1/step#2", 725);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::xVel, r(0.0))?; }
                        // 726: obj.yVel = -12;
                        g.at("oPlayer1/step#2", 726);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::yVel, r(-12.0))?; }
                        // 727: global.rope -= 1;
                        g.at("oPlayer1/step#2", 727);
                        g.gset(gv::rope, lenient(Value::sub, g.gget(gv::rope)?, &r(1.0)))?;
                        // 728: playSound(global.sndThrow);
                        g.at("oPlayer1/step#2", 728);
                        { let a = [g.gget(gv::sndThrow)?]; scr::playSound(g, cx, &a)? };
                    }
                }
            } else {
                // 732: if (inGame && kBombPressed && global.bombs > 0 && !whipping && bowArmed)
                g.at("oPlayer1/step#2", 732);
                if and(and(and(and(g.get(cx.this, v::inGame)?.is_truthy(), g.get(cx.this, v::kBombPressed)?.is_truthy()), g.gget(gv::bombs)?.gml_gt(&r(0.0))?), g.get(cx.this, v::whipping)?.not()?.is_truthy()), g.get(cx.this, v::bowArmed)?.is_truthy()) {
                    // 734: holdArrow = ARROW_BOMB;
                    g.at("oPlayer1/step#2", 734);
                    g.set(cx.this, v::holdArrow, g.get(cx.this, v::ARROW_BOMB)?)?;
                    // 735: alarm[11] = 1;
                    g.at("oPlayer1/step#2", 735);
                    g.set_a(cx.this, v::alarm, 0, 11, r(1.0))?;
                } else {
                    // 737: if (inGame && kBombPressed && global.bombs > 0 && !whipping)
                    g.at("oPlayer1/step#2", 737);
                    if and(and(and(g.get(cx.this, v::inGame)?.is_truthy(), g.get(cx.this, v::kBombPressed)?.is_truthy()), g.gget(gv::bombs)?.gml_gt(&r(0.0))?), g.get(cx.this, v::whipping)?.not()?.is_truthy()) {
                        // 739: obj = instance_create(x, y, oBomb);
                        g.at("oPlayer1/step#2", 739);
                        {
                            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oBomb]; b::instance_create(g, cx, a)? };
                            g.set(cx.this, v::obj, x)?;
                        }
                        // 740: if (global.hasStickyBombs)
                        g.at("oPlayer1/step#2", 740);
                        if g.gget(gv::hasStickyBombs)?.is_truthy() {
                            // 740: obj.sticky = true;
                            g.at("oPlayer1/step#2", 740);
                            { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sticky, r(1.0))?; }
                        }
                        // 741: obj.sprite_index = sBombArmed;
                        g.at("oPlayer1/step#2", 741);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sBombArmed)?; }
                        // 742: obj.armed = true;
                        g.at("oPlayer1/step#2", 742);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::armed, r(1.0))?; }
                        // 743: with (obj)
                        g.at("oPlayer1/step#2", 743);
                        {
                            let t = g.get(cx.this, v::obj)?;
                            let mut it55 = g.with_iter(cx, &t)?;
                            let outer57 = (cx.this, cx.other);
                            'with24: while let Some(h56) = it55.next(g) {
                                (cx.this, cx.other) = (h56, outer57.0);
                                // 745: alarm[0] = 80;
                                g.at("oPlayer1/step#2", 745);
                                g.set_a(cx.this, v::alarm, 0, 0, r(80.0))?;
                                // 746: image_speed = 0.2;
                                g.at("oPlayer1/step#2", 746);
                                g.set(cx.this, v::image_speed, r(0.2))?;
                            }
                            (cx.this, cx.other) = outer57;
                        }
                        // 748: obj.safe = true;
                        g.at("oPlayer1/step#2", 748);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::safe, r(1.0))?; }
                        // 749: obj.alarm[2] = 10;
                        g.at("oPlayer1/step#2", 749);
                        { let t = g.get(cx.this, v::obj)?; g.set_on_a(cx, &t, v::alarm, 0, 2, r(10.0))?; }
                        // 751: if (facing == LEFT)
                        g.at("oPlayer1/step#2", 751);
                        if g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)? {
                            // 753: obj.xVel = -8 + xVel;
                            g.at("oPlayer1/step#2", 753);
                            {
                                let t = g.get(cx.this, v::obj)?;
                                g.set_on(cx, &t, v::xVel, r(-8.0).add(&g.get(cx.this, v::xVel)?)?)?;
                            }
                        } else {
                            // 755: if (facing == RIGHT)
                            g.at("oPlayer1/step#2", 755);
                            if g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)? {
                                // 757: obj.xVel = 8 + xVel;
                                g.at("oPlayer1/step#2", 757);
                                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::xVel, r(8.0).add(&g.get(cx.this, v::xVel)?)?)?; }
                            }
                        }
                        // 759: obj.yVel = -3;
                        g.at("oPlayer1/step#2", 759);
                        { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::yVel, r(-3.0))?; }
                        // 761: if (kUp)
                        g.at("oPlayer1/step#2", 761);
                        if g.get(cx.this, v::kUp)?.is_truthy() {
                            // 763: obj.yVel = -9;
                            g.at("oPlayer1/step#2", 763);
                            { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::yVel, r(-9.0))?; }
                        }
                        // 766: if (kDown)
                        g.at("oPlayer1/step#2", 766);
                        if g.get(cx.this, v::kDown)?.is_truthy() {
                            // 768: if (platformCharacterIs(ON_GROUND))
                            g.at("oPlayer1/step#2", 768);
                            if ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy() {
                                // 768: obj.xVel *= 0.1;
                                g.at("oPlayer1/step#2", 768);
                                {
                                    let t = g.get(cx.this, v::obj)?;
                                    g.set_on(cx, &t, v::xVel, lenient(Value::mul, { let t2 = g.get(cx.this, v::obj)?; g.get_on(cx, &t2, v::xVel)? }, &r(0.1)))?;
                                }
                            }
                            // 769: obj.yVel = 3;
                            g.at("oPlayer1/step#2", 769);
                            { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::yVel, r(3.0))?; }
                        }
                        // 772: global.bombs -= 1;
                        g.at("oPlayer1/step#2", 772);
                        g.gset(gv::bombs, lenient(Value::sub, g.gget(gv::bombs)?, &r(1.0)))?;
                        // 773: playSound(global.sndThrow);
                        g.at("oPlayer1/step#2", 773);
                        { let a = [g.gget(gv::sndThrow)?]; scr::playSound(g, cx, &a)? };
                    } else {
                        // 775: if (holdItem == 0)
                        g.at("oPlayer1/step#2", 775);
                        if g.get(cx.this, v::holdItem)?.gml_eq(&r(0.0))? {
                            // 777: if (kAttackPressed && state != DUCKING && state != DUCKTOHANG && !whipping && sprite_index != sPExit
                            g.at("oPlayer1/step#2", 777);
                            if and(and(and(and(and(g.get(cx.this, v::kAttackPressed)?.is_truthy(), g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::DUCKING)?)?), g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::DUCKTOHANG)?)?), g.get(cx.this, v::whipping)?.not()?.is_truthy()), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sPExit)?), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sDamselExit)?) {
                                // 780: image_speed = 0.6;
                                g.at("oPlayer1/step#2", 780);
                                g.set(cx.this, v::image_speed, r(0.6000000000000001))?;
                                // 781: if (global.isTunnelMan)
                                g.at("oPlayer1/step#2", 781);
                                if g.gget(gv::isTunnelMan)?.is_truthy() {
                                    // 783: if (platformCharacterIs(ON_GROUND))
                                    g.at("oPlayer1/step#2", 783);
                                    if ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy() {
                                        // 785: sprite_index = sTunnelAttackL;
                                        g.at("oPlayer1/step#2", 785);
                                        g.set(cx.this, v::sprite_index, spr::sTunnelAttackL)?;
                                        // 786: image_index = 0;
                                        g.at("oPlayer1/step#2", 786);
                                        g.set(cx.this, v::image_index, r(0.0))?;
                                        // 787: whipping = true;
                                        g.at("oPlayer1/step#2", 787);
                                        g.set(cx.this, v::whipping, r(1.0))?;
                                    }
                                } else {
                                    // 790: if (global.isDamsel)
                                    g.at("oPlayer1/step#2", 790);
                                    if g.gget(gv::isDamsel)?.is_truthy() {
                                        // 792: sprite_index = sDamselAttackL;
                                        g.at("oPlayer1/step#2", 792);
                                        g.set(cx.this, v::sprite_index, spr::sDamselAttackL)?;
                                        // 793: image_index = 0;
                                        g.at("oPlayer1/step#2", 793);
                                        g.set(cx.this, v::image_index, r(0.0))?;
                                        // 794: whipping = true;
                                        g.at("oPlayer1/step#2", 794);
                                        g.set(cx.this, v::whipping, r(1.0))?;
                                    } else {
                                        // 798: sprite_index = sAttackLeft;
                                        g.at("oPlayer1/step#2", 798);
                                        g.set(cx.this, v::sprite_index, spr::sAttackLeft)?;
                                        // 799: image_index = 0;
                                        g.at("oPlayer1/step#2", 799);
                                        g.set(cx.this, v::image_index, r(0.0))?;
                                        // 800: whipping = true;
                                        g.at("oPlayer1/step#2", 800);
                                        g.set(cx.this, v::whipping, r(1.0))?;
                                    }
                                }
                            } else {
                                // 803: if (kAttackPressed && kDown)
                                g.at("oPlayer1/step#2", 803);
                                if and(g.get(cx.this, v::kAttackPressed)?.is_truthy(), g.get(cx.this, v::kDown)?.is_truthy()) {
                                    // 806: if (collision_rectangle(x - 8, y, x + 8, y + 8, oItem, 0, 0))
                                    g.at("oPlayer1/step#2", 806);
                                    if ({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oItem, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy() {
                                        // 808: obj = instance_nearest(x, y, oItem);
                                        g.at("oPlayer1/step#2", 808);
                                        {
                                            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oItem]; b::instance_nearest(g, cx, a)? };
                                            g.set(cx.this, v::obj, x)?;
                                        }
                                        // 809: if (obj.canPickUp && !collision_point(obj.x, obj.y, oSolid, 0, 0))
                                        g.at("oPlayer1/step#2", 809);
                                        if and(g.get_on(cx, &g.get(cx.this, v::obj)?, v::canPickUp)?.is_truthy(), ({ let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::obj)?, v::y)?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()) {
                                            // 811: holdItem = obj;
                                            g.at("oPlayer1/step#2", 811);
                                            g.set(cx.this, v::holdItem, g.get(cx.this, v::obj)?)?;
                                            // 812: holdItem.held = true;
                                            g.at("oPlayer1/step#2", 812);
                                            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(1.0))?; }
                                            // 813: whoaTimer = whoaTimerMax;
                                            g.at("oPlayer1/step#2", 813);
                                            g.set(cx.this, v::whoaTimer, g.get(cx.this, v::whoaTimerMax)?)?;
                                            // 814: pickupItemType = holdItem.type;
                                            g.at("oPlayer1/step#2", 814);
                                            g.set(cx.this, v::pickupItemType, g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?)?;
                                            // 816: if (holdItem.type == "Bow" && holdItem.new)
                                            g.at("oPlayer1/step#2", 816);
                                            if and(g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Bow"))?, g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::new)?.is_truthy()) {
                                                // 818: holdItem.new = false;
                                                g.at("oPlayer1/step#2", 818);
                                                { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::new, r(0.0))?; }
                                                // 819: global.arrows += 6;
                                                g.at("oPlayer1/step#2", 819);
                                                g.gset(gv::arrows, lenient(Value::add, g.gget(gv::arrows)?, &r(6.0)))?;
                                            }
                                            // 822: if (holdItem.type == "Gold Idol" && holdItem.trigger && !isRoom("rLoadLevel"))
                                            g.at("oPlayer1/step#2", 822);
                                            if and(and(g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Gold Idol"))?, g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::trigger)?.is_truthy()), scr::isRoom(g, cx, &[s!("rLoadLevel")])?.not()?.is_truthy()) {
                                                // 824: global.idolsGrabbed += 1;
                                                g.at("oPlayer1/step#2", 824);
                                                g.gset(gv::idolsGrabbed, lenient(Value::add, g.gget(gv::idolsGrabbed)?, &r(1.0)))?;
                                                // 825: if (global.levelType == 0)
                                                g.at("oPlayer1/step#2", 825);
                                                if g.gget(gv::levelType)?.gml_eq(&r(0.0))? {
                                                    // 827: trap = instance_nearest(x, y - 64, oGiantTikiHead);
                                                    g.at("oPlayer1/step#2", 827);
                                                    {
                                                        let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(64.0))?, obj::oGiantTikiHead]; b::instance_nearest(g, cx, a)? };
                                                        g.set(cx.this, v::trap, x)?;
                                                    }
                                                    // 828: with (trap)
                                                    g.at("oPlayer1/step#2", 828);
                                                    {
                                                        let t = g.get(cx.this, v::trap)?;
                                                        let mut it58 = g.with_iter(cx, &t)?;
                                                        let outer60 = (cx.this, cx.other);
                                                        'with25: while let Some(h59) = it58.next(g) {
                                                            (cx.this, cx.other) = (h59, outer60.0);
                                                            // 828: alarm[0] = 100;
                                                            g.at("oPlayer1/step#2", 828);
                                                            g.set_a(cx.this, v::alarm, 0, 0, r(100.0))?;
                                                        }
                                                        (cx.this, cx.other) = outer60;
                                                    }
                                                    // 829: scrShake(100);
                                                    g.at("oPlayer1/step#2", 829);
                                                    scr::scrShake(g, cx, &[r(100.0)])?;
                                                    // 830: holdItem.trigger = false;
                                                    g.at("oPlayer1/step#2", 830);
                                                    { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::trigger, r(0.0))?; }
                                                } else {
                                                    // 832: if (global.levelType == 1)
                                                    g.at("oPlayer1/step#2", 832);
                                                    if g.gget(gv::levelType)?.gml_eq(&r(1.0))? {
                                                        // 834: if (global.cemetary && !global.ghostExists)
                                                        g.at("oPlayer1/step#2", 834);
                                                        if and(g.gget(gv::cemetary)?.is_truthy(), g.gget(gv::ghostExists)?.not()?.is_truthy()) {
                                                            // 836: if (oPlayer1.x > room_width / 2)
                                                            g.at("oPlayer1/step#2", 836);
                                                            if g.get_on(cx, &obj::oPlayer1, v::x)?.gml_gt(&g.bget(v::room_width, 0)?.div(&r(2.0))?)? {
                                                                // 836: instance_create(view_xview[0] + view_wview[0] + 8, view_yview[0] + floor(view_hview[0] / 2), oGhost)
                                                                g.at("oPlayer1/step#2", 836);
                                                                { let a = [g.bget(v::view_xview, 0)?.add(&g.bget(v::view_wview, 0)?)?.add(&r(8.0))?, g.bget(v::view_yview, 0)?.add(&{ let a = [g.bget(v::view_hview, 0)?.div(&r(2.0))?]; b::floor(g, cx, a)? })?, obj::oGhost]; b::instance_create(g, cx, a)? };
                                                            } else {
                                                                // 837: instance_create(view_xview[0] - 32, view_yview[0] + floor(view_hview[0] / 2), oGhost);
                                                                g.at("oPlayer1/step#2", 837);
                                                                { let a = [g.bget(v::view_xview, 0)?.sub(&r(32.0))?, g.bget(v::view_yview, 0)?.add(&{ let a = [g.bget(v::view_hview, 0)?.div(&r(2.0))?]; b::floor(g, cx, a)? })?, obj::oGhost]; b::instance_create(g, cx, a)? };
                                                            }
                                                            // 838: global.ghostExists = true;
                                                            g.at("oPlayer1/step#2", 838);
                                                            g.gset(gv::ghostExists, r(1.0))?;
                                                        }
                                                        // 840: with (oTrapBlock)
                                                        g.at("oPlayer1/step#2", 840);
                                                        {
                                                            let t = obj::oTrapBlock;
                                                            let mut it61 = g.with_iter(cx, &t)?;
                                                            let outer63 = (cx.this, cx.other);
                                                            'with26: while let Some(h62) = it61.next(g) {
                                                                (cx.this, cx.other) = (h62, outer63.0);
                                                                // 842: dist = distance_to_object(oCharacter);
                                                                g.at("oPlayer1/step#2", 842);
                                                                { let x = b::distance_to_object(g, cx, [obj::oCharacter])?; g.set(cx.this, v::dist, x)?; }
                                                                // 843: if (dist < 90)
                                                                g.at("oPlayer1/step#2", 843);
                                                                if g.get(cx.this, v::dist)?.gml_lt(&r(90.0))? {
                                                                    // 845: dying = true;
                                                                    g.at("oPlayer1/step#2", 845);
                                                                    g.set(cx.this, v::dying, r(1.0))?;
                                                                }
                                                            }
                                                            (cx.this, cx.other) = outer63;
                                                        }
                                                    } else {
                                                        // 850: if (global.levelType == 3)
                                                        g.at("oPlayer1/step#2", 850);
                                                        if g.gget(gv::levelType)?.gml_eq(&r(3.0))? {
                                                            // 852: if (instance_exists(oCeilingTrap))
                                                            g.at("oPlayer1/step#2", 852);
                                                            if b::instance_exists(g, cx, [obj::oCeilingTrap])?.is_truthy() {
                                                                // 854: with (oCeilingTrap)
                                                                g.at("oPlayer1/step#2", 854);
                                                                {
                                                                    let t = obj::oCeilingTrap;
                                                                    let mut it64 = g.with_iter(cx, &t)?;
                                                                    let outer66 = (cx.this, cx.other);
                                                                    'with27: while let Some(h65) = it64.next(g) {
                                                                        (cx.this, cx.other) = (h65, outer66.0);
                                                                        // 856: status = 1;
                                                                        g.at("oPlayer1/step#2", 856);
                                                                        g.set(cx.this, v::status, r(1.0))?;
                                                                        // 857: yVel = 0.5;
                                                                        g.at("oPlayer1/step#2", 857);
                                                                        g.set(cx.this, v::yVel, r(0.5))?;
                                                                    }
                                                                    (cx.this, cx.other) = outer66;
                                                                }
                                                                // 859: scrShake(20);
                                                                g.at("oPlayer1/step#2", 859);
                                                                scr::scrShake(g, cx, &[r(20.0)])?;
                                                                // 860: trap = instance_nearest(x - 64, y - 64, oDoor);
                                                                g.at("oPlayer1/step#2", 860);
                                                                {
                                                                    let x = { let a = [g.get(cx.this, v::x)?.sub(&r(64.0))?, g.get(cx.this, v::y)?.sub(&r(64.0))?, obj::oDoor]; b::instance_nearest(g, cx, a)? };
                                                                    g.set(cx.this, v::trap, x)?;
                                                                }
                                                                // 861: trap.status = 1;
                                                                g.at("oPlayer1/step#2", 861);
                                                                { let t = g.get(cx.this, v::trap)?; g.set_on(cx, &t, v::status, r(1.0))?; }
                                                                // 862: trap.yVel = 1;
                                                                g.at("oPlayer1/step#2", 862);
                                                                { let t = g.get(cx.this, v::trap)?; g.set_on(cx, &t, v::yVel, r(1.0))?; }
                                                                // 863: trap = instance_nearest(x + 64, y - 64, oDoor);
                                                                g.at("oPlayer1/step#2", 863);
                                                                {
                                                                    let x = { let a = [g.get(cx.this, v::x)?.add(&r(64.0))?, g.get(cx.this, v::y)?.sub(&r(64.0))?, obj::oDoor]; b::instance_nearest(g, cx, a)? };
                                                                    g.set(cx.this, v::trap, x)?;
                                                                }
                                                                // 864: trap.status = 1;
                                                                g.at("oPlayer1/step#2", 864);
                                                                { let t = g.get(cx.this, v::trap)?; g.set_on(cx, &t, v::status, r(1.0))?; }
                                                                // 865: trap.yVel = 1;
                                                                g.at("oPlayer1/step#2", 865);
                                                                { let t = g.get(cx.this, v::trap)?; g.set_on(cx, &t, v::yVel, r(1.0))?; }
                                                            } else {
                                                                // 869: with (oTrapBlock)
                                                                g.at("oPlayer1/step#2", 869);
                                                                {
                                                                    let t = obj::oTrapBlock;
                                                                    let mut it67 = g.with_iter(cx, &t)?;
                                                                    let outer69 = (cx.this, cx.other);
                                                                    'with28: while let Some(h68) = it67.next(g) {
                                                                        (cx.this, cx.other) = (h68, outer69.0);
                                                                        // 871: dist = distance_to_object(oCharacter);
                                                                        g.at("oPlayer1/step#2", 871);
                                                                        { let x = b::distance_to_object(g, cx, [obj::oCharacter])?; g.set(cx.this, v::dist, x)?; }
                                                                        // 872: if (dist < 90)
                                                                        g.at("oPlayer1/step#2", 872);
                                                                        if g.get(cx.this, v::dist)?.gml_lt(&r(90.0))? {
                                                                            // 874: instance_destroy();
                                                                            g.at("oPlayer1/step#2", 874);
                                                                            b::instance_destroy(g, cx, [])?;
                                                                        }
                                                                        // 876: playSound(global.sndThump);
                                                                        g.at("oPlayer1/step#2", 876);
                                                                        { let a = [g.gget(gv::sndThump)?]; scr::playSound(g, cx, &a)? };
                                                                        // 877: scrShake(10);
                                                                        g.at("oPlayer1/step#2", 877);
                                                                        scr::scrShake(g, cx, &[r(10.0)])?;
                                                                    }
                                                                    (cx.this, cx.other) = outer69;
                                                                }
                                                            }
                                                            // 881: holdItem.trigger = false;
                                                            g.at("oPlayer1/step#2", 881);
                                                            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::trigger, r(0.0))?; }
                                                        }
                                                    }
                                                }
                                            } else {
                                                // 884: if (holdItem.type == "Damsel")
                                                g.at("oPlayer1/step#2", 884);
                                                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Damsel"))? {
                                                    // 886: if (holdItem.status == 4)
                                                    g.at("oPlayer1/step#2", 886);
                                                    if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::status)?.gml_eq(&r(4.0))? {
                                                        // 888: holdItem = 0;
                                                        g.at("oPlayer1/step#2", 888);
                                                        g.set(cx.this, v::holdItem, r(0.0))?;
                                                        // 889: holdItem.held = false;
                                                        g.at("oPlayer1/step#2", 889);
                                                        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
                                                    } else {
                                                        // 893: if (global.isDamsel)
                                                        g.at("oPlayer1/step#2", 893);
                                                        if g.gget(gv::isDamsel)?.is_truthy() {
                                                            // 893: holdItem.sprite_index = sDieLBounce;
                                                            g.at("oPlayer1/step#2", 893);
                                                            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::sprite_index, spr::sDieLBounce)?; }
                                                        } else {
                                                            // 894: holdItem.sprite_index = sDamselHoldL;
                                                            g.at("oPlayer1/step#2", 894);
                                                            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::sprite_index, spr::sDamselHoldL)?; }
                                                        }
                                                    }
                                                } else {
                                                    // 897: if (holdItem.cost == 0)
                                                    g.at("oPlayer1/step#2", 897);
                                                    if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::cost)?.gml_eq(&r(0.0))? {
                                                        // 897: scrStealItem();
                                                        g.at("oPlayer1/step#2", 897);
                                                        scr::scrStealItem(g, cx, &[])?;
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // 900: if (collision_rectangle(x - 8, y, x + 8, y + 8, oEnemy, 0, 0))
                                        g.at("oPlayer1/step#2", 900);
                                        if ({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oEnemy, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy() {
                                            // 902: obj = instance_nearest(x, y, oEnemy);
                                            g.at("oPlayer1/step#2", 902);
                                            {
                                                let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oEnemy]; b::instance_nearest(g, cx, a)? };
                                                g.set(cx.this, v::obj, x)?;
                                            }
                                            // 903: if (obj.status >= 98 && obj.canPickUp)
                                            g.at("oPlayer1/step#2", 903);
                                            if and(g.get_on(cx, &g.get(cx.this, v::obj)?, v::status)?.gml_ge(&r(98.0))?, g.get_on(cx, &g.get(cx.this, v::obj)?, v::canPickUp)?.is_truthy()) {
                                                // 905: holdItem = obj;
                                                g.at("oPlayer1/step#2", 905);
                                                g.set(cx.this, v::holdItem, g.get(cx.this, v::obj)?)?;
                                                // 906: holdItem.held = true;
                                                g.at("oPlayer1/step#2", 906);
                                                { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(1.0))?; }
                                                // 907: whoaTimer = whoaTimerMax;
                                                g.at("oPlayer1/step#2", 907);
                                                g.set(cx.this, v::whoaTimer, g.get(cx.this, v::whoaTimerMax)?)?;
                                                // 908: pickupItemType = holdItem.type;
                                                g.at("oPlayer1/step#2", 908);
                                                g.set(cx.this, v::pickupItemType, g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?)?;
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // 913: if (kAttackPressed)
                            g.at("oPlayer1/step#2", 913);
                            if g.get(cx.this, v::kAttackPressed)?.is_truthy() {
                                // 915: if (holdItem)
                                g.at("oPlayer1/step#2", 915);
                                if g.get(cx.this, v::holdItem)?.is_truthy() {
                                    // 917: scrUseItem();
                                    g.at("oPlayer1/step#2", 917);
                                    scr::scrUseItem(g, cx, &[])?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // 959: if (isLevel() && active && kPayPressed && !dead && !stunned)
    g.at("oPlayer1/step#2", 959);
    if and(and(and(and(scr::isLevel(g, cx, &[])?.is_truthy(), g.get(cx.this, v::active)?.is_truthy()), g.get(cx.this, v::kPayPressed)?.is_truthy()), g.get(cx.this, v::dead)?.not()?.is_truthy()), g.get(cx.this, v::stunned)?.not()?.is_truthy()) {
        // 961: if (isInShop(x, y) && instance_exists(oShopkeeper))
        g.at("oPlayer1/step#2", 961);
        if and(({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?]; scr::isInShop(g, cx, &a)? }).is_truthy(), b::instance_exists(g, cx, [obj::oShopkeeper])?.is_truthy()) {
            // 963: n = 0;
            g.at("oPlayer1/step#2", 963);
            g.set(cx.this, v::n, r(0.0))?;
            // 964: if (holdItem)
            g.at("oPlayer1/step#2", 964);
            if g.get(cx.this, v::holdItem)?.is_truthy() {
                // 966: if (holdItem.cost <= 0)
                g.at("oPlayer1/step#2", 966);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::cost)?.gml_le(&r(0.0))? {
                } else {
                    // 970: if (holdItem.cost > global.money)
                    g.at("oPlayer1/step#2", 970);
                    if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::cost)?.gml_gt(&g.gget(gv::money)?)? {
                        // 972: global.message = "YOU HAVEN'T GOT ENOUGH MONEY!";
                        g.at("oPlayer1/step#2", 972);
                        g.gset(gv::message, s!("YOU HAVEN'T GOT ENOUGH MONEY!"))?;
                        // 973: global.message2 = "";
                        g.at("oPlayer1/step#2", 973);
                        g.gset(gv::message2, s!(""))?;
                        // 974: global.messageTimer = 80;
                        g.at("oPlayer1/step#2", 974);
                        g.gset(gv::messageTimer, r(80.0))?;
                        // 975: with (holdItem)
                        g.at("oPlayer1/step#2", 975);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            let mut it70 = g.with_iter(cx, &t)?;
                            let outer72 = (cx.this, cx.other);
                            'with29: while let Some(h71) = it70.next(g) {
                                (cx.this, cx.other) = (h71, outer72.0);
                                // 975: held = false;
                                g.at("oPlayer1/step#2", 975);
                                g.set(cx.this, v::held, r(0.0))?;
                            }
                            (cx.this, cx.other) = outer72;
                        }
                        // 976: holdItem = 0;
                        g.at("oPlayer1/step#2", 976);
                        g.set(cx.this, v::holdItem, r(0.0))?;
                        // 977: pickupItemType = "";
                        g.at("oPlayer1/step#2", 977);
                        g.set(cx.this, v::pickupItemType, s!(""))?;
                        // 978: n = 1;
                        g.at("oPlayer1/step#2", 978);
                        g.set(cx.this, v::n, r(1.0))?;
                    } else {
                        // 982: if (isRealLevel())
                        g.at("oPlayer1/step#2", 982);
                        if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                            // 982: global.itemsBought += 1;
                            g.at("oPlayer1/step#2", 982);
                            g.gset(gv::itemsBought, lenient(Value::add, g.gget(gv::itemsBought)?, &r(1.0)))?;
                        }
                        // 983: global.money -= holdItem.cost;
                        g.at("oPlayer1/step#2", 983);
                        g.gset(gv::money, lenient(Value::sub, g.gget(gv::money)?, &g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::cost)?))?;
                        // 984: scrStealItem();
                        g.at("oPlayer1/step#2", 984);
                        scr::scrStealItem(g, cx, &[])?;
                        // 987: global.messageTimer = 80;
                        g.at("oPlayer1/step#2", 987);
                        g.gset(gv::messageTimer, r(80.0))?;
                    }
                }
            }
            // 992: if (global.blackMarket && global.roomPath[scrGetRoomX(x), scrGetRoomY(y)] == 5 || (!global.blackMark
            g.at("oPlayer1/step#2", 992);
            if or(and(g.gget(gv::blackMarket)?.is_truthy(), ({ let i = ix(&{ let a = [g.get(cx.this, v::x)?]; scr::scrGetRoomX(g, cx, &a)? })?; let j = ix(&{ let a = [g.get(cx.this, v::y)?]; scr::scrGetRoomY(g, cx, &a)? })?; g.gget_a(gv::roomPath, i, j)? }).gml_eq(&r(5.0))?), and(g.gget(gv::blackMarket)?.not()?.is_truthy(), g.get_on(cx, &obj::oShopkeeper, v::style)?.gml_eq(&s!("Craps"))?)) {
                // 995: if (global.thiefLevel > 0 || global.murderer)
                g.at("oPlayer1/step#2", 995);
                if or(g.gget(gv::thiefLevel)?.gml_gt(&r(0.0))?, g.gget(gv::murderer)?.is_truthy()) {
                } else {
                    // 999: if (bet == 0 && global.money >= 1000 + global.currLevel * 500)
                    g.at("oPlayer1/step#2", 999);
                    if and(g.get(cx.this, v::bet)?.gml_eq(&r(0.0))?, g.gget(gv::money)?.gml_ge(&r(1000.0).add(&g.gget(gv::currLevel)?.mul(&r(500.0))?)?)?) {
                        // 1001: if (isRealLevel())
                        g.at("oPlayer1/step#2", 1001);
                        if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                            // 1001: global.diceGamesPlayed += 1;
                            g.at("oPlayer1/step#2", 1001);
                            g.gset(gv::diceGamesPlayed, lenient(Value::add, g.gget(gv::diceGamesPlayed)?, &r(1.0)))?;
                        }
                        // 1002: bet = 1000 + global.currLevel * 500;
                        g.at("oPlayer1/step#2", 1002);
                        g.set(cx.this, v::bet, r(1000.0).add(&g.gget(gv::currLevel)?.mul(&r(500.0))?)?)?;
                        // 1003: global.money -= 1000 + global.currLevel * 500;
                        g.at("oPlayer1/step#2", 1003);
                        g.gset(gv::money, lenient(Value::sub, g.gget(gv::money)?, &r(1000.0).add(&g.gget(gv::currLevel)?.mul(&r(500.0))?)?))?;
                        // 1004: global.message = "YOU BET $" + string(1000 + global.currLevel * 500) + "!";
                        g.at("oPlayer1/step#2", 1004);
                        {
                            let x = s!("YOU BET $").add(&{ let a = [r(1000.0).add(&g.gget(gv::currLevel)?.mul(&r(500.0))?)?]; b::string(g, cx, a)? })?.add(&s!("!"))?;
                            g.gset(gv::message, x)?;
                        }
                        // 1005: global.message2 = "NOW ROLL THE DICE!";
                        g.at("oPlayer1/step#2", 1005);
                        g.gset(gv::message2, s!("NOW ROLL THE DICE!"))?;
                        // 1006: global.messageTimer = 200;
                        g.at("oPlayer1/step#2", 1006);
                        g.gset(gv::messageTimer, r(200.0))?;
                    } else {
                        // 1008: if (bet > 0)
                        g.at("oPlayer1/step#2", 1008);
                        if g.get(cx.this, v::bet)?.gml_gt(&r(0.0))? {
                            // 1010: global.message = "ONE BET AT A TIME!";
                            g.at("oPlayer1/step#2", 1010);
                            g.gset(gv::message, s!("ONE BET AT A TIME!"))?;
                            // 1011: global.message2 = "PLEASE ROLL THE DICE!";
                            g.at("oPlayer1/step#2", 1011);
                            g.gset(gv::message2, s!("PLEASE ROLL THE DICE!"))?;
                            // 1012: global.messageTimer = 200;
                            g.at("oPlayer1/step#2", 1012);
                            g.gset(gv::messageTimer, r(200.0))?;
                        } else {
                            // 1016: global.message = "YOU NEED $" + string(1000 + global.currLevel * 500) + " TO BET!";
                            g.at("oPlayer1/step#2", 1016);
                            {
                                let x = s!("YOU NEED $").add(&{ let a = [r(1000.0).add(&g.gget(gv::currLevel)?.mul(&r(500.0))?)?]; b::string(g, cx, a)? })?.add(&s!(" TO BET!"))?;
                                g.gset(gv::message, x)?;
                            }
                            // 1017: global.message2 = "";
                            g.at("oPlayer1/step#2", 1017);
                            g.gset(gv::message2, s!(""))?;
                            // 1018: global.messageTimer = 200;
                            g.at("oPlayer1/step#2", 1018);
                            g.gset(gv::messageTimer, r(200.0))?;
                        }
                    }
                }
            }
            // 1022: if (oShopkeeper.style == "Kissing" && distance_to_object(oDamsel) < 16)
            g.at("oPlayer1/step#2", 1022);
            if and(g.get_on(cx, &obj::oShopkeeper, v::style)?.gml_eq(&s!("Kissing"))?, b::distance_to_object(g, cx, [obj::oDamsel])?.gml_lt(&r(16.0))?) {
                // 1024: obj = instance_nearest(x, y, oDamsel);
                g.at("oPlayer1/step#2", 1024);
                {
                    let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oDamsel]; b::instance_nearest(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 1025: if (global.thiefLevel > 0 || global.murderer || !obj.forSale)
                g.at("oPlayer1/step#2", 1025);
                if or(or(g.gget(gv::thiefLevel)?.gml_gt(&r(0.0))?, g.gget(gv::murderer)?.is_truthy()), g.get_on(cx, &g.get(cx.this, v::obj)?, v::forSale)?.not()?.is_truthy()) {
                } else {
                    // 1029: if (n == 0 && global.money >= getKissValue())
                    g.at("oPlayer1/step#2", 1029);
                    if and(g.get(cx.this, v::n)?.gml_eq(&r(0.0))?, g.gget(gv::money)?.gml_ge(&scr::getKissValue(g, cx, &[])?)?) {
                        // 1031: if (obj.forSale && !obj.held)
                        g.at("oPlayer1/step#2", 1031);
                        if and(g.get_on(cx, &g.get(cx.this, v::obj)?, v::forSale)?.is_truthy(), g.get_on(cx, &g.get(cx.this, v::obj)?, v::held)?.not()?.is_truthy()) {
                            // 1033: if (isRealLevel())
                            g.at("oPlayer1/step#2", 1033);
                            if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                                // 1033: global.kissesBought += 1;
                                g.at("oPlayer1/step#2", 1033);
                                g.gset(gv::kissesBought, lenient(Value::add, g.gget(gv::kissesBought)?, &r(1.0)))?;
                            }
                            // 1034: obj.status = 6;
                            g.at("oPlayer1/step#2", 1034);
                            { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::status, r(6.0))?; }
                            // 1035: if (global.isDamsel)
                            g.at("oPlayer1/step#2", 1035);
                            if g.gget(gv::isDamsel)?.is_truthy() {
                                // 1037: obj.sprite_index = sPKissL;
                                g.at("oPlayer1/step#2", 1037);
                                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sPKissL)?; }
                            } else {
                                // 1041: obj.sprite_index = sDamselKissL;
                                g.at("oPlayer1/step#2", 1041);
                                { let t = g.get(cx.this, v::obj)?; g.set_on(cx, &t, v::sprite_index, spr::sDamselKissL)?; }
                            }
                            // 1043: global.money -= getKissValue();
                            g.at("oPlayer1/step#2", 1043);
                            {
                                let old = g.gget(gv::money)?;
                                let x = lenient(Value::sub, old, &scr::getKissValue(g, cx, &[])?);
                                g.gset(gv::money, x)?;
                            }
                            // 1044: global.plife += 1;
                            g.at("oPlayer1/step#2", 1044);
                            g.gset(gv::plife, lenient(Value::add, g.gget(gv::plife)?, &r(1.0)))?;
                            // 1045: if (global.isDamsel)
                            g.at("oPlayer1/step#2", 1045);
                            if g.gget(gv::isDamsel)?.is_truthy() {
                                // 1045: global.message = "NOW AIN'T HE SWEET!";
                                g.at("oPlayer1/step#2", 1045);
                                g.gset(gv::message, s!("NOW AIN'T HE SWEET!"))?;
                            } else {
                                // 1046: global.message = "NOW AIN'T SHE SWEET!";
                                g.at("oPlayer1/step#2", 1046);
                                g.gset(gv::message, s!("NOW AIN'T SHE SWEET!"))?;
                            }
                            // 1047: global.message2 = "";
                            g.at("oPlayer1/step#2", 1047);
                            g.gset(gv::message2, s!(""))?;
                            // 1048: global.messageTimer = 200;
                            g.at("oPlayer1/step#2", 1048);
                            g.gset(gv::messageTimer, r(200.0))?;
                        }
                    } else {
                        // 1053: if (n == 0)
                        g.at("oPlayer1/step#2", 1053);
                        if g.get(cx.this, v::n)?.gml_eq(&r(0.0))? {
                            // 1053: global.message = "YOU NEED $" + string(getKissValue()) + "!";
                            g.at("oPlayer1/step#2", 1053);
                            {
                                let x = s!("YOU NEED $").add(&{ let a = [scr::getKissValue(g, cx, &[])?]; b::string(g, cx, a)? })?.add(&s!("!"))?;
                                g.gset(gv::message, x)?;
                            }
                        } else {
                            // 1054: global.message = "YOU NEED $" + string(obj.cost) + "!";
                            g.at("oPlayer1/step#2", 1054);
                            {
                                let x = s!("YOU NEED $").add(&{ let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::cost)?]; b::string(g, cx, a)? })?.add(&s!("!"))?;
                                g.gset(gv::message, x)?;
                            }
                        }
                        // 1055: global.message2 = "GET OUTTA HERE, DEADBEAT!";
                        g.at("oPlayer1/step#2", 1055);
                        g.gset(gv::message2, s!("GET OUTTA HERE, DEADBEAT!"))?;
                        // 1056: global.messageTimer = 200;
                        g.at("oPlayer1/step#2", 1056);
                        g.gset(gv::messageTimer, r(200.0))?;
                    }
                }
            }
        }
    }
    // 1063: if (kAttack && bowArmed && bowStrength < 12)
    g.at("oPlayer1/step#2", 1063);
    if and(and(g.get(cx.this, v::kAttack)?.is_truthy(), g.get(cx.this, v::bowArmed)?.is_truthy()), g.get(cx.this, v::bowStrength)?.gml_lt(&r(12.0))?) {
        // 1065: bowStrength += 0.2;
        g.at("oPlayer1/step#2", 1065);
        g.set(cx.this, v::bowStrength, lenient(Value::add, g.get(cx.this, v::bowStrength)?, &r(0.2)))?;
        // 1066: if (!holdItem)
        g.at("oPlayer1/step#2", 1066);
        if g.get(cx.this, v::holdItem)?.not()?.is_truthy() {
            // 1068: bowArmed = false;
            g.at("oPlayer1/step#2", 1068);
            g.set(cx.this, v::bowArmed, r(0.0))?;
            // 1069: bowStrength = 0;
            g.at("oPlayer1/step#2", 1069);
            g.set(cx.this, v::bowStrength, r(0.0))?;
            // 1070: if (SS_IsSoundPlaying(global.sndBowPull))
            g.at("oPlayer1/step#2", 1070);
            if ({ let a = [g.gget(gv::sndBowPull)?]; scr::SS_IsSoundPlaying(g, cx, &a)? }).is_truthy() {
                // 1071: SS_StopSound(global.sndBowPull);
                g.at("oPlayer1/step#2", 1071);
                { let a = [g.gget(gv::sndBowPull)?]; scr::SS_StopSound(g, cx, &a)? };
            }
        }
        // 1073: holdArrow = ARROW_NORM;
        g.at("oPlayer1/step#2", 1073);
        g.set(cx.this, v::holdArrow, g.get(cx.this, v::ARROW_NORM)?)?;
    }
    // 1076: if (kAttackReleased && bowArmed)
    g.at("oPlayer1/step#2", 1076);
    if and(g.get(cx.this, v::kAttackReleased)?.is_truthy(), g.get(cx.this, v::bowArmed)?.is_truthy()) {
        // 1078: scrFireBow();
        g.at("oPlayer1/step#2", 1078);
        scr::scrFireBow(g, cx, &[])?;
    }
    Ok(())
}

/// GML: `oPlayer1/step#4`.
pub fn step_a4(g: &mut World, cx: &mut Ctx) -> R<()> {
    let cx = &mut cx.clone();
    // 1: if (global.plife < -10000)
    g.at("oPlayer1/step#4", 1);
    if g.gget(gv::plife)?.gml_lt(&r(-10000.0))? {
        // 1: global.plife = -10000;
        g.at("oPlayer1/step#4", 1);
        g.gset(gv::plife, r(-10000.0))?;
    }
    // 3: if (global.plife < -99 && visible)
    g.at("oPlayer1/step#4", 3);
    if and(g.gget(gv::plife)?.gml_lt(&r(-99.0))?, g.get(cx.this, v::visible)?.is_truthy()) {
        // 5: scrCreateBlood(x, y, 3);
        g.at("oPlayer1/step#4", 5);
        { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(3.0)]; scr::scrCreateBlood(g, cx, &a)? };
        // 6: visible = false;
        g.at("oPlayer1/step#4", 6);
        g.set(cx.this, v::visible, r(0.0))?;
    }
    // 9: if (global.plife >= -99 && visible && sprite_index != sPExit && sprite_index != sDamselExit && sprit
    g.at("oPlayer1/step#4", 9);
    if and(and(and(and(g.gget(gv::plife)?.gml_ge(&r(-99.0))?, g.get(cx.this, v::visible)?.is_truthy()), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sPExit)?), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sDamselExit)?), g.get(cx.this, v::sprite_index)?.gml_ne(&spr::sTunnelExit)?) {
        // 12: if (collision_point(x, y, oSolid, 0, 0))
        g.at("oPlayer1/step#4", 12);
        if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
            // 14: if (global.plife > 0)
            g.at("oPlayer1/step#4", 14);
            if g.gget(gv::plife)?.gml_gt(&r(0.0))? {
                // 16: if (isRealLevel())
                g.at("oPlayer1/step#4", 16);
                if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                    // 18: if (isRoom("rOlmec"))
                    g.at("oPlayer1/step#4", 18);
                    if scr::isRoom(g, cx, &[s!("rOlmec")])?.is_truthy() {
                        // 18: global.enemyDeaths[22] += 1;
                        g.at("oPlayer1/step#4", 18);
                        g.gset_a(gv::enemyDeaths, 0, 22, lenient(Value::add, g.gget_a(gv::enemyDeaths, 0, 22)?, &r(1.0)))?;
                    } else {
                        // 19: global.miscDeaths[2] += 1;
                        g.at("oPlayer1/step#4", 19);
                        g.gset_a(gv::miscDeaths, 0, 2, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 2)?, &r(1.0)))?;
                    }
                }
            }
            // 22: global.plife -= 99;
            g.at("oPlayer1/step#4", 22);
            g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(99.0)))?;
            // 23: active = false;
            g.at("oPlayer1/step#4", 23);
            g.set(cx.this, v::active, r(0.0))?;
            // 24: yVel = -3;
            g.at("oPlayer1/step#4", 24);
            g.set(cx.this, v::yVel, r(-3.0))?;
            // 25: playSound(global.sndDie);
            g.at("oPlayer1/step#4", 25);
            { let a = [g.gget(gv::sndDie)?]; scr::playSound(g, cx, &a)? };
            // 27: scrCreateBlood(x, y, 3);
            g.at("oPlayer1/step#4", 27);
            { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(3.0)]; scr::scrCreateBlood(g, cx, &a)? };
            // 29: visible = false;
            g.at("oPlayer1/step#4", 29);
            g.set(cx.this, v::visible, r(0.0))?;
        }
        // 32: if (collision_rectangle(x - 8, y - 8, x + 8, y + 8, oArrow, 0, 0))
        g.at("oPlayer1/step#4", 32);
        if ({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oArrow, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy() {
            // 34: obj = instance_nearest(x, y, oArrow);
            g.at("oPlayer1/step#4", 34);
            {
                let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oArrow]; b::instance_nearest(g, cx, a)? };
                g.set(cx.this, v::obj, x)?;
            }
            // 35: if (obj && abs(obj.xVel) > 3 && !obj.safe)
            g.at("oPlayer1/step#4", 35);
            if and(and(g.get(cx.this, v::obj)?.is_truthy(), ({ let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::xVel)?]; b::abs(g, cx, a)? }).gml_gt(&r(3.0))?), g.get_on(cx, &g.get(cx.this, v::obj)?, v::safe)?.not()?.is_truthy()) {
                // 37: if (global.plife > 0)
                g.at("oPlayer1/step#4", 37);
                if g.gget(gv::plife)?.gml_gt(&r(0.0))? {
                    // 39: global.plife -= 2;
                    g.at("oPlayer1/step#4", 39);
                    g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(2.0)))?;
                    // 40: if (global.plife <= 0 && isRealLevel())
                    g.at("oPlayer1/step#4", 40);
                    if and(g.gget(gv::plife)?.gml_le(&r(0.0))?, scr::isRealLevel(g, cx, &[])?.is_truthy()) {
                        // 40: global.miscDeaths[6] += 1;
                        g.at("oPlayer1/step#4", 40);
                        g.gset_a(gv::miscDeaths, 0, 6, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 6)?, &r(1.0)))?;
                    }
                }
                // 42: xVel = obj.xVel;
                g.at("oPlayer1/step#4", 42);
                g.set(cx.this, v::xVel, g.get_on(cx, &g.get(cx.this, v::obj)?, v::xVel)?)?;
                // 43: yVel = -4;
                g.at("oPlayer1/step#4", 43);
                g.set(cx.this, v::yVel, r(-4.0))?;
                // 45: scrCreateBlood(x, y, 3);
                g.at("oPlayer1/step#4", 45);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(3.0)]; scr::scrCreateBlood(g, cx, &a)? };
                // 47: with (obj)
                g.at("oPlayer1/step#4", 47);
                {
                    let t = g.get(cx.this, v::obj)?;
                    let mut it1 = g.with_iter(cx, &t)?;
                    let outer3 = (cx.this, cx.other);
                    'with1: while let Some(h2) = it1.next(g) {
                        (cx.this, cx.other) = (h2, outer3.0);
                        // 49: instance_destroy();
                        g.at("oPlayer1/step#4", 49);
                        b::instance_destroy(g, cx, [])?;
                    }
                    (cx.this, cx.other) = outer3;
                }
                // 51: playSound(global.sndHurt);
                g.at("oPlayer1/step#4", 51);
                { let a = [g.gget(gv::sndHurt)?]; scr::playSound(g, cx, &a)? };
                // 52: stunned = true;
                g.at("oPlayer1/step#4", 52);
                g.set(cx.this, v::stunned, r(1.0))?;
                // 53: stunTimer = 20;
                g.at("oPlayer1/step#4", 53);
                g.set(cx.this, v::stunTimer, r(20.0))?;
            }
        }
        // 57: if (collision_rectangle(x - 8, y - 8, x + 8, y + 8, oRock, 0, 0))
        g.at("oPlayer1/step#4", 57);
        if ({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oRock, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy() {
            // 59: obj = instance_nearest(x, y, oRock);
            g.at("oPlayer1/step#4", 59);
            {
                let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oRock]; b::instance_nearest(g, cx, a)? };
                g.set(cx.this, v::obj, x)?;
            }
            // 60: if (obj && abs(obj.xVel) > 4 && !obj.safe && !stunned && !dead)
            g.at("oPlayer1/step#4", 60);
            if and(and(and(and(g.get(cx.this, v::obj)?.is_truthy(), ({ let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::xVel)?]; b::abs(g, cx, a)? }).gml_gt(&r(4.0))?), g.get_on(cx, &g.get(cx.this, v::obj)?, v::safe)?.not()?.is_truthy()), g.get(cx.this, v::stunned)?.not()?.is_truthy()), g.get(cx.this, v::dead)?.not()?.is_truthy()) {
                // 62: if (global.hasMitt && !holdItem)
                g.at("oPlayer1/step#4", 62);
                if and(g.gget(gv::hasMitt)?.is_truthy(), g.get(cx.this, v::holdItem)?.not()?.is_truthy()) {
                    // 64: holdItem = oRock;
                    g.at("oPlayer1/step#4", 64);
                    g.set(cx.this, v::holdItem, obj::oRock)?;
                    // 65: holdItem.held = true;
                    g.at("oPlayer1/step#4", 65);
                    { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(1.0))?; }
                    // 66: pickupItemType = holdItem.type;
                    g.at("oPlayer1/step#4", 66);
                    g.set(cx.this, v::pickupItemType, g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?)?;
                } else {
                    // 69: if (global.plife > 0)
                    g.at("oPlayer1/step#4", 69);
                    if g.gget(gv::plife)?.gml_gt(&r(0.0))? {
                        // 71: global.plife -= 2;
                        g.at("oPlayer1/step#4", 71);
                        g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(2.0)))?;
                        // 72: if (global.plife <= 0 && isRealLevel())
                        g.at("oPlayer1/step#4", 72);
                        if and(g.gget(gv::plife)?.gml_le(&r(0.0))?, scr::isRealLevel(g, cx, &[])?.is_truthy()) {
                            // 72: global.miscDeaths[0] += 1;
                            g.at("oPlayer1/step#4", 72);
                            g.gset_a(gv::miscDeaths, 0, 0, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 0)?, &r(1.0)))?;
                        }
                    }
                    // 74: xVel = obj.xVel;
                    g.at("oPlayer1/step#4", 74);
                    g.set(cx.this, v::xVel, g.get_on(cx, &g.get(cx.this, v::obj)?, v::xVel)?)?;
                    // 75: yVel = -4;
                    g.at("oPlayer1/step#4", 75);
                    g.set(cx.this, v::yVel, r(-4.0))?;
                    // 77: scrCreateBlood(x, y, 3);
                    g.at("oPlayer1/step#4", 77);
                    { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(3.0)]; scr::scrCreateBlood(g, cx, &a)? };
                    // 79: playSound(global.sndHurt);
                    g.at("oPlayer1/step#4", 79);
                    { let a = [g.gget(gv::sndHurt)?]; scr::playSound(g, cx, &a)? };
                    // 80: stunned = true;
                    g.at("oPlayer1/step#4", 80);
                    g.set(cx.this, v::stunned, r(1.0))?;
                    // 81: stunTimer = 20;
                    g.at("oPlayer1/step#4", 81);
                    g.set(cx.this, v::stunTimer, r(20.0))?;
                }
            }
        }
        // 86: if (collision_rectangle(x - 8, y - 8, x + 8, y + 8, oLaser, 0, 0))
        g.at("oPlayer1/step#4", 86);
        if ({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oLaser, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy() {
            // 88: obj = instance_nearest(x, y, oLaser);
            g.at("oPlayer1/step#4", 88);
            {
                let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLaser]; b::instance_nearest(g, cx, a)? };
                g.set(cx.this, v::obj, x)?;
            }
            // 89: if (obj)
            g.at("oPlayer1/step#4", 89);
            if g.get(cx.this, v::obj)?.is_truthy() {
                // 91: if (global.plife > 0)
                g.at("oPlayer1/step#4", 91);
                if g.gget(gv::plife)?.gml_gt(&r(0.0))? {
                    // 93: global.plife -= 3;
                    g.at("oPlayer1/step#4", 93);
                    g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(3.0)))?;
                    // 94: if (global.plife <= 0 && isRealLevel())
                    g.at("oPlayer1/step#4", 94);
                    if and(g.gget(gv::plife)?.gml_le(&r(0.0))?, scr::isRealLevel(g, cx, &[])?.is_truthy()) {
                        // 94: global.enemyDeaths[16] += 1;
                        g.at("oPlayer1/step#4", 94);
                        g.gset_a(gv::enemyDeaths, 0, 16, lenient(Value::add, g.gget_a(gv::enemyDeaths, 0, 16)?, &r(1.0)))?;
                    }
                }
                // 96: if (obj.x < x)
                g.at("oPlayer1/step#4", 96);
                if g.get_on(cx, &g.get(cx.this, v::obj)?, v::x)?.gml_lt(&g.get(cx.this, v::x)?)? {
                    // 96: xVel = 2;
                    g.at("oPlayer1/step#4", 96);
                    g.set(cx.this, v::xVel, r(2.0))?;
                } else {
                    // 97: xVel = -2;
                    g.at("oPlayer1/step#4", 97);
                    g.set(cx.this, v::xVel, r(-2.0))?;
                }
                // 98: yVel = -4;
                g.at("oPlayer1/step#4", 98);
                g.set(cx.this, v::yVel, r(-4.0))?;
                // 100: scrCreateBlood(x, y, 3);
                g.at("oPlayer1/step#4", 100);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(3.0)]; scr::scrCreateBlood(g, cx, &a)? };
                // 102: with (obj)
                g.at("oPlayer1/step#4", 102);
                {
                    let t = g.get(cx.this, v::obj)?;
                    let mut it4 = g.with_iter(cx, &t)?;
                    let outer6 = (cx.this, cx.other);
                    'with2: while let Some(h5) = it4.next(g) {
                        (cx.this, cx.other) = (h5, outer6.0);
                        // 104: instance_create(x, y, oLaserExplode);
                        g.at("oPlayer1/step#4", 104);
                        { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLaserExplode]; b::instance_create(g, cx, a)? };
                        // 105: instance_destroy();
                        g.at("oPlayer1/step#4", 105);
                        b::instance_destroy(g, cx, [])?;
                    }
                    (cx.this, cx.other) = outer6;
                }
                // 107: playSound(global.sndHurt);
                g.at("oPlayer1/step#4", 107);
                { let a = [g.gget(gv::sndHurt)?]; scr::playSound(g, cx, &a)? };
                // 108: stunned = true;
                g.at("oPlayer1/step#4", 108);
                g.set(cx.this, v::stunned, r(1.0))?;
                // 109: stunTimer = 20;
                g.at("oPlayer1/step#4", 109);
                g.set(cx.this, v::stunTimer, r(20.0))?;
            }
        }
        // 113: if (collision_rectangle(x - 8, y - 8, x + 8, y + 8, oPsychicWave, 0, 0) && !stunned && !dead)
        g.at("oPlayer1/step#4", 113);
        if and(and(({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oPsychicWave, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy(), g.get(cx.this, v::stunned)?.not()?.is_truthy()), g.get(cx.this, v::dead)?.not()?.is_truthy()) {
            // 115: obj = instance_nearest(x, y, oPsychicWave);
            g.at("oPlayer1/step#4", 115);
            {
                let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oPsychicWave]; b::instance_nearest(g, cx, a)? };
                g.set(cx.this, v::obj, x)?;
            }
            // 116: if (obj)
            g.at("oPlayer1/step#4", 116);
            if g.get(cx.this, v::obj)?.is_truthy() {
                // 118: if (global.plife > 0)
                g.at("oPlayer1/step#4", 118);
                if g.gget(gv::plife)?.gml_gt(&r(0.0))? {
                    // 120: global.plife -= 1;
                    g.at("oPlayer1/step#4", 120);
                    g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(1.0)))?;
                    // 121: if (global.plife <= 0 && isRealLevel())
                    g.at("oPlayer1/step#4", 121);
                    if and(g.gget(gv::plife)?.gml_le(&r(0.0))?, scr::isRealLevel(g, cx, &[])?.is_truthy()) {
                        // 121: global.enemyDeaths[17] += 1;
                        g.at("oPlayer1/step#4", 121);
                        g.gset_a(gv::enemyDeaths, 0, 17, lenient(Value::add, g.gget_a(gv::enemyDeaths, 0, 17)?, &r(1.0)))?;
                    }
                }
                // 123: if (obj.x < x)
                g.at("oPlayer1/step#4", 123);
                if g.get_on(cx, &g.get(cx.this, v::obj)?, v::x)?.gml_lt(&g.get(cx.this, v::x)?)? {
                    // 123: xVel = 2;
                    g.at("oPlayer1/step#4", 123);
                    g.set(cx.this, v::xVel, r(2.0))?;
                } else {
                    // 124: xVel = -2;
                    g.at("oPlayer1/step#4", 124);
                    g.set(cx.this, v::xVel, r(-2.0))?;
                }
                // 125: yVel = -4;
                g.at("oPlayer1/step#4", 125);
                g.set(cx.this, v::yVel, r(-4.0))?;
                // 127: playSound(global.sndHurt);
                g.at("oPlayer1/step#4", 127);
                { let a = [g.gget(gv::sndHurt)?]; scr::playSound(g, cx, &a)? };
                // 128: stunned = true;
                g.at("oPlayer1/step#4", 128);
                g.set(cx.this, v::stunned, r(1.0))?;
                // 129: stunTimer = 40;
                g.at("oPlayer1/step#4", 129);
                g.set(cx.this, v::stunTimer, r(40.0))?;
            }
        }
        // 133: if (collision_rectangle(x - 8, y - 8, x + 8, y + 8, oExplosion, 0, 0))
        g.at("oPlayer1/step#4", 133);
        if ({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oExplosion, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy() {
            // 135: global.plife -= 10;
            g.at("oPlayer1/step#4", 135);
            g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(10.0)))?;
            // 136: if (global.plife > 0 && isRealLevel())
            g.at("oPlayer1/step#4", 136);
            if and(g.gget(gv::plife)?.gml_gt(&r(0.0))?, scr::isRealLevel(g, cx, &[])?.is_truthy()) {
                // 136: global.miscDeaths[1] += 1;
                g.at("oPlayer1/step#4", 136);
                g.gset_a(gv::miscDeaths, 0, 1, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 1)?, &r(1.0)))?;
            }
            // 137: explosion = instance_nearest(x, y, oExplosion);
            g.at("oPlayer1/step#4", 137);
            {
                let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oExplosion]; b::instance_nearest(g, cx, a)? };
                g.set(cx.this, v::explosion, x)?;
            }
            // 138: if (explosion.x < x)
            g.at("oPlayer1/step#4", 138);
            if g.get_on(cx, &g.get(cx.this, v::explosion)?, v::x)?.gml_lt(&g.get(cx.this, v::x)?)? {
                // 138: xVel = rand(4, 6);
                g.at("oPlayer1/step#4", 138);
                { let x = scr::rand(g, cx, &[r(4.0), r(6.0)])?; g.set(cx.this, v::xVel, x)?; }
            } else {
                // 139: xVel = -rand(4, 6);
                g.at("oPlayer1/step#4", 139);
                { let x = scr::rand(g, cx, &[r(4.0), r(6.0)])?.neg()?; g.set(cx.this, v::xVel, x)?; }
            }
            // 140: yVel = -6;
            g.at("oPlayer1/step#4", 140);
            g.set(cx.this, v::yVel, r(-6.0))?;
            // 141: burning = 50;
            g.at("oPlayer1/step#4", 141);
            g.set(cx.this, v::burning, r(50.0))?;
            // 142: stunned = true;
            g.at("oPlayer1/step#4", 142);
            g.set(cx.this, v::stunned, r(1.0))?;
            // 143: stunTimer = 100;
            g.at("oPlayer1/step#4", 143);
            g.set(cx.this, v::stunTimer, r(100.0))?;
            // 145: scrCreateBlood(x, y, 1);
            g.at("oPlayer1/step#4", 145);
            { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(1.0)]; scr::scrCreateBlood(g, cx, &a)? };
        }
        // 148: obj = collision_rectangle(x - 6, y - 6, x + 6, y + 7, oSpearsLeft, 0, 0);
        g.at("oPlayer1/step#4", 148);
        {
            let x = { let a = [g.get(cx.this, v::x)?.sub(&r(6.0))?, g.get(cx.this, v::y)?.sub(&r(6.0))?, g.get(cx.this, v::x)?.add(&r(6.0))?, g.get(cx.this, v::y)?.add(&r(7.0))?, obj::oSpearsLeft, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? };
            g.set(cx.this, v::obj, x)?;
        }
        // 148: instance_nearest(x, y, oSpearsLeft);
        g.at("oPlayer1/step#4", 148);
        { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oSpearsLeft]; b::instance_nearest(g, cx, a)? };
        // 149: if (obj)
        g.at("oPlayer1/step#4", 149);
        if g.get(cx.this, v::obj)?.is_truthy() {
            // 151: if (obj.image_index >= 20 && obj.image_index < 24)
            g.at("oPlayer1/step#4", 151);
            if and(g.get_on(cx, &g.get(cx.this, v::obj)?, v::image_index)?.gml_ge(&r(20.0))?, g.get_on(cx, &g.get(cx.this, v::obj)?, v::image_index)?.gml_lt(&r(24.0))?) {
                // 153: if (global.plife > 0 && isRealLevel())
                g.at("oPlayer1/step#4", 153);
                if and(g.gget(gv::plife)?.gml_gt(&r(0.0))?, scr::isRealLevel(g, cx, &[])?.is_truthy()) {
                    // 153: global.miscDeaths[7] += 1;
                    g.at("oPlayer1/step#4", 153);
                    g.gset_a(gv::miscDeaths, 0, 7, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 7)?, &r(1.0)))?;
                }
                // 157: global.plife -= 4;
                g.at("oPlayer1/step#4", 157);
                g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(4.0)))?;
                // 158: if (obj.sprite_index == sSpearsLeft)
                g.at("oPlayer1/step#4", 158);
                if g.get_on(cx, &g.get(cx.this, v::obj)?, v::sprite_index)?.gml_eq(&spr::sSpearsLeft)? {
                    // 158: xVel = -rand(4, 6);
                    g.at("oPlayer1/step#4", 158);
                    { let x = scr::rand(g, cx, &[r(4.0), r(6.0)])?.neg()?; g.set(cx.this, v::xVel, x)?; }
                } else {
                    // 159: xVel = rand(4, 6);
                    g.at("oPlayer1/step#4", 159);
                    { let x = scr::rand(g, cx, &[r(4.0), r(6.0)])?; g.set(cx.this, v::xVel, x)?; }
                }
                // 160: yVel = -6;
                g.at("oPlayer1/step#4", 160);
                g.set(cx.this, v::yVel, r(-6.0))?;
                // 161: y -= 1;
                g.at("oPlayer1/step#4", 161);
                g.set(cx.this, v::y, g.get(cx.this, v::y)?.sub(&r(1.0))?)?;
                // 164: scrCreateBlood(x, y, 1);
                g.at("oPlayer1/step#4", 164);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(1.0)]; scr::scrCreateBlood(g, cx, &a)? };
            }
        }
        // 168: if (collision_rectangle(x - 6, y - 6, x + 6, y + 7, oSmashTrap, 0, 0))
        g.at("oPlayer1/step#4", 168);
        if ({ let a = [g.get(cx.this, v::x)?.sub(&r(6.0))?, g.get(cx.this, v::y)?.sub(&r(6.0))?, g.get(cx.this, v::x)?.add(&r(6.0))?, g.get(cx.this, v::y)?.add(&r(7.0))?, obj::oSmashTrap, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy() {
            // 170: if (global.plife > 0 && isRealLevel())
            g.at("oPlayer1/step#4", 170);
            if and(g.gget(gv::plife)?.gml_gt(&r(0.0))?, scr::isRealLevel(g, cx, &[])?.is_truthy()) {
                // 170: global.miscDeaths[8] += 1;
                g.at("oPlayer1/step#4", 170);
                g.gset_a(gv::miscDeaths, 0, 8, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 8)?, &r(1.0)))?;
            }
            // 172: obj = instance_nearest(x, y, oSmashTrap);
            g.at("oPlayer1/step#4", 172);
            {
                let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oSmashTrap]; b::instance_nearest(g, cx, a)? };
                g.set(cx.this, v::obj, x)?;
            }
            // 173: global.plife -= 10;
            g.at("oPlayer1/step#4", 173);
            g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(10.0)))?;
            // 174: if (obj.x + 8 < x)
            g.at("oPlayer1/step#4", 174);
            if g.get_on(cx, &g.get(cx.this, v::obj)?, v::x)?.add(&r(8.0))?.gml_lt(&g.get(cx.this, v::x)?)? {
                // 174: xVel = -rand(4, 6);
                g.at("oPlayer1/step#4", 174);
                { let x = scr::rand(g, cx, &[r(4.0), r(6.0)])?.neg()?; g.set(cx.this, v::xVel, x)?; }
            } else {
                // 175: xVel = rand(4, 6);
                g.at("oPlayer1/step#4", 175);
                { let x = scr::rand(g, cx, &[r(4.0), r(6.0)])?; g.set(cx.this, v::xVel, x)?; }
            }
            // 176: yVel = -6;
            g.at("oPlayer1/step#4", 176);
            g.set(cx.this, v::yVel, r(-6.0))?;
            // 177: if (obj)
            g.at("oPlayer1/step#4", 177);
            if g.get(cx.this, v::obj)?.is_truthy() {
                // 179: if (obj.dir == 1)
                g.at("oPlayer1/step#4", 179);
                if g.get_on(cx, &g.get(cx.this, v::obj)?, v::dir)?.gml_eq(&r(1.0))? {
                    // 179: yVel = 4;
                    g.at("oPlayer1/step#4", 179);
                    g.set(cx.this, v::yVel, r(4.0))?;
                }
            }
            // 185: scrCreateBlood(x, y, 1);
            g.at("oPlayer1/step#4", 185);
            { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(1.0)]; scr::scrCreateBlood(g, cx, &a)? };
            // 186: if (holdItem)
            g.at("oPlayer1/step#4", 186);
            if g.get(cx.this, v::holdItem)?.is_truthy() {
                // 186: holdItem.held = false;
                g.at("oPlayer1/step#4", 186);
                { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
                // 186: holdItem = 0;
                g.at("oPlayer1/step#4", 186);
                g.set(cx.this, v::holdItem, r(0.0))?;
            }
        }
        // 189: obj = collision_rectangle(x - 2, y - 9, x + 2, y - 7, oCeilingTrap, 0, 0);
        g.at("oPlayer1/step#4", 189);
        {
            let x = { let a = [g.get(cx.this, v::x)?.sub(&r(2.0))?, g.get(cx.this, v::y)?.sub(&r(9.0))?, g.get(cx.this, v::x)?.add(&r(2.0))?, g.get(cx.this, v::y)?.sub(&r(7.0))?, obj::oCeilingTrap, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? };
            g.set(cx.this, v::obj, x)?;
        }
        // 190: if (obj)
        g.at("oPlayer1/step#4", 190);
        if g.get(cx.this, v::obj)?.is_truthy() {
            // 192: if (obj.status > 0)
            g.at("oPlayer1/step#4", 192);
            if g.get_on(cx, &g.get(cx.this, v::obj)?, v::status)?.gml_gt(&r(0.0))? {
                // 194: if (global.plife > 0 && isRealLevel())
                g.at("oPlayer1/step#4", 194);
                if and(g.gget(gv::plife)?.gml_gt(&r(0.0))?, scr::isRealLevel(g, cx, &[])?.is_truthy()) {
                    // 194: global.miscDeaths[9] += 1;
                    g.at("oPlayer1/step#4", 194);
                    g.gset_a(gv::miscDeaths, 0, 9, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 9)?, &r(1.0)))?;
                }
                // 196: global.plife -= 10;
                g.at("oPlayer1/step#4", 196);
                g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(10.0)))?;
                // 197: scrCreateBlood(x, y, 1);
                g.at("oPlayer1/step#4", 197);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(1.0)]; scr::scrCreateBlood(g, cx, &a)? };
            }
        }
        // 201: colSpikes = false;
        g.at("oPlayer1/step#4", 201);
        g.set(cx.this, v::colSpikes, r(0.0))?;
        // 202: if (collision_rectangle(x - 4, y - 4, x + 4, y + 8, oSpikes, 0, 0))
        g.at("oPlayer1/step#4", 202);
        if ({ let a = [g.get(cx.this, v::x)?.sub(&r(4.0))?, g.get(cx.this, v::y)?.sub(&r(4.0))?, g.get(cx.this, v::x)?.add(&r(4.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oSpikes, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy() {
            // 202: colSpikes = true;
            g.at("oPlayer1/step#4", 202);
            g.set(cx.this, v::colSpikes, r(1.0))?;
        }
        // 204: if (colSpikes && dead)
        g.at("oPlayer1/step#4", 204);
        if and(g.get(cx.this, v::colSpikes)?.is_truthy(), g.get(cx.this, v::dead)?.is_truthy()) {
            // 207: if (!collision_point(x, y + 9, oSolid, 0, 0))
            g.at("oPlayer1/step#4", 207);
            if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(9.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy() {
                // 207: y += 0.05;
                g.at("oPlayer1/step#4", 207);
                g.set(cx.this, v::y, g.get(cx.this, v::y)?.add(&r(0.05))?)?;
            } else {
                // 208: myGrav = 0.6;
                g.at("oPlayer1/step#4", 208);
                g.set(cx.this, v::myGrav, r(0.6000000000000001))?;
            }
        } else {
            // 210: myGrav = 0.6;
            g.at("oPlayer1/step#4", 210);
            g.set(cx.this, v::myGrav, r(0.6000000000000001))?;
        }
        // 212: if (colSpikes && yVel > 0 && (fallTimer > 4 || stunned))
        g.at("oPlayer1/step#4", 212);
        if and(and(g.get(cx.this, v::colSpikes)?.is_truthy(), g.get(cx.this, v::yVel)?.gml_gt(&r(0.0))?), or(g.get(cx.this, v::fallTimer)?.gml_gt(&r(4.0))?, g.get(cx.this, v::stunned)?.is_truthy())) {
            // 214: if (!dead)
            g.at("oPlayer1/step#4", 214);
            if g.get(cx.this, v::dead)?.not()?.is_truthy() {
                // 216: if (isRealLevel())
                g.at("oPlayer1/step#4", 216);
                if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                    // 216: global.miscDeaths[4] += 1;
                    g.at("oPlayer1/step#4", 216);
                    g.gset_a(gv::miscDeaths, 0, 4, lenient(Value::add, g.gget_a(gv::miscDeaths, 0, 4)?, &r(1.0)))?;
                }
                // 217: scrCreateBlood(x, y, 3);
                g.at("oPlayer1/step#4", 217);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, r(3.0)]; scr::scrCreateBlood(g, cx, &a)? };
                // 218: global.plife -= 99;
                g.at("oPlayer1/step#4", 218);
                g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(99.0)))?;
                // 219: xVel = 0;
                g.at("oPlayer1/step#4", 219);
                g.set(cx.this, v::xVel, r(0.0))?;
                // 220: yVel = 0;
                g.at("oPlayer1/step#4", 220);
                g.set(cx.this, v::yVel, r(0.0))?;
                // 221: myGrav = 0;
                g.at("oPlayer1/step#4", 221);
                g.set(cx.this, v::myGrav, r(0.0))?;
            }
            // 224: obj = instance_place(x, y, oSpikes);
            g.at("oPlayer1/step#4", 224);
            {
                let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oSpikes]; b::instance_place(g, cx, a)? };
                g.set(cx.this, v::obj, x)?;
            }
            // 225: if (obj)
            g.at("oPlayer1/step#4", 225);
            if g.get(cx.this, v::obj)?.is_truthy() {
                // 227: with (obj)
                g.at("oPlayer1/step#4", 227);
                {
                    let t = g.get(cx.this, v::obj)?;
                    let mut it7 = g.with_iter(cx, &t)?;
                    let outer9 = (cx.this, cx.other);
                    'with3: while let Some(h8) = it7.next(g) {
                        (cx.this, cx.other) = (h8, outer9.0);
                        // 227: sprite_index = sSpikesBlood;
                        g.at("oPlayer1/step#4", 227);
                        g.set(cx.this, v::sprite_index, spr::sSpikesBlood)?;
                    }
                    (cx.this, cx.other) = outer9;
                }
            }
        }
    }
    // 233: if (dead || stunned && holdItem != 0)
    g.at("oPlayer1/step#4", 233);
    if and(or(g.get(cx.this, v::dead)?.is_truthy(), g.get(cx.this, v::stunned)?.is_truthy()), g.get(cx.this, v::holdItem)?.gml_ne(&r(0.0))?) {
        // 235: holdItem.held = false;
        g.at("oPlayer1/step#4", 235);
        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::held, r(0.0))?; }
        // 237: holdItem.xVel = xVel;
        g.at("oPlayer1/step#4", 237);
        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::xVel, g.get(cx.this, v::xVel)?)?; }
        // 238: holdItem.yVel = -6;
        g.at("oPlayer1/step#4", 238);
        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::yVel, r(-6.0))?; }
        // 239: holdItem.armed = true;
        g.at("oPlayer1/step#4", 239);
        { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::armed, r(1.0))?; }
        // 240: if (holdItem.type == "Damsel")
        g.at("oPlayer1/step#4", 240);
        if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Damsel"))? {
            // 242: holdItem.status = 2;
            g.at("oPlayer1/step#4", 242);
            { let t = g.get(cx.this, v::holdItem)?; g.set_on(cx, &t, v::status, r(2.0))?; }
        } else {
            // 244: if (holdItem.type == "Bow")
            g.at("oPlayer1/step#4", 244);
            if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Bow"))? {
                // 246: scrFireBow();
                g.at("oPlayer1/step#4", 246);
                scr::scrFireBow(g, cx, &[])?;
            }
        }
        // 249: if (holdItem.type == pickupItemType)
        g.at("oPlayer1/step#4", 249);
        if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&g.get(cx.this, v::pickupItemType)?)? {
            // 251: holdItem = 0;
            g.at("oPlayer1/step#4", 251);
            g.set(cx.this, v::holdItem, r(0.0))?;
            // 252: pickupItemType = "";
            g.at("oPlayer1/step#4", 252);
            g.set(cx.this, v::pickupItemType, s!(""))?;
        } else {
            // 254: scrHoldItem(pickupItemType);
            g.at("oPlayer1/step#4", 254);
            { let a = [g.get(cx.this, v::pickupItemType)?]; scr::scrHoldItem(g, cx, &a)? };
        }
    }
    // 257: if (dead || stunned)
    g.at("oPlayer1/step#4", 257);
    if or(g.get(cx.this, v::dead)?.is_truthy(), g.get(cx.this, v::stunned)?.is_truthy()) {
        // 259: if (instance_exists(oParachute))
        g.at("oPlayer1/step#4", 259);
        if b::instance_exists(g, cx, [obj::oParachute])?.is_truthy() {
            // 261: instance_create(x - 8, y - 16 - 8, oParaUsed);
            g.at("oPlayer1/step#4", 261);
            { let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(16.0))?.sub(&r(8.0))?, obj::oParaUsed]; b::instance_create(g, cx, a)? };
            // 262: with (oParachute)
            g.at("oPlayer1/step#4", 262);
            {
                let t = obj::oParachute;
                let mut it10 = g.with_iter(cx, &t)?;
                let outer12 = (cx.this, cx.other);
                'with4: while let Some(h11) = it10.next(g) {
                    (cx.this, cx.other) = (h11, outer12.0);
                    // 262: instance_destroy();
                    g.at("oPlayer1/step#4", 262);
                    b::instance_destroy(g, cx, [])?;
                }
                (cx.this, cx.other) = outer12;
            }
        }
        // 265: if (whipping)
        g.at("oPlayer1/step#4", 265);
        if g.get(cx.this, v::whipping)?.is_truthy() {
            // 267: whipping = false;
            g.at("oPlayer1/step#4", 267);
            g.set(cx.this, v::whipping, r(0.0))?;
            // 268: with (oWhip)
            g.at("oPlayer1/step#4", 268);
            {
                let t = obj::oWhip;
                let mut it13 = g.with_iter(cx, &t)?;
                let outer15 = (cx.this, cx.other);
                'with5: while let Some(h14) = it13.next(g) {
                    (cx.this, cx.other) = (h14, outer15.0);
                    // 270: instance_destroy();
                    g.at("oPlayer1/step#4", 270);
                    b::instance_destroy(g, cx, [])?;
                }
                (cx.this, cx.other) = outer15;
            }
        }
        // 274: if (global.isDamsel)
        g.at("oPlayer1/step#4", 274);
        if g.gget(gv::isDamsel)?.is_truthy() {
            // 276: if (xVel == 0)
            g.at("oPlayer1/step#4", 276);
            if g.get(cx.this, v::xVel)?.gml_eq(&r(0.0))? {
                // 278: if (dead)
                g.at("oPlayer1/step#4", 278);
                if g.get(cx.this, v::dead)?.is_truthy() {
                    // 278: sprite_index = sDamselDieL;
                    g.at("oPlayer1/step#4", 278);
                    g.set(cx.this, v::sprite_index, spr::sDamselDieL)?;
                } else {
                    // 279: if (stunned)
                    g.at("oPlayer1/step#4", 279);
                    if g.get(cx.this, v::stunned)?.is_truthy() {
                        // 279: sprite_index = sDamselStunL;
                        g.at("oPlayer1/step#4", 279);
                        g.set(cx.this, v::sprite_index, spr::sDamselStunL)?;
                    }
                }
            } else {
                // 281: if (bounced)
                g.at("oPlayer1/step#4", 281);
                if g.get(cx.this, v::bounced)?.is_truthy() {
                    // 283: if (yVel < 0)
                    g.at("oPlayer1/step#4", 283);
                    if g.get(cx.this, v::yVel)?.gml_lt(&r(0.0))? {
                        // 283: sprite_index = sDamselBounceL;
                        g.at("oPlayer1/step#4", 283);
                        g.set(cx.this, v::sprite_index, spr::sDamselBounceL)?;
                    } else {
                        // 284: sprite_index = sDamselFallL;
                        g.at("oPlayer1/step#4", 284);
                        g.set(cx.this, v::sprite_index, spr::sDamselFallL)?;
                    }
                } else {
                    // 288: if (xVel < 0)
                    g.at("oPlayer1/step#4", 288);
                    if g.get(cx.this, v::xVel)?.gml_lt(&r(0.0))? {
                        // 288: sprite_index = sDamselDieLL;
                        g.at("oPlayer1/step#4", 288);
                        g.set(cx.this, v::sprite_index, spr::sDamselDieLL)?;
                    } else {
                        // 289: sprite_index = sDamselDieLR;
                        g.at("oPlayer1/step#4", 289);
                        g.set(cx.this, v::sprite_index, spr::sDamselDieLR)?;
                    }
                }
            }
        } else {
            // 292: if (global.isTunnelMan)
            g.at("oPlayer1/step#4", 292);
            if g.gget(gv::isTunnelMan)?.is_truthy() {
                // 294: if (xVel == 0)
                g.at("oPlayer1/step#4", 294);
                if g.get(cx.this, v::xVel)?.gml_eq(&r(0.0))? {
                    // 296: if (dead)
                    g.at("oPlayer1/step#4", 296);
                    if g.get(cx.this, v::dead)?.is_truthy() {
                        // 296: sprite_index = sTunnelDieL;
                        g.at("oPlayer1/step#4", 296);
                        g.set(cx.this, v::sprite_index, spr::sTunnelDieL)?;
                    } else {
                        // 297: if (stunned)
                        g.at("oPlayer1/step#4", 297);
                        if g.get(cx.this, v::stunned)?.is_truthy() {
                            // 297: sprite_index = sTunnelStunL;
                            g.at("oPlayer1/step#4", 297);
                            g.set(cx.this, v::sprite_index, spr::sTunnelStunL)?;
                        }
                    }
                } else {
                    // 299: if (bounced)
                    g.at("oPlayer1/step#4", 299);
                    if g.get(cx.this, v::bounced)?.is_truthy() {
                        // 301: if (yVel < 0)
                        g.at("oPlayer1/step#4", 301);
                        if g.get(cx.this, v::yVel)?.gml_lt(&r(0.0))? {
                            // 301: sprite_index = sTunnelLBounce;
                            g.at("oPlayer1/step#4", 301);
                            g.set(cx.this, v::sprite_index, spr::sTunnelLBounce)?;
                        } else {
                            // 302: sprite_index = sTunnelFallL;
                            g.at("oPlayer1/step#4", 302);
                            g.set(cx.this, v::sprite_index, spr::sTunnelFallL)?;
                        }
                    } else {
                        // 306: if (xVel < 0)
                        g.at("oPlayer1/step#4", 306);
                        if g.get(cx.this, v::xVel)?.gml_lt(&r(0.0))? {
                            // 306: sprite_index = sTunnelDieLL;
                            g.at("oPlayer1/step#4", 306);
                            g.set(cx.this, v::sprite_index, spr::sTunnelDieLL)?;
                        } else {
                            // 307: sprite_index = sTunnelDieLR;
                            g.at("oPlayer1/step#4", 307);
                            g.set(cx.this, v::sprite_index, spr::sTunnelDieLR)?;
                        }
                    }
                }
            } else {
                // 312: if (xVel == 0)
                g.at("oPlayer1/step#4", 312);
                if g.get(cx.this, v::xVel)?.gml_eq(&r(0.0))? {
                    // 314: if (dead)
                    g.at("oPlayer1/step#4", 314);
                    if g.get(cx.this, v::dead)?.is_truthy() {
                        // 314: sprite_index = sDieL;
                        g.at("oPlayer1/step#4", 314);
                        g.set(cx.this, v::sprite_index, spr::sDieL)?;
                    } else {
                        // 315: if (stunned)
                        g.at("oPlayer1/step#4", 315);
                        if g.get(cx.this, v::stunned)?.is_truthy() {
                            // 315: sprite_index = sStunL;
                            g.at("oPlayer1/step#4", 315);
                            g.set(cx.this, v::sprite_index, spr::sStunL)?;
                        }
                    }
                } else {
                    // 317: if (bounced)
                    g.at("oPlayer1/step#4", 317);
                    if g.get(cx.this, v::bounced)?.is_truthy() {
                        // 319: if (yVel < 0)
                        g.at("oPlayer1/step#4", 319);
                        if g.get(cx.this, v::yVel)?.gml_lt(&r(0.0))? {
                            // 319: sprite_index = sDieLBounce;
                            g.at("oPlayer1/step#4", 319);
                            g.set(cx.this, v::sprite_index, spr::sDieLBounce)?;
                        } else {
                            // 320: sprite_index = sDieLFall;
                            g.at("oPlayer1/step#4", 320);
                            g.set(cx.this, v::sprite_index, spr::sDieLFall)?;
                        }
                    } else {
                        // 324: if (xVel < 0)
                        g.at("oPlayer1/step#4", 324);
                        if g.get(cx.this, v::xVel)?.gml_lt(&r(0.0))? {
                            // 324: sprite_index = sDieLL;
                            g.at("oPlayer1/step#4", 324);
                            g.set(cx.this, v::sprite_index, spr::sDieLL)?;
                        } else {
                            // 325: sprite_index = sDieLR;
                            g.at("oPlayer1/step#4", 325);
                            g.set(cx.this, v::sprite_index, spr::sDieLR)?;
                        }
                    }
                }
            }
        }
        // 329: if (collision_point(x, y, oSpikes, 0, 0) && dead && yVel != 0)
        g.at("oPlayer1/step#4", 329);
        if and(and(({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oSpikes, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), g.get(cx.this, v::dead)?.is_truthy()), g.get(cx.this, v::yVel)?.gml_ne(&r(0.0))?) {
            // 331: if (rand(1, 8) == 1)
            g.at("oPlayer1/step#4", 331);
            if scr::rand(g, cx, &[r(1.0), r(8.0)])?.gml_eq(&r(1.0))? {
                // 331: scrCreateBlood(other.x, other.y, 1);
                g.at("oPlayer1/step#4", 331);
                { let a = [g.get(cx.other, v::x)?, g.get(cx.other, v::y)?, r(1.0)]; scr::scrCreateBlood(g, cx, &a)? };
            }
        }
        // 334: if (isCollisionRight(1) || isCollisionLeft(1) || isCollisionBottom(1))
        g.at("oPlayer1/step#4", 334);
        if or(or(scr::isCollisionRight(g, cx, &[r(1.0)])?.is_truthy(), scr::isCollisionLeft(g, cx, &[r(1.0)])?.is_truthy()), scr::isCollisionBottom(g, cx, &[r(1.0)])?.is_truthy()) {
            // 336: if (wallHurt > 0)
            g.at("oPlayer1/step#4", 336);
            if g.get(cx.this, v::wallHurt)?.gml_gt(&r(0.0))? {
                // 338: for (...; i < 3; ...)
                g.at("oPlayer1/step#4", 338);
                // 338: i = 0;
                g.at("oPlayer1/step#4", 338);
                g.set(cx.this, v::i, r(0.0))?;
                'for6: loop {
                    if !(g.get(cx.this, v::i)?.gml_lt(&r(3.0))?) { break 'for6; }
                    'body7: {
                        // 340: instance_create(other.x, other.y, oBlood);
                        g.at("oPlayer1/step#4", 340);
                        { let a = [g.get(cx.other, v::x)?, g.get(cx.other, v::y)?, obj::oBlood]; b::instance_create(g, cx, a)? };
                    }
                    // 338: i += 1;
                    g.at("oPlayer1/step#4", 338);
                    g.set(cx.this, v::i, lenient(Value::add, g.get(cx.this, v::i)?, &r(1.0)))?;
                }
                // 342: global.plife -= 1;
                g.at("oPlayer1/step#4", 342);
                g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(1.0)))?;
                // 343: wallHurt -= 1;
                g.at("oPlayer1/step#4", 343);
                g.set(cx.this, v::wallHurt, lenient(Value::sub, g.get(cx.this, v::wallHurt)?, &r(1.0)))?;
                // 344: playSound(global.sndHurt);
                g.at("oPlayer1/step#4", 344);
                { let a = [g.gget(gv::sndHurt)?]; scr::playSound(g, cx, &a)? };
            }
        }
        // 348: if (isCollisionBottom(1) && !bounced)
        g.at("oPlayer1/step#4", 348);
        if and(scr::isCollisionBottom(g, cx, &[r(1.0)])?.is_truthy(), g.get(cx.this, v::bounced)?.not()?.is_truthy()) {
            // 350: bounced = true;
            g.at("oPlayer1/step#4", 350);
            g.set(cx.this, v::bounced, r(1.0))?;
            // 351: for (...; i < 3; ...)
            g.at("oPlayer1/step#4", 351);
            // 351: i = 0;
            g.at("oPlayer1/step#4", 351);
            g.set(cx.this, v::i, r(0.0))?;
            'for8: loop {
                if !(g.get(cx.this, v::i)?.gml_lt(&r(3.0))?) { break 'for8; }
                'body9: {
                    // 353: scrCreateBlood(other.x, other.y, 1);
                    g.at("oPlayer1/step#4", 353);
                    { let a = [g.get(cx.other, v::x)?, g.get(cx.other, v::y)?, r(1.0)]; scr::scrCreateBlood(g, cx, &a)? };
                }
                // 351: i += 1;
                g.at("oPlayer1/step#4", 351);
                g.set(cx.this, v::i, lenient(Value::add, g.get(cx.this, v::i)?, &r(1.0)))?;
            }
            // 356: if (wallHurt > 0)
            g.at("oPlayer1/step#4", 356);
            if g.get(cx.this, v::wallHurt)?.gml_gt(&r(0.0))? {
                // 358: global.plife -= 1;
                g.at("oPlayer1/step#4", 358);
                g.gset(gv::plife, lenient(Value::sub, g.gget(gv::plife)?, &r(1.0)))?;
                // 359: wallHurt -= 1;
                g.at("oPlayer1/step#4", 359);
                g.set(cx.this, v::wallHurt, lenient(Value::sub, g.get(cx.this, v::wallHurt)?, &r(1.0)))?;
            }
        }
    }
    Ok(())
}

/// GML: `oPlayer1/step#5`.
pub fn step_a5(g: &mut World, cx: &mut Ctx) -> R<()> {
    let cx = &mut cx.clone();
    // 3: if (dead && deadCounter > 0)
    g.at("oPlayer1/step#5", 3);
    if and(g.get(cx.this, v::dead)?.is_truthy(), g.get(cx.this, v::deadCounter)?.gml_gt(&r(0.0))?) {
        // 3: deadCounter -= 1;
        g.at("oPlayer1/step#5", 3);
        g.set(cx.this, v::deadCounter, lenient(Value::sub, g.get(cx.this, v::deadCounter)?, &r(1.0)))?;
    }
    // 5: if (isLevel() || isRoom("rSun") || isRoom("rMoon") || isRoom("rStars"))
    g.at("oPlayer1/step#5", 5);
    if or(or(or(scr::isLevel(g, cx, &[])?.is_truthy(), scr::isRoom(g, cx, &[s!("rSun")])?.is_truthy()), scr::isRoom(g, cx, &[s!("rMoon")])?.is_truthy()), scr::isRoom(g, cx, &[s!("rStars")])?.is_truthy()) {
        // 7: if (!dead && global.plife < 1)
        g.at("oPlayer1/step#5", 7);
        if and(g.get(cx.this, v::dead)?.not()?.is_truthy(), g.gget(gv::plife)?.gml_lt(&r(1.0))?) {
            // 9: if (global.hasAnkh)
            g.at("oPlayer1/step#5", 9);
            if g.gget(gv::hasAnkh)?.is_truthy() {
                // 11: global.plife = 4;
                g.at("oPlayer1/step#5", 11);
                g.gset(gv::plife, r(4.0))?;
                // 12: if (instance_exists(oMoai))
                g.at("oPlayer1/step#5", 12);
                if b::instance_exists(g, cx, [obj::oMoai])?.is_truthy() {
                    // 14: with (oMoaiInside)
                    g.at("oPlayer1/step#5", 14);
                    {
                        let t = obj::oMoaiInside;
                        let mut it1 = g.with_iter(cx, &t)?;
                        let outer3 = (cx.this, cx.other);
                        'with1: while let Some(h2) = it1.next(g) {
                            (cx.this, cx.other) = (h2, outer3.0);
                            // 14: instance_destroy();
                            g.at("oPlayer1/step#5", 14);
                            b::instance_destroy(g, cx, [])?;
                        }
                        (cx.this, cx.other) = outer3;
                    }
                    // 15: x = oMoai.x + 16 + 8;
                    g.at("oPlayer1/step#5", 15);
                    g.set(cx.this, v::x, g.get_on(cx, &obj::oMoai, v::x)?.add(&r(16.0))?.add(&r(8.0))?)?;
                    // 16: y = oMoai.y + 16 + 8;
                    g.at("oPlayer1/step#5", 16);
                    g.set(cx.this, v::y, g.get_on(cx, &obj::oMoai, v::y)?.add(&r(16.0))?.add(&r(8.0))?)?;
                } else {
                    // 18: if (isRoom("rOlmec"))
                    g.at("oPlayer1/step#5", 18);
                    if scr::isRoom(g, cx, &[s!("rOlmec")])?.is_truthy() {
                        // 20: x = 16 + 8;
                        g.at("oPlayer1/step#5", 20);
                        g.set(cx.this, v::x, r(16.0).add(&r(8.0))?)?;
                        // 21: y = 544 + 8;
                        g.at("oPlayer1/step#5", 21);
                        g.set(cx.this, v::y, r(544.0).add(&r(8.0))?)?;
                    } else {
                        // 25: instance_activate_object(oEntrance);
                        g.at("oPlayer1/step#5", 25);
                        b::instance_activate_object(g, cx, [obj::oEntrance])?;
                        // 26: x = oEntrance.x + 8;
                        g.at("oPlayer1/step#5", 26);
                        g.set(cx.this, v::x, g.get_on(cx, &obj::oEntrance, v::x)?.add(&r(8.0))?)?;
                        // 27: y = oEntrance.y + 8;
                        g.at("oPlayer1/step#5", 27);
                        g.set(cx.this, v::y, g.get_on(cx, &obj::oEntrance, v::y)?.add(&r(8.0))?)?;
                    }
                }
                // 29: with (oBall)
                g.at("oPlayer1/step#5", 29);
                {
                    let t = obj::oBall;
                    let mut it4 = g.with_iter(cx, &t)?;
                    let outer6 = (cx.this, cx.other);
                    'with2: while let Some(h5) = it4.next(g) {
                        (cx.this, cx.other) = (h5, outer6.0);
                        // 31: x = oPlayer1.x;
                        g.at("oPlayer1/step#5", 31);
                        g.set(cx.this, v::x, g.get_on(cx, &obj::oPlayer1, v::x)?)?;
                        // 32: y = oPlayer1.y;
                        g.at("oPlayer1/step#5", 32);
                        g.set(cx.this, v::y, g.get_on(cx, &obj::oPlayer1, v::y)?)?;
                    }
                    (cx.this, cx.other) = outer6;
                }
                // 34: with (oChain)
                g.at("oPlayer1/step#5", 34);
                {
                    let t = obj::oChain;
                    let mut it7 = g.with_iter(cx, &t)?;
                    let outer9 = (cx.this, cx.other);
                    'with3: while let Some(h8) = it7.next(g) {
                        (cx.this, cx.other) = (h8, outer9.0);
                        // 36: x = oPlayer1.x;
                        g.at("oPlayer1/step#5", 36);
                        g.set(cx.this, v::x, g.get_on(cx, &obj::oPlayer1, v::x)?)?;
                        // 37: y = oPlayer1.y;
                        g.at("oPlayer1/step#5", 37);
                        g.set(cx.this, v::y, g.get_on(cx, &obj::oPlayer1, v::y)?)?;
                    }
                    (cx.this, cx.other) = outer9;
                }
                // 39: xVel = 0;
                g.at("oPlayer1/step#5", 39);
                g.set(cx.this, v::xVel, r(0.0))?;
                // 40: yVel = 0;
                g.at("oPlayer1/step#5", 40);
                g.set(cx.this, v::yVel, r(0.0))?;
                // 41: blink = 60;
                g.at("oPlayer1/step#5", 41);
                g.set(cx.this, v::blink, r(60.0))?;
                // 42: invincible = 60;
                g.at("oPlayer1/step#5", 42);
                g.set(cx.this, v::invincible, r(60.0))?;
                // 43: fallTimer = 0;
                g.at("oPlayer1/step#5", 43);
                g.set(cx.this, v::fallTimer, r(0.0))?;
                // 44: visible = true;
                g.at("oPlayer1/step#5", 44);
                g.set(cx.this, v::visible, r(1.0))?;
                // 45: active = true;
                g.at("oPlayer1/step#5", 45);
                g.set(cx.this, v::active, r(1.0))?;
                // 46: dead = false;
                g.at("oPlayer1/step#5", 46);
                g.set(cx.this, v::dead, r(0.0))?;
                // 47: global.hasAnkh = false;
                g.at("oPlayer1/step#5", 47);
                g.gset(gv::hasAnkh, r(0.0))?;
                // 48: global.message = "THE ANKH SHATTERS!";
                g.at("oPlayer1/step#5", 48);
                g.gset(gv::message, s!("THE ANKH SHATTERS!"))?;
                // 49: global.message2 = "YOU HAVE BEEN REVIVED!";
                g.at("oPlayer1/step#5", 49);
                g.gset(gv::message2, s!("YOU HAVE BEEN REVIVED!"))?;
                // 50: global.messageTimer = 150;
                g.at("oPlayer1/step#5", 50);
                g.gset(gv::messageTimer, r(150.0))?;
                // 51: playSound(global.sndTeleport);
                g.at("oPlayer1/step#5", 51);
                { let a = [g.gget(gv::sndTeleport)?]; scr::playSound(g, cx, &a)? };
            } else {
                // 55: global.plife = 0;
                g.at("oPlayer1/step#5", 55);
                g.gset(gv::plife, r(0.0))?;
                // 56: global.drawHUD = false;
                g.at("oPlayer1/step#5", 56);
                g.gset(gv::drawHUD, r(0.0))?;
                // 58: if (isRoom("rSun"))
                g.at("oPlayer1/step#5", 58);
                if scr::isRoom(g, cx, &[s!("rSun")])?.is_truthy() {
                    // 58: global.mini1 = oSunRoom.points;
                    g.at("oPlayer1/step#5", 58);
                    g.gset(gv::mini1, g.get_on(cx, &obj::oSunRoom, v::points)?)?;
                }
                // 59: if (isRoom("rMoon"))
                g.at("oPlayer1/step#5", 59);
                if scr::isRoom(g, cx, &[s!("rMoon")])?.is_truthy() {
                    // 61: global.mini2 = oMoonRoom.baskets;
                    g.at("oPlayer1/step#5", 61);
                    g.gset(gv::mini2, g.get_on(cx, &obj::oMoonRoom, v::baskets)?)?;
                    // 62: oMoonRoom.timer = -1;
                    g.at("oPlayer1/step#5", 62);
                    { let t = obj::oMoonRoom; g.set_on(cx, &t, v::timer, r(-1.0))?; }
                    // 63: oMoonRoom.alarm[10] = 30;
                    g.at("oPlayer1/step#5", 63);
                    { let t = obj::oMoonRoom; g.set_on_a(cx, &t, v::alarm, 0, 10, r(30.0))?; }
                }
                // 65: if (isRoom("rStars"))
                g.at("oPlayer1/step#5", 65);
                if scr::isRoom(g, cx, &[s!("rStars")])?.is_truthy() {
                    // 65: global.mini3 = oStarsRoom.kills;
                    g.at("oPlayer1/step#5", 65);
                    g.gset(gv::mini3, g.get_on(cx, &obj::oStarsRoom, v::kills)?)?;
                }
                // 66: if (global.mini1 > 99)
                g.at("oPlayer1/step#5", 66);
                if g.gget(gv::mini1)?.gml_gt(&r(99.0))? {
                    // 66: global.mini1 = 99;
                    g.at("oPlayer1/step#5", 66);
                    g.gset(gv::mini1, r(99.0))?;
                }
                // 67: if (global.mini2 > 99)
                g.at("oPlayer1/step#5", 67);
                if g.gget(gv::mini2)?.gml_gt(&r(99.0))? {
                    // 67: global.mini2 = 99;
                    g.at("oPlayer1/step#5", 67);
                    g.gset(gv::mini2, r(99.0))?;
                }
                // 68: if (global.mini3 > 99)
                g.at("oPlayer1/step#5", 68);
                if g.gget(gv::mini3)?.gml_gt(&r(99.0))? {
                    // 68: global.mini3 = 99;
                    g.at("oPlayer1/step#5", 68);
                    g.gset(gv::mini3, r(99.0))?;
                }
                // 70: if (isRoom("rSun") || isRoom("rMoon") || isRoom("rStars"))
                g.at("oPlayer1/step#5", 70);
                if or(or(scr::isRoom(g, cx, &[s!("rSun")])?.is_truthy(), scr::isRoom(g, cx, &[s!("rMoon")])?.is_truthy()), scr::isRoom(g, cx, &[s!("rStars")])?.is_truthy()) {
                    // 70: scrUpdateHighscores(2);
                    g.at("oPlayer1/step#5", 70);
                    scr::scrUpdateHighscores(g, cx, &[r(2.0)])?;
                } else {
                    // 71: scrUpdateHighscores(0);
                    g.at("oPlayer1/step#5", 71);
                    scr::scrUpdateHighscores(g, cx, &[r(0.0)])?;
                }
                // 73: dead = true;
                g.at("oPlayer1/step#5", 73);
                g.set(cx.this, v::dead, r(1.0))?;
                // 75: playSound(global.sndDie);
                g.at("oPlayer1/step#5", 75);
                { let a = [g.gget(gv::sndDie)?]; scr::playSound(g, cx, &a)? };
            }
        }
        // 79: if (dead)
        g.at("oPlayer1/step#5", 79);
        if g.get(cx.this, v::dead)?.is_truthy() {
            // 81: stopAllMusic();
            g.at("oPlayer1/step#5", 81);
            scr::stopAllMusic(g, cx, &[])?;
        }
    }
    Ok(())
}

/// GML: `oPlayer1/step#6`.
pub fn step_a6(g: &mut World, cx: &mut Ctx) -> R<()> {
    // 1: if (!dead && invincible > 0)
    g.at("oPlayer1/step#6", 1);
    if and(g.get(cx.this, v::dead)?.not()?.is_truthy(), g.get(cx.this, v::invincible)?.gml_gt(&r(0.0))?) {
        // 2: invincible -= 1;
        g.at("oPlayer1/step#6", 2);
        g.set(cx.this, v::invincible, lenient(Value::sub, g.get(cx.this, v::invincible)?, &r(1.0)))?;
    }
    // 4: if (blink > 0)
    g.at("oPlayer1/step#6", 4);
    if g.get(cx.this, v::blink)?.gml_gt(&r(0.0))? {
        // 6: blinkToggle *= -1;
        g.at("oPlayer1/step#6", 6);
        g.set(cx.this, v::blinkToggle, lenient(Value::mul, g.get(cx.this, v::blinkToggle)?, &r(-1.0)))?;
        // 7: blink -= 1;
        g.at("oPlayer1/step#6", 7);
        g.set(cx.this, v::blink, lenient(Value::sub, g.get(cx.this, v::blink)?, &r(1.0)))?;
    } else {
        // 9: blinkToggle = -1;
        g.at("oPlayer1/step#6", 9);
        g.set(cx.this, v::blinkToggle, r(-1.0))?;
    }
    Ok(())
}

/// GML: `oPlayer1/step#8`.
pub fn step_a8(g: &mut World, cx: &mut Ctx) -> R<()> {
    let cx = &mut cx.clone();
    // 1: money = global.money;
    g.at("oPlayer1/step#8", 1);
    g.set(cx.this, v::money, g.gget(gv::money)?)?;
    // 3: if (global.collectCounter == 0)
    g.at("oPlayer1/step#8", 3);
    if g.gget(gv::collectCounter)?.gml_eq(&r(0.0))? {
        // 5: if (global.collect > 100)
        g.at("oPlayer1/step#8", 5);
        if g.gget(gv::collect)?.gml_gt(&r(100.0))? {
            // 7: global.money += 100;
            g.at("oPlayer1/step#8", 7);
            g.gset(gv::money, lenient(Value::add, g.gget(gv::money)?, &r(100.0)))?;
            // 8: global.collect -= 100;
            g.at("oPlayer1/step#8", 8);
            g.gset(gv::collect, lenient(Value::sub, g.gget(gv::collect)?, &r(100.0)))?;
        } else {
            // 12: global.money += global.collect;
            g.at("oPlayer1/step#8", 12);
            g.gset(gv::money, lenient(Value::add, g.gget(gv::money)?, &g.gget(gv::collect)?))?;
            // 13: global.collect -= global.collect;
            g.at("oPlayer1/step#8", 13);
            g.gset(gv::collect, lenient(Value::sub, g.gget(gv::collect)?, &g.gget(gv::collect)?))?;
        }
    } else {
        // 18: global.collectCounter -= 1;
        g.at("oPlayer1/step#8", 18);
        g.gset(gv::collectCounter, lenient(Value::sub, g.gget(gv::collectCounter)?, &r(1.0)))?;
    }
    // 21: if (holdItem)
    g.at("oPlayer1/step#8", 21);
    if g.get(cx.this, v::holdItem)?.is_truthy() {
        // 23: if (holdItem.type == "Bow")
        g.at("oPlayer1/step#8", 23);
        if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Bow"))? {
            // 25: if (collision_rectangle(x - 8, y - 8, x + 8, y + 8, oArrow, 0, 0) && !dead && !stunned)
            g.at("oPlayer1/step#8", 25);
            if and(and(({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oArrow, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy(), g.get(cx.this, v::dead)?.not()?.is_truthy()), g.get(cx.this, v::stunned)?.not()?.is_truthy()) {
                // 27: obj = instance_nearest(x, y, oArrow);
                g.at("oPlayer1/step#8", 27);
                {
                    let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oArrow]; b::instance_nearest(g, cx, a)? };
                    g.set(cx.this, v::obj, x)?;
                }
                // 28: if (abs(obj.xVel) < 1 && abs(obj.yVel) < 1 && !obj.stuck)
                g.at("oPlayer1/step#8", 28);
                if and(and(({ let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::xVel)?]; b::abs(g, cx, a)? }).gml_lt(&r(1.0))?, ({ let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::yVel)?]; b::abs(g, cx, a)? }).gml_lt(&r(1.0))?), g.get_on(cx, &g.get(cx.this, v::obj)?, v::stuck)?.not()?.is_truthy()) {
                    // 30: global.arrows += 1;
                    g.at("oPlayer1/step#8", 30);
                    g.gset(gv::arrows, lenient(Value::add, g.gget(gv::arrows)?, &r(1.0)))?;
                    // 31: playSound(global.sndPickup);
                    g.at("oPlayer1/step#8", 31);
                    { let a = [g.gget(gv::sndPickup)?]; scr::playSound(g, cx, &a)? };
                    // 32: with (obj)
                    g.at("oPlayer1/step#8", 32);
                    {
                        let t = g.get(cx.this, v::obj)?;
                        let mut it1 = g.with_iter(cx, &t)?;
                        let outer3 = (cx.this, cx.other);
                        'with1: while let Some(h2) = it1.next(g) {
                            (cx.this, cx.other) = (h2, outer3.0);
                            // 32: instance_destroy();
                            g.at("oPlayer1/step#8", 32);
                            b::instance_destroy(g, cx, [])?;
                        }
                        (cx.this, cx.other) = outer3;
                    }
                }
            }
        }
    }
    // 38: if (collision_rectangle(x - 8, y - 8, x + 8, y + 8, oTreasure, 0, 0) && !dead && !stunned)
    g.at("oPlayer1/step#8", 38);
    if and(and(({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oTreasure, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy(), g.get(cx.this, v::dead)?.not()?.is_truthy()), g.get(cx.this, v::stunned)?.not()?.is_truthy()) {
        // 40: gem = instance_nearest(x, y, oTreasure);
        g.at("oPlayer1/step#8", 40);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oTreasure]; b::instance_nearest(g, cx, a)? };
            g.set(cx.this, v::gem, x)?;
        }
        // 41: if (gem.canCollect)
        g.at("oPlayer1/step#8", 41);
        if g.get_on(cx, &g.get(cx.this, v::gem)?, v::canCollect)?.is_truthy() {
            // 44: global.collect += gem.value + ceil(gem.value / 4) * global.levelType;
            g.at("oPlayer1/step#8", 44);
            {
                let old = g.gget(gv::collect)?;
                let x = lenient(Value::add, old, &g.get_on(cx, &g.get(cx.this, v::gem)?, v::value)?.add(&({ let a = [g.get_on(cx, &g.get(cx.this, v::gem)?, v::value)?.div(&r(4.0))?]; b::ceil(g, cx, a)? }).mul(&g.gget(gv::levelType)?)?)?);
                g.gset(gv::collect, x)?;
            }
            // 45: global.collectCounter += 20;
            g.at("oPlayer1/step#8", 45);
            g.gset(gv::collectCounter, lenient(Value::add, g.gget(gv::collectCounter)?, &r(20.0)))?;
            // 46: if (global.collectCounter > 100)
            g.at("oPlayer1/step#8", 46);
            if g.gget(gv::collectCounter)?.gml_gt(&r(100.0))? {
                // 46: global.collectCounter = 100;
                g.at("oPlayer1/step#8", 46);
                g.gset(gv::collectCounter, r(100.0))?;
            }
            // 48: coin = false;
            g.at("oPlayer1/step#8", 48);
            g.set(cx.this, v::coin, r(0.0))?;
            // 50: if (gem.type == "Gold Chunk")
            g.at("oPlayer1/step#8", 50);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Gold Chunk"))? {
                // 50: global.gold += 1;
                g.at("oPlayer1/step#8", 50);
                g.gset(gv::gold, lenient(Value::add, g.gget(gv::gold)?, &r(1.0)))?;
                // 50: coin = true;
                g.at("oPlayer1/step#8", 50);
                g.set(cx.this, v::coin, r(1.0))?;
            }
            // 51: if (gem.type == "Gold Nugget")
            g.at("oPlayer1/step#8", 51);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Gold Nugget"))? {
                // 51: global.nuggets += 1;
                g.at("oPlayer1/step#8", 51);
                g.gset(gv::nuggets, lenient(Value::add, g.gget(gv::nuggets)?, &r(1.0)))?;
                // 51: coin = true;
                g.at("oPlayer1/step#8", 51);
                g.set(cx.this, v::coin, r(1.0))?;
            }
            // 52: if (gem.type == "Gold Bar")
            g.at("oPlayer1/step#8", 52);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Gold Bar"))? {
                // 52: global.goldbar += 1;
                g.at("oPlayer1/step#8", 52);
                g.gset(gv::goldbar, lenient(Value::add, g.gget(gv::goldbar)?, &r(1.0)))?;
                // 52: coin = true;
                g.at("oPlayer1/step#8", 52);
                g.set(cx.this, v::coin, r(1.0))?;
            }
            // 53: if (gem.type == "Gold Bars")
            g.at("oPlayer1/step#8", 53);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Gold Bars"))? {
                // 53: global.goldbars += 1;
                g.at("oPlayer1/step#8", 53);
                g.gset(gv::goldbars, lenient(Value::add, g.gget(gv::goldbars)?, &r(1.0)))?;
                // 53: coin = true;
                g.at("oPlayer1/step#8", 53);
                g.set(cx.this, v::coin, r(1.0))?;
            }
            // 54: if (gem.type == "Emerald")
            g.at("oPlayer1/step#8", 54);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Emerald"))? {
                // 54: global.emeralds += 1;
                g.at("oPlayer1/step#8", 54);
                g.gset(gv::emeralds, lenient(Value::add, g.gget(gv::emeralds)?, &r(1.0)))?;
            }
            // 55: if (gem.type == "Big Emerald")
            g.at("oPlayer1/step#8", 55);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Big Emerald"))? {
                // 55: global.bigemeralds += 1;
                g.at("oPlayer1/step#8", 55);
                g.gset(gv::bigemeralds, lenient(Value::add, g.gget(gv::bigemeralds)?, &r(1.0)))?;
            }
            // 56: if (gem.type == "Sapphire")
            g.at("oPlayer1/step#8", 56);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Sapphire"))? {
                // 56: global.sapphires += 1;
                g.at("oPlayer1/step#8", 56);
                g.gset(gv::sapphires, lenient(Value::add, g.gget(gv::sapphires)?, &r(1.0)))?;
            }
            // 57: if (gem.type == "Big Sapphire")
            g.at("oPlayer1/step#8", 57);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Big Sapphire"))? {
                // 57: global.bigsapphires += 1;
                g.at("oPlayer1/step#8", 57);
                g.gset(gv::bigsapphires, lenient(Value::add, g.gget(gv::bigsapphires)?, &r(1.0)))?;
            }
            // 58: if (gem.type == "Ruby")
            g.at("oPlayer1/step#8", 58);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Ruby"))? {
                // 58: global.rubies += 1;
                g.at("oPlayer1/step#8", 58);
                g.gset(gv::rubies, lenient(Value::add, g.gget(gv::rubies)?, &r(1.0)))?;
            }
            // 59: if (gem.type == "Big Ruby")
            g.at("oPlayer1/step#8", 59);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Big Ruby"))? {
                // 59: global.bigrubies += 1;
                g.at("oPlayer1/step#8", 59);
                g.gset(gv::bigrubies, lenient(Value::add, g.gget(gv::bigrubies)?, &r(1.0)))?;
            }
            // 60: if (gem.type == "Diamond")
            g.at("oPlayer1/step#8", 60);
            if g.get_on(cx, &g.get(cx.this, v::gem)?, v::r#type)?.gml_eq(&s!("Diamond"))? {
                // 60: global.diamonds += 1;
                g.at("oPlayer1/step#8", 60);
                g.gset(gv::diamonds, lenient(Value::add, g.gget(gv::diamonds)?, &r(1.0)))?;
            }
            // 61: if (coin)
            g.at("oPlayer1/step#8", 61);
            if g.get(cx.this, v::coin)?.is_truthy() {
                // 61: playSound(global.sndCoin);
                g.at("oPlayer1/step#8", 61);
                { let a = [g.gget(gv::sndCoin)?]; scr::playSound(g, cx, &a)? };
            } else {
                // 62: playSound(global.sndGem);
                g.at("oPlayer1/step#8", 62);
                { let a = [g.gget(gv::sndGem)?]; scr::playSound(g, cx, &a)? };
            }
            // 64: with (gem)
            g.at("oPlayer1/step#8", 64);
            {
                let t = g.get(cx.this, v::gem)?;
                let mut it4 = g.with_iter(cx, &t)?;
                let outer6 = (cx.this, cx.other);
                'with2: while let Some(h5) = it4.next(g) {
                    (cx.this, cx.other) = (h5, outer6.0);
                    // 64: instance_destroy();
                    g.at("oPlayer1/step#8", 64);
                    b::instance_destroy(g, cx, [])?;
                }
                (cx.this, cx.other) = outer6;
            }
        }
    }
    // 68: if (collision_rectangle(x - 8, y - 8, x + 8, y + 8, oBombBag, 0, 0) && !dead && !stunned)
    g.at("oPlayer1/step#8", 68);
    if and(and(({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oBombBag, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy(), g.get(cx.this, v::dead)?.not()?.is_truthy()), g.get(cx.this, v::stunned)?.not()?.is_truthy()) {
        // 70: obj = collision_rectangle(x - 8, y - 8, x + 8, y + 8, oBombBag, 0, 0);
        g.at("oPlayer1/step#8", 70);
        {
            let x = { let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oBombBag, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? };
            g.set(cx.this, v::obj, x)?;
        }
        // 71: if (!obj.held && obj.cost == 0 && !collision_point(obj.x, obj.y, oSolid, 0, 0))
        g.at("oPlayer1/step#8", 71);
        if and(and(g.get_on(cx, &g.get(cx.this, v::obj)?, v::held)?.not()?.is_truthy(), g.get_on(cx, &g.get(cx.this, v::obj)?, v::cost)?.gml_eq(&r(0.0))?), ({ let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::obj)?, v::y)?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()) {
            // 73: global.bombs += 3;
            g.at("oPlayer1/step#8", 73);
            g.gset(gv::bombs, lenient(Value::add, g.gget(gv::bombs)?, &r(3.0)))?;
            // 74: disp = instance_create(obj.x, obj.y - 14, oItemsGet);
            g.at("oPlayer1/step#8", 74);
            {
                let x = { let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::obj)?, v::y)?.sub(&r(14.0))?, obj::oItemsGet]; b::instance_create(g, cx, a)? };
                g.set(cx.this, v::disp, x)?;
            }
            // 75: disp.sprite_index = sBombsGet;
            g.at("oPlayer1/step#8", 75);
            { let t = g.get(cx.this, v::disp)?; g.set_on(cx, &t, v::sprite_index, spr::sBombsGet)?; }
            // 76: with (obj)
            g.at("oPlayer1/step#8", 76);
            {
                let t = g.get(cx.this, v::obj)?;
                let mut it7 = g.with_iter(cx, &t)?;
                let outer9 = (cx.this, cx.other);
                'with3: while let Some(h8) = it7.next(g) {
                    (cx.this, cx.other) = (h8, outer9.0);
                    // 76: instance_destroy();
                    g.at("oPlayer1/step#8", 76);
                    b::instance_destroy(g, cx, [])?;
                }
                (cx.this, cx.other) = outer9;
            }
            // 77: playSound(global.sndPickup);
            g.at("oPlayer1/step#8", 77);
            { let a = [g.gget(gv::sndPickup)?]; scr::playSound(g, cx, &a)? };
            // 78: global.message = "YOU GOT 3 MORE BOMBS!";
            g.at("oPlayer1/step#8", 78);
            g.gset(gv::message, s!("YOU GOT 3 MORE BOMBS!"))?;
            // 79: global.message2 = "";
            g.at("oPlayer1/step#8", 79);
            g.gset(gv::message2, s!(""))?;
            // 80: global.messageTimer = 120;
            g.at("oPlayer1/step#8", 80);
            g.gset(gv::messageTimer, r(120.0))?;
        }
    }
    // 84: if (collision_rectangle(x - 8, y - 8, x + 8, y + 8, oBombBox, 0, 0) && !dead && !stunned)
    g.at("oPlayer1/step#8", 84);
    if and(and(({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oBombBox, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy(), g.get(cx.this, v::dead)?.not()?.is_truthy()), g.get(cx.this, v::stunned)?.not()?.is_truthy()) {
        // 86: obj = collision_rectangle(x - 8, y - 8, x + 8, y + 8, oBombBox, 0, 0);
        g.at("oPlayer1/step#8", 86);
        {
            let x = { let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oBombBox, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? };
            g.set(cx.this, v::obj, x)?;
        }
        // 87: if (!obj.held && obj.cost == 0 && !collision_point(obj.x, obj.y, oSolid, 0, 0))
        g.at("oPlayer1/step#8", 87);
        if and(and(g.get_on(cx, &g.get(cx.this, v::obj)?, v::held)?.not()?.is_truthy(), g.get_on(cx, &g.get(cx.this, v::obj)?, v::cost)?.gml_eq(&r(0.0))?), ({ let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::obj)?, v::y)?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()) {
            // 89: global.bombs += 12;
            g.at("oPlayer1/step#8", 89);
            g.gset(gv::bombs, lenient(Value::add, g.gget(gv::bombs)?, &r(12.0)))?;
            // 90: disp = instance_create(obj.x, obj.y - 14, oItemsGet);
            g.at("oPlayer1/step#8", 90);
            {
                let x = { let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::obj)?, v::y)?.sub(&r(14.0))?, obj::oItemsGet]; b::instance_create(g, cx, a)? };
                g.set(cx.this, v::disp, x)?;
            }
            // 91: disp.sprite_index = sBombsGet;
            g.at("oPlayer1/step#8", 91);
            { let t = g.get(cx.this, v::disp)?; g.set_on(cx, &t, v::sprite_index, spr::sBombsGet)?; }
            // 92: with (obj)
            g.at("oPlayer1/step#8", 92);
            {
                let t = g.get(cx.this, v::obj)?;
                let mut it10 = g.with_iter(cx, &t)?;
                let outer12 = (cx.this, cx.other);
                'with4: while let Some(h11) = it10.next(g) {
                    (cx.this, cx.other) = (h11, outer12.0);
                    // 92: instance_destroy();
                    g.at("oPlayer1/step#8", 92);
                    b::instance_destroy(g, cx, [])?;
                }
                (cx.this, cx.other) = outer12;
            }
            // 93: playSound(global.sndPickup);
            g.at("oPlayer1/step#8", 93);
            { let a = [g.gget(gv::sndPickup)?]; scr::playSound(g, cx, &a)? };
            // 94: global.message = "YOU GOT 12 MORE BOMBS!";
            g.at("oPlayer1/step#8", 94);
            g.gset(gv::message, s!("YOU GOT 12 MORE BOMBS!"))?;
            // 95: global.message2 = "";
            g.at("oPlayer1/step#8", 95);
            g.gset(gv::message2, s!(""))?;
            // 96: global.messageTimer = 120;
            g.at("oPlayer1/step#8", 96);
            g.gset(gv::messageTimer, r(120.0))?;
        }
    }
    // 100: if (collision_rectangle(x - 8, y - 8, x + 8, y + 8, oRopePile, 0, 0) && !dead && !stunned)
    g.at("oPlayer1/step#8", 100);
    if and(and(({ let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oRopePile, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? }).is_truthy(), g.get(cx.this, v::dead)?.not()?.is_truthy()), g.get(cx.this, v::stunned)?.not()?.is_truthy()) {
        // 102: obj = collision_rectangle(x - 8, y - 8, x + 8, y + 8, oRopePile, 0, 0);
        g.at("oPlayer1/step#8", 102);
        {
            let x = { let a = [g.get(cx.this, v::x)?.sub(&r(8.0))?, g.get(cx.this, v::y)?.sub(&r(8.0))?, g.get(cx.this, v::x)?.add(&r(8.0))?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oRopePile, r(0.0), r(0.0)]; b::collision_rectangle(g, cx, a)? };
            g.set(cx.this, v::obj, x)?;
        }
        // 103: if (!obj.held && obj.cost == 0 && !collision_point(obj.x, obj.y, oSolid, 0, 0))
        g.at("oPlayer1/step#8", 103);
        if and(and(g.get_on(cx, &g.get(cx.this, v::obj)?, v::held)?.not()?.is_truthy(), g.get_on(cx, &g.get(cx.this, v::obj)?, v::cost)?.gml_eq(&r(0.0))?), ({ let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::obj)?, v::y)?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()) {
            // 105: global.rope += 3;
            g.at("oPlayer1/step#8", 105);
            g.gset(gv::rope, lenient(Value::add, g.gget(gv::rope)?, &r(3.0)))?;
            // 106: disp = instance_create(obj.x, obj.y - 15, oItemsGet);
            g.at("oPlayer1/step#8", 106);
            {
                let x = { let a = [g.get_on(cx, &g.get(cx.this, v::obj)?, v::x)?, g.get_on(cx, &g.get(cx.this, v::obj)?, v::y)?.sub(&r(15.0))?, obj::oItemsGet]; b::instance_create(g, cx, a)? };
                g.set(cx.this, v::disp, x)?;
            }
            // 107: disp.sprite_index = sRopeGet;
            g.at("oPlayer1/step#8", 107);
            { let t = g.get(cx.this, v::disp)?; g.set_on(cx, &t, v::sprite_index, spr::sRopeGet)?; }
            // 108: with (obj)
            g.at("oPlayer1/step#8", 108);
            {
                let t = g.get(cx.this, v::obj)?;
                let mut it13 = g.with_iter(cx, &t)?;
                let outer15 = (cx.this, cx.other);
                'with5: while let Some(h14) = it13.next(g) {
                    (cx.this, cx.other) = (h14, outer15.0);
                    // 108: instance_destroy();
                    g.at("oPlayer1/step#8", 108);
                    b::instance_destroy(g, cx, [])?;
                }
                (cx.this, cx.other) = outer15;
            }
            // 109: playSound(global.sndPickup);
            g.at("oPlayer1/step#8", 109);
            { let a = [g.gget(gv::sndPickup)?]; scr::playSound(g, cx, &a)? };
            // 110: global.message = "YOU GOT 3 MORE ROPES!";
            g.at("oPlayer1/step#8", 110);
            g.gset(gv::message, s!("YOU GOT 3 MORE ROPES!"))?;
            // 111: global.message2 = "";
            g.at("oPlayer1/step#8", 111);
            g.gset(gv::message2, s!(""))?;
            // 112: global.messageTimer = 120;
            g.at("oPlayer1/step#8", 112);
            g.gset(gv::messageTimer, r(120.0))?;
        }
    }
    // 116: if (collision_point(x, y, oExit, 0, 0))
    g.at("oPlayer1/step#8", 116);
    if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oExit, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
        // 118: if (holdItem != 0)
        g.at("oPlayer1/step#8", 118);
        if g.get(cx.this, v::holdItem)?.gml_ne(&r(0.0))? {
            // 120: collect = false;
            g.at("oPlayer1/step#8", 120);
            g.set(cx.this, v::collect, r(0.0))?;
            // 121: if (holdItem.type == "Gold Idol")
            g.at("oPlayer1/step#8", 121);
            if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Gold Idol"))? {
                // 123: if (isRealLevel())
                g.at("oPlayer1/step#8", 123);
                if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                    // 123: global.idolsConverted += 1;
                    g.at("oPlayer1/step#8", 123);
                    g.gset(gv::idolsConverted, lenient(Value::add, g.gget(gv::idolsConverted)?, &r(1.0)))?;
                }
                // 124: global.collect += holdItem.value * (global.levelType + 1);
                g.at("oPlayer1/step#8", 124);
                g.gset(gv::collect, lenient(Value::add, g.gget(gv::collect)?, &g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::value)?.mul(&g.gget(gv::levelType)?.add(&r(1.0))?)?))?;
                // 125: global.collectCounter += 20;
                g.at("oPlayer1/step#8", 125);
                g.gset(gv::collectCounter, lenient(Value::add, g.gget(gv::collectCounter)?, &r(20.0)))?;
                // 126: if (global.collectCounter > 100)
                g.at("oPlayer1/step#8", 126);
                if g.gget(gv::collectCounter)?.gml_gt(&r(100.0))? {
                    // 126: global.collectCounter = 100;
                    g.at("oPlayer1/step#8", 126);
                    g.gset(gv::collectCounter, r(100.0))?;
                }
                // 127: if (holdItem.sprite_index == sCrystalSkull)
                g.at("oPlayer1/step#8", 127);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::sprite_index)?.gml_eq(&spr::sCrystalSkull)? {
                    // 127: global.skulls += 1;
                    g.at("oPlayer1/step#8", 127);
                    g.gset(gv::skulls, lenient(Value::add, g.gget(gv::skulls)?, &r(1.0)))?;
                } else {
                    // 128: global.idols += 1;
                    g.at("oPlayer1/step#8", 128);
                    g.gset(gv::idols, lenient(Value::add, g.gget(gv::idols)?, &r(1.0)))?;
                }
                // 129: playSound(global.sndCoin);
                g.at("oPlayer1/step#8", 129);
                { let a = [g.gget(gv::sndCoin)?]; scr::playSound(g, cx, &a)? };
                // 130: instance_create(x, y - 8, oBigCollect);
                g.at("oPlayer1/step#8", 130);
                { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(8.0))?, obj::oBigCollect]; b::instance_create(g, cx, a)? };
                // 131: with (holdItem)
                g.at("oPlayer1/step#8", 131);
                {
                    let t = g.get(cx.this, v::holdItem)?;
                    let mut it16 = g.with_iter(cx, &t)?;
                    let outer18 = (cx.this, cx.other);
                    'with6: while let Some(h17) = it16.next(g) {
                        (cx.this, cx.other) = (h17, outer18.0);
                        // 131: instance_destroy();
                        g.at("oPlayer1/step#8", 131);
                        b::instance_destroy(g, cx, [])?;
                    }
                    (cx.this, cx.other) = outer18;
                }
                // 132: holdItem = 0;
                g.at("oPlayer1/step#8", 132);
                g.set(cx.this, v::holdItem, r(0.0))?;
            } else {
                // 134: if (holdItem.type == "Damsel")
                g.at("oPlayer1/step#8", 134);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Damsel"))? {
                    // 136: if (holdItem.active && holdItem.hp > 0)
                    g.at("oPlayer1/step#8", 136);
                    if and(g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::active)?.is_truthy(), g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::hp)?.gml_gt(&r(0.0))?) {
                        // 138: if (isRealLevel())
                        g.at("oPlayer1/step#8", 138);
                        if scr::isRealLevel(g, cx, &[])?.is_truthy() {
                            // 138: global.damselsSavedTotal += 1;
                            g.at("oPlayer1/step#8", 138);
                            g.gset(gv::damselsSavedTotal, lenient(Value::add, g.gget(gv::damselsSavedTotal)?, &r(1.0)))?;
                        }
                        // 139: global.damsels += 1;
                        g.at("oPlayer1/step#8", 139);
                        g.gset(gv::damsels, lenient(Value::add, g.gget(gv::damsels)?, &r(1.0)))?;
                        // 140: global.xdamsels += 1;
                        g.at("oPlayer1/step#8", 140);
                        g.gset(gv::xdamsels, lenient(Value::add, g.gget(gv::xdamsels)?, &r(1.0)))?;
                        // 141: door = instance_place(x, y, oExit);
                        g.at("oPlayer1/step#8", 141);
                        {
                            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oExit]; b::instance_place(g, cx, a)? };
                            g.set(cx.this, v::door, x)?;
                        }
                        // 142: holdItem.x = door.x + 8;
                        g.at("oPlayer1/step#8", 142);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            g.set_on(cx, &t, v::x, g.get_on(cx, &g.get(cx.this, v::door)?, v::x)?.add(&r(8.0))?)?;
                        }
                        // 143: holdItem.y = door.y + 8;
                        g.at("oPlayer1/step#8", 143);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            g.set_on(cx, &t, v::y, g.get_on(cx, &g.get(cx.this, v::door)?, v::y)?.add(&r(8.0))?)?;
                        }
                        // 144: with (holdItem)
                        g.at("oPlayer1/step#8", 144);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            let mut it19 = g.with_iter(cx, &t)?;
                            let outer21 = (cx.this, cx.other);
                            'with7: while let Some(h20) = it19.next(g) {
                                (cx.this, cx.other) = (h20, outer21.0);
                                // 146: if (global.isDamsel)
                                g.at("oPlayer1/step#8", 146);
                                if g.gget(gv::isDamsel)?.is_truthy() {
                                    // 146: sprite_index = sPExit;
                                    g.at("oPlayer1/step#8", 146);
                                    g.set(cx.this, v::sprite_index, spr::sPExit)?;
                                } else {
                                    // 147: sprite_index = sDamselExit2;
                                    g.at("oPlayer1/step#8", 147);
                                    g.set(cx.this, v::sprite_index, spr::sDamselExit2)?;
                                }
                                // 148: status = 4;
                                g.at("oPlayer1/step#8", 148);
                                g.set(cx.this, v::status, r(4.0))?;
                                // 149: held = false;
                                g.at("oPlayer1/step#8", 149);
                                g.set(cx.this, v::held, r(0.0))?;
                                // 150: xVel = 0;
                                g.at("oPlayer1/step#8", 150);
                                g.set(cx.this, v::xVel, r(0.0))?;
                                // 151: yVel = 0;
                                g.at("oPlayer1/step#8", 151);
                                g.set(cx.this, v::yVel, r(0.0))?;
                                // 152: playSound(global.sndSteps);
                                g.at("oPlayer1/step#8", 152);
                                { let a = [g.gget(gv::sndSteps)?]; scr::playSound(g, cx, &a)? };
                                // 153: depth = 1000;
                                g.at("oPlayer1/step#8", 153);
                                g.set(cx.this, v::depth, r(1000.0))?;
                                // 154: active = false;
                                g.at("oPlayer1/step#8", 154);
                                g.set(cx.this, v::active, r(0.0))?;
                                // 155: canPickUp = false;
                                g.at("oPlayer1/step#8", 155);
                                g.set(cx.this, v::canPickUp, r(0.0))?;
                            }
                            (cx.this, cx.other) = outer21;
                        }
                        // 157: holdItem = 0;
                        g.at("oPlayer1/step#8", 157);
                        g.set(cx.this, v::holdItem, r(0.0))?;
                    }
                }
            }
        }
    }
    // 163: global.xmoney += global.money - money;
    g.at("oPlayer1/step#8", 163);
    g.gset(gv::xmoney, lenient(Value::add, g.gget(gv::xmoney)?, &g.gget(gv::money)?.sub(&g.get(cx.this, v::money)?)?))?;
    Ok(())
}

/// Event `step` of `oPlayer1`: its actions in order.
pub fn step(g: &mut World, cx: &mut Ctx) -> R<()> {
    step_a0(g, cx)?;
    if g.pending().is_some() { return Ok(()); } step_a2(g, cx)?;
    if g.pending().is_some() { return Ok(()); } step_a4(g, cx)?;
    if g.pending().is_some() { return Ok(()); } step_a5(g, cx)?;
    if g.pending().is_some() { return Ok(()); } step_a6(g, cx)?;
    if g.pending().is_some() { return Ok(()); } step_a8(g, cx)?;
    Ok(())
}

