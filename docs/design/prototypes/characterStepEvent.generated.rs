// Output of the transpiler prototype (crates/gml/examples/transpile), unedited. Regenerate with:
//   scripts/cargo run -p gml --example transpile -- scripts/characterStepEvent
// EXCERPT: the first 220 of 3,618 lines (about GML lines 1-118 of 1,031).

/// GML: `scripts/characterStepEvent`.
pub fn characterStepEvent(g: &mut Game, cx: &mut Ctx, args: &[Value]) -> R<Value> {
    let cx = &mut cx.call(args);
    // 6: hangCountMax = 3;
    g.at("scripts/characterStepEvent", 6);
    {
        let x = r(3.0);
        cx.ret = x.clone();
        g.set(cx.this, v::hangCountMax, x)?;
    }
    // 12: kLeft = checkLeft();
    g.at("scripts/characterStepEvent", 12);
    {
        let x = scr::checkLeft(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kLeft, x)?;
    }
    // 14: if (kLeft)
    g.at("scripts/characterStepEvent", 14);
    if truthy(&g.get(cx.this, v::kLeft)?) {
        // 14: kLeftPushedSteps += 1;
        g.at("scripts/characterStepEvent", 14);
        {
            let old = g.get(cx.this, v::kLeftPushedSteps)?;
            let x = lenient(add, old, r(1.0));
            cx.ret = x.clone();
            g.set(cx.this, v::kLeftPushedSteps, x)?;
        }
    } else {
        // 15: kLeftPushedSteps = 0;
        g.at("scripts/characterStepEvent", 15);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kLeftPushedSteps, x)?;
        }
    }
    // 17: kLeftPressed = checkLeftPressed();
    g.at("scripts/characterStepEvent", 17);
    {
        let x = scr::checkLeftPressed(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kLeftPressed, x)?;
    }
    // 18: kLeftReleased = checkLeftReleased();
    g.at("scripts/characterStepEvent", 18);
    {
        let x = scr::checkLeftReleased(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kLeftReleased, x)?;
    }
    // 20: kRight = checkRight();
    g.at("scripts/characterStepEvent", 20);
    {
        let x = scr::checkRight(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kRight, x)?;
    }
    // 22: if (kRight)
    g.at("scripts/characterStepEvent", 22);
    if truthy(&g.get(cx.this, v::kRight)?) {
        // 22: kRightPushedSteps += 1;
        g.at("scripts/characterStepEvent", 22);
        {
            let old = g.get(cx.this, v::kRightPushedSteps)?;
            let x = lenient(add, old, r(1.0));
            cx.ret = x.clone();
            g.set(cx.this, v::kRightPushedSteps, x)?;
        }
    } else {
        // 23: kRightPushedSteps = 0;
        g.at("scripts/characterStepEvent", 23);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kRightPushedSteps, x)?;
        }
    }
    // 25: kRightPressed = checkRightPressed();
    g.at("scripts/characterStepEvent", 25);
    {
        let x = scr::checkRightPressed(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kRightPressed, x)?;
    }
    // 26: kRightReleased = checkRightReleased();
    g.at("scripts/characterStepEvent", 26);
    {
        let x = scr::checkRightReleased(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kRightReleased, x)?;
    }
    // 28: kUp = checkUp();
    g.at("scripts/characterStepEvent", 28);
    {
        let x = scr::checkUp(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kUp, x)?;
    }
    // 29: kDown = checkDown();
    g.at("scripts/characterStepEvent", 29);
    {
        let x = scr::checkDown(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kDown, x)?;
    }
    // 32: if (canRun)
    g.at("scripts/characterStepEvent", 32);
    if truthy(&g.get(cx.this, v::canRun)?) {
        // 33: kRun = 0;
        g.at("scripts/characterStepEvent", 33);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kRun, x)?;
        }
    } else {
        // 36: kRun = 0;
        g.at("scripts/characterStepEvent", 36);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kRun, x)?;
        }
    }
    // 38: kJump = checkJump();
    g.at("scripts/characterStepEvent", 38);
    {
        let x = scr::checkJump(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kJump, x)?;
    }
    // 39: kJumpPressed = checkJumpPressed();
    g.at("scripts/characterStepEvent", 39);
    {
        let x = scr::checkJumpPressed(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kJumpPressed, x)?;
    }
    // 40: kJumpReleased = checkJumpReleased();
    g.at("scripts/characterStepEvent", 40);
    {
        let x = scr::checkJumpReleased(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kJumpReleased, x)?;
    }
    // 42: if (cantJump > 0)
    g.at("scripts/characterStepEvent", 42);
    if gt(&g.get(cx.this, v::cantJump)?, &r(0.0))? {
        // 44: kJump = 0;
        g.at("scripts/characterStepEvent", 44);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kJump, x)?;
        }
        // 45: kJumpPressed = 0;
        g.at("scripts/characterStepEvent", 45);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kJumpPressed, x)?;
        }
        // 46: kJumpReleased = 0;
        g.at("scripts/characterStepEvent", 46);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kJumpReleased, x)?;
        }
        // 47: cantJump -= 1;
        g.at("scripts/characterStepEvent", 47);
        {
            let old = g.get(cx.this, v::cantJump)?;
            let x = lenient(sub, old, r(1.0));
            cx.ret = x.clone();
            g.set(cx.this, v::cantJump, x)?;
        }
    } else {
        // 51: if (global.isTunnelMan && sprite_index == sTunnelAttackL && !holdItem)
        g.at("scripts/characterStepEvent", 51);
        if and(and(truthy(&g.gget(gv::isTunnelMan)?), eq(&g.get(cx.this, v::sprite_index)?, &spr::sTunnelAttackL)?), !truthy(&g.get(cx.this, v::holdItem)?)) {
            // 55: kJump = 0;
            g.at("scripts/characterStepEvent", 55);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::kJump, x)?;
            }
            // 56: kJumpPressed = 0;
            g.at("scripts/characterStepEvent", 56);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::kJumpPressed, x)?;
            }
            // 57: kJumpReleased = 0;
            g.at("scripts/characterStepEvent", 57);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::kJumpReleased, x)?;
            }
            // 58: cantJump -= 1;
            g.at("scripts/characterStepEvent", 58);
            {
                let old = g.get(cx.this, v::cantJump)?;
                let x = lenient(sub, old, r(1.0));
                cx.ret = x.clone();
                g.set(cx.this, v::cantJump, x)?;
            }
        }
    }
    // 62: kAttack = checkAttack();
    g.at("scripts/characterStepEvent", 62);
    {
        let x = scr::checkAttack(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kAttack, x)?;
    }
    // ... (excerpt ends)
