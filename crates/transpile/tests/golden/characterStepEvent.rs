/// GML: `scripts/characterStepEvent`.
pub fn characterStepEvent(g: &mut World, cx: &Ctx, args: &[Value]) -> R<Value> {
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
    if g.get(cx.this, v::kLeft)?.is_truthy() {
        // 14: kLeftPushedSteps += 1;
        g.at("scripts/characterStepEvent", 14);
        {
            let old = g.get(cx.this, v::kLeftPushedSteps)?;
            let x = lenient(Value::add, old, &r(1.0));
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
    if g.get(cx.this, v::kRight)?.is_truthy() {
        // 22: kRightPushedSteps += 1;
        g.at("scripts/characterStepEvent", 22);
        {
            let old = g.get(cx.this, v::kRightPushedSteps)?;
            let x = lenient(Value::add, old, &r(1.0));
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
    if g.get(cx.this, v::canRun)?.is_truthy() {
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
    if g.get(cx.this, v::cantJump)?.gml_gt(&r(0.0))? {
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
            let x = lenient(Value::sub, old, &r(1.0));
            cx.ret = x.clone();
            g.set(cx.this, v::cantJump, x)?;
        }
    } else {
        // 51: if (global.isTunnelMan && sprite_index == sTunnelAttackL && !holdItem)
        g.at("scripts/characterStepEvent", 51);
        if and(and(g.gget(gv::isTunnelMan)?.is_truthy(), g.get(cx.this, v::sprite_index)?.gml_eq(&spr::sTunnelAttackL)?), g.get(cx.this, v::holdItem)?.not()?.is_truthy()) {
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
                let x = lenient(Value::sub, old, &r(1.0));
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
    // 63: kAttackPressed = checkAttackPressed();
    g.at("scripts/characterStepEvent", 63);
    {
        let x = scr::checkAttackPressed(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kAttackPressed, x)?;
    }
    // 64: kAttackReleased = checkAttackReleased();
    g.at("scripts/characterStepEvent", 64);
    {
        let x = scr::checkAttackReleased(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kAttackReleased, x)?;
    }
    // 66: kItemPressed = checkItemPressed();
    g.at("scripts/characterStepEvent", 66);
    {
        let x = scr::checkItemPressed(g, cx, &[])?;
        cx.ret = x.clone();
        g.set(cx.this, v::kItemPressed, x)?;
    }
    // 68: xPrev = x;
    g.at("scripts/characterStepEvent", 68);
    {
        let x = g.get(cx.this, v::x)?;
        cx.ret = x.clone();
        g.set(cx.this, v::xPrev, x)?;
    }
    // 69: yPrev = y;
    g.at("scripts/characterStepEvent", 69);
    {
        let x = g.get(cx.this, v::y)?;
        cx.ret = x.clone();
        g.set(cx.this, v::yPrev, x)?;
    }
    // 71: if (stunned || dead)
    g.at("scripts/characterStepEvent", 71);
    if or(g.get(cx.this, v::stunned)?.is_truthy(), g.get(cx.this, v::dead)?.is_truthy()) {
        // 73: kLeft = false;
        g.at("scripts/characterStepEvent", 73);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kLeft, x)?;
        }
        // 74: kLeftPressed = false;
        g.at("scripts/characterStepEvent", 74);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kLeftPressed, x)?;
        }
        // 75: kLeftReleased = false;
        g.at("scripts/characterStepEvent", 75);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kLeftReleased, x)?;
        }
        // 76: kRight = false;
        g.at("scripts/characterStepEvent", 76);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kRight, x)?;
        }
        // 77: kRightPressed = false;
        g.at("scripts/characterStepEvent", 77);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kRightPressed, x)?;
        }
        // 78: kRightReleased = false;
        g.at("scripts/characterStepEvent", 78);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kRightReleased, x)?;
        }
        // 79: kUp = false;
        g.at("scripts/characterStepEvent", 79);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kUp, x)?;
        }
        // 80: kDown = false;
        g.at("scripts/characterStepEvent", 80);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kDown, x)?;
        }
        // 81: kJump = false;
        g.at("scripts/characterStepEvent", 81);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kJump, x)?;
        }
        // 82: kJumpPressed = false;
        g.at("scripts/characterStepEvent", 82);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kJumpPressed, x)?;
        }
        // 83: kJumpReleased = false;
        g.at("scripts/characterStepEvent", 83);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kJumpReleased, x)?;
        }
        // 84: kAttack = false;
        g.at("scripts/characterStepEvent", 84);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kAttack, x)?;
        }
        // 85: kAttackPressed = false;
        g.at("scripts/characterStepEvent", 85);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kAttackPressed, x)?;
        }
        // 86: kAttackReleased = false;
        g.at("scripts/characterStepEvent", 86);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kAttackReleased, x)?;
        }
        // 87: kItemPressed = false;
        g.at("scripts/characterStepEvent", 87);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kItemPressed, x)?;
        }
    }
    // 94: colSolidLeft = false;
    g.at("scripts/characterStepEvent", 94);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colSolidLeft, x)?;
    }
    // 95: colSolidRight = false;
    g.at("scripts/characterStepEvent", 95);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colSolidRight, x)?;
    }
    // 96: colLeft = false;
    g.at("scripts/characterStepEvent", 96);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colLeft, x)?;
    }
    // 97: colRight = false;
    g.at("scripts/characterStepEvent", 97);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colRight, x)?;
    }
    // 98: colTop = false;
    g.at("scripts/characterStepEvent", 98);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colTop, x)?;
    }
    // 99: colBot = false;
    g.at("scripts/characterStepEvent", 99);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colBot, x)?;
    }
    // 100: colLadder = false;
    g.at("scripts/characterStepEvent", 100);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colLadder, x)?;
    }
    // 101: colPlatBot = false;
    g.at("scripts/characterStepEvent", 101);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colPlatBot, x)?;
    }
    // 102: colPlat = false;
    g.at("scripts/characterStepEvent", 102);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colPlat, x)?;
    }
    // 103: colWaterTop = false;
    g.at("scripts/characterStepEvent", 103);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colWaterTop, x)?;
    }
    // 104: colIceBot = false;
    g.at("scripts/characterStepEvent", 104);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::colIceBot, x)?;
    }
    // 105: runKey = false;
    g.at("scripts/characterStepEvent", 105);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::runKey, x)?;
    }
    // 106: if (isCollisionMoveableSolidLeft(1))
    g.at("scripts/characterStepEvent", 106);
    if scr::isCollisionMoveableSolidLeft(g, cx, &[r(1.0)])?.is_truthy() {
        // 106: colSolidLeft = true;
        g.at("scripts/characterStepEvent", 106);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colSolidLeft, x)?;
        }
    }
    // 107: if (isCollisionMoveableSolidRight(1))
    g.at("scripts/characterStepEvent", 107);
    if scr::isCollisionMoveableSolidRight(g, cx, &[r(1.0)])?.is_truthy() {
        // 107: colSolidRight = true;
        g.at("scripts/characterStepEvent", 107);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colSolidRight, x)?;
        }
    }
    // 108: if (isCollisionLeft(1))
    g.at("scripts/characterStepEvent", 108);
    if scr::isCollisionLeft(g, cx, &[r(1.0)])?.is_truthy() {
        // 108: colLeft = true;
        g.at("scripts/characterStepEvent", 108);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colLeft, x)?;
        }
    }
    // 109: if (isCollisionRight(1))
    g.at("scripts/characterStepEvent", 109);
    if scr::isCollisionRight(g, cx, &[r(1.0)])?.is_truthy() {
        // 109: colRight = true;
        g.at("scripts/characterStepEvent", 109);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colRight, x)?;
        }
    }
    // 110: if (isCollisionTop(1))
    g.at("scripts/characterStepEvent", 110);
    if scr::isCollisionTop(g, cx, &[r(1.0)])?.is_truthy() {
        // 110: colTop = true;
        g.at("scripts/characterStepEvent", 110);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colTop, x)?;
        }
    }
    // 111: if (isCollisionBottom(1))
    g.at("scripts/characterStepEvent", 111);
    if scr::isCollisionBottom(g, cx, &[r(1.0)])?.is_truthy() {
        // 111: colBot = true;
        g.at("scripts/characterStepEvent", 111);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colBot, x)?;
        }
    }
    // 112: if (isCollisionLadder())
    g.at("scripts/characterStepEvent", 112);
    if scr::isCollisionLadder(g, cx, &[])?.is_truthy() {
        // 112: colLadder = true;
        g.at("scripts/characterStepEvent", 112);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colLadder, x)?;
        }
    }
    // 113: if (isCollisionPlatformBottom(1))
    g.at("scripts/characterStepEvent", 113);
    if scr::isCollisionPlatformBottom(g, cx, &[r(1.0)])?.is_truthy() {
        // 113: colPlatBot = true;
        g.at("scripts/characterStepEvent", 113);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colPlatBot, x)?;
        }
    }
    // 114: if (isCollisionPlatform())
    g.at("scripts/characterStepEvent", 114);
    if scr::isCollisionPlatform(g, cx, &[])?.is_truthy() {
        // 114: colPlat = true;
        g.at("scripts/characterStepEvent", 114);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colPlat, x)?;
        }
    }
    // 115: if (isCollisionWaterTop(1))
    g.at("scripts/characterStepEvent", 115);
    if scr::isCollisionWaterTop(g, cx, &[r(1.0)])?.is_truthy() {
        // 115: colWaterTop = true;
        g.at("scripts/characterStepEvent", 115);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colWaterTop, x)?;
        }
    }
    // 116: if (collision_point(x, y + 8, oIce, 0, 0))
    g.at("scripts/characterStepEvent", 116);
    if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oIce, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
        // 116: colIceBot = true;
        g.at("scripts/characterStepEvent", 116);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::colIceBot, x)?;
        }
    }
    // 117: if (checkRun())
    g.at("scripts/characterStepEvent", 117);
    if scr::checkRun(g, cx, &[])?.is_truthy() {
        // 119: runHeld = 100;
        g.at("scripts/characterStepEvent", 119);
        {
            let x = r(100.0);
            cx.ret = x.clone();
            g.set(cx.this, v::runHeld, x)?;
        }
        // 120: runKey = true;
        g.at("scripts/characterStepEvent", 120);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::runKey, x)?;
        }
    }
    // 123: if (checkAttack() && !whipping)
    g.at("scripts/characterStepEvent", 123);
    if and(scr::checkAttack(g, cx, &[])?.is_truthy(), g.get(cx.this, v::whipping)?.not()?.is_truthy()) {
        // 125: runHeld += 1;
        g.at("scripts/characterStepEvent", 125);
        {
            let old = g.get(cx.this, v::runHeld)?;
            let x = lenient(Value::add, old, &r(1.0));
            cx.ret = x.clone();
            g.set(cx.this, v::runHeld, x)?;
        }
        // 126: runKey = true;
        g.at("scripts/characterStepEvent", 126);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::runKey, x)?;
        }
    }
    // 129: if (!runKey || (!kLeft && !kRight))
    g.at("scripts/characterStepEvent", 129);
    if or(g.get(cx.this, v::runKey)?.not()?.is_truthy(), and(g.get(cx.this, v::kLeft)?.not()?.is_truthy(), g.get(cx.this, v::kRight)?.not()?.is_truthy())) {
        // 129: runHeld = 0;
        g.at("scripts/characterStepEvent", 129);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::runHeld, x)?;
        }
    }
    // 133: if (state != CLIMBING && state != HANGING)
    g.at("scripts/characterStepEvent", 133);
    if and(g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::CLIMBING)?)?, g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::HANGING)?)?) {
        // 135: if (kLeftReleased && approximatelyZero(xVel))
        g.at("scripts/characterStepEvent", 135);
        if and(g.get(cx.this, v::kLeftReleased)?.is_truthy(), ({ let a = [g.get(cx.this, v::xVel)?]; scr::approximatelyZero(g, cx, &a)? }).is_truthy()) {
            // 135: xAcc -= 0.5;
            g.at("scripts/characterStepEvent", 135);
            {
                let old = g.get(cx.this, v::xAcc)?;
                let x = lenient(Value::sub, old, &r(0.5));
                cx.ret = x.clone();
                g.set(cx.this, v::xAcc, x)?;
            }
        }
        // 136: if (kRightReleased && approximatelyZero(xVel))
        g.at("scripts/characterStepEvent", 136);
        if and(g.get(cx.this, v::kRightReleased)?.is_truthy(), ({ let a = [g.get(cx.this, v::xVel)?]; scr::approximatelyZero(g, cx, &a)? }).is_truthy()) {
            // 136: xAcc += 0.5;
            g.at("scripts/characterStepEvent", 136);
            {
                let old = g.get(cx.this, v::xAcc)?;
                let x = lenient(Value::add, old, &r(0.5));
                cx.ret = x.clone();
                g.set(cx.this, v::xAcc, x)?;
            }
        }
        // 138: if (kLeft && !kRight)
        g.at("scripts/characterStepEvent", 138);
        if and(g.get(cx.this, v::kLeft)?.is_truthy(), g.get(cx.this, v::kRight)?.not()?.is_truthy()) {
            // 140: if (colSolidLeft)
            g.at("scripts/characterStepEvent", 140);
            if g.get(cx.this, v::colSolidLeft)?.is_truthy() {
                // 143: if (platformCharacterIs(ON_GROUND) && state != DUCKING)
                g.at("scripts/characterStepEvent", 143);
                if and(({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy(), g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::DUCKING)?)?) {
                    // 145: xAcc -= 1;
                    g.at("scripts/characterStepEvent", 145);
                    {
                        let old = g.get(cx.this, v::xAcc)?;
                        let x = lenient(Value::sub, old, &r(1.0));
                        cx.ret = x.clone();
                        g.set(cx.this, v::xAcc, x)?;
                    }
                    // 146: pushTimer += 10;
                    g.at("scripts/characterStepEvent", 146);
                    {
                        let old = g.get(cx.this, v::pushTimer)?;
                        let x = lenient(Value::add, old, &r(10.0));
                        cx.ret = x.clone();
                        g.set(cx.this, v::pushTimer, x)?;
                    }
                }
            } else {
                // 150: if (kLeftPushedSteps > 2 && (facing == LEFT || approximatelyZero(xVel)))
                g.at("scripts/characterStepEvent", 150);
                if and(g.get(cx.this, v::kLeftPushedSteps)?.gml_gt(&r(2.0))?, or(g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)?, ({ let a = [g.get(cx.this, v::xVel)?]; scr::approximatelyZero(g, cx, &a)? }).is_truthy())) {
                    // 152: xAcc -= runAcc;
                    g.at("scripts/characterStepEvent", 152);
                    {
                        let old = g.get(cx.this, v::xAcc)?;
                        let x = lenient(Value::sub, old, &g.get(cx.this, v::runAcc)?);
                        cx.ret = x.clone();
                        g.set(cx.this, v::xAcc, x)?;
                    }
                }
            }
            // 154: facing = LEFT;
            g.at("scripts/characterStepEvent", 154);
            {
                let x = g.get(cx.this, v::LEFT)?;
                cx.ret = x.clone();
                g.set(cx.this, v::facing, x)?;
            }
        }
        // 158: if (kRight && !kLeft)
        g.at("scripts/characterStepEvent", 158);
        if and(g.get(cx.this, v::kRight)?.is_truthy(), g.get(cx.this, v::kLeft)?.not()?.is_truthy()) {
            // 160: if (colSolidRight)
            g.at("scripts/characterStepEvent", 160);
            if g.get(cx.this, v::colSolidRight)?.is_truthy() {
                // 163: if (platformCharacterIs(ON_GROUND) && state != DUCKING)
                g.at("scripts/characterStepEvent", 163);
                if and(({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy(), g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::DUCKING)?)?) {
                    // 165: xAcc += 1;
                    g.at("scripts/characterStepEvent", 165);
                    {
                        let old = g.get(cx.this, v::xAcc)?;
                        let x = lenient(Value::add, old, &r(1.0));
                        cx.ret = x.clone();
                        g.set(cx.this, v::xAcc, x)?;
                    }
                    // 166: pushTimer += 10;
                    g.at("scripts/characterStepEvent", 166);
                    {
                        let old = g.get(cx.this, v::pushTimer)?;
                        let x = lenient(Value::add, old, &r(10.0));
                        cx.ret = x.clone();
                        g.set(cx.this, v::pushTimer, x)?;
                    }
                }
            } else {
                // 170: if (kRightPushedSteps > 2 || colSolidLeft && (facing == RIGHT || approximatelyZero(xVel)))
                g.at("scripts/characterStepEvent", 170);
                if and(or(g.get(cx.this, v::kRightPushedSteps)?.gml_gt(&r(2.0))?, g.get(cx.this, v::colSolidLeft)?.is_truthy()), or(g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)?, ({ let a = [g.get(cx.this, v::xVel)?]; scr::approximatelyZero(g, cx, &a)? }).is_truthy())) {
                    // 172: xAcc += runAcc;
                    g.at("scripts/characterStepEvent", 172);
                    {
                        let old = g.get(cx.this, v::xAcc)?;
                        let x = lenient(Value::add, old, &g.get(cx.this, v::runAcc)?);
                        cx.ret = x.clone();
                        g.set(cx.this, v::xAcc, x)?;
                    }
                }
            }
            // 174: facing = RIGHT;
            g.at("scripts/characterStepEvent", 174);
            {
                let x = g.get(cx.this, v::RIGHT)?;
                cx.ret = x.clone();
                g.set(cx.this, v::facing, x)?;
            }
        }
    }
    // 185: if (state == CLIMBING)
    g.at("scripts/characterStepEvent", 185);
    if g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::CLIMBING)?)? {
        // 187: if (instance_exists(oCape))
        g.at("scripts/characterStepEvent", 187);
        if b::instance_exists(g, cx, [obj::oCape])?.is_truthy() {
            // 189: oCape.open = false;
            g.at("scripts/characterStepEvent", 189);
            {
                let t = obj::oCape;
                let x = r(0.0);
                cx.ret = x.clone();
                g.set_on(cx, &t, v::open, x)?;
            }
        }
        // 191: kJumped = false;
        g.at("scripts/characterStepEvent", 191);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kJumped, x)?;
        }
        // 192: ladderTimer = 10;
        g.at("scripts/characterStepEvent", 192);
        {
            let x = r(10.0);
            cx.ret = x.clone();
            g.set(cx.this, v::ladderTimer, x)?;
        }
        // 193: ladder = collision_point(x, y, oLadder, 0, 0);
        g.at("scripts/characterStepEvent", 193);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLadder, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? };
            cx.ret = x.clone();
            g.set(cx.this, v::ladder, x)?;
        }
        // 194: if (ladder)
        g.at("scripts/characterStepEvent", 194);
        if g.get(cx.this, v::ladder)?.is_truthy() {
            // 194: x = ladder.x + 8;
            g.at("scripts/characterStepEvent", 194);
            {
                let x = g.get_on(cx, &g.get(cx.this, v::ladder)?, v::x)?.add(&r(8.0))?;
                cx.ret = x.clone();
                g.set(cx.this, v::x, x)?;
            }
        }
        // 196: if (kLeft)
        g.at("scripts/characterStepEvent", 196);
        if g.get(cx.this, v::kLeft)?.is_truthy() {
            // 196: facing = LEFT;
            g.at("scripts/characterStepEvent", 196);
            {
                let x = g.get(cx.this, v::LEFT)?;
                cx.ret = x.clone();
                g.set(cx.this, v::facing, x)?;
            }
        } else {
            // 197: if (kRight)
            g.at("scripts/characterStepEvent", 197);
            if g.get(cx.this, v::kRight)?.is_truthy() {
                // 197: facing = RIGHT;
                g.at("scripts/characterStepEvent", 197);
                {
                    let x = g.get(cx.this, v::RIGHT)?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::facing, x)?;
                }
            }
        }
        // 198: if (kUp)
        g.at("scripts/characterStepEvent", 198);
        if g.get(cx.this, v::kUp)?.is_truthy() {
            // 200: if (collision_point(x, y - 8, oLadder, 0, 0) || collision_point(x, y - 8, oLadderTop, 0, 0))
            g.at("scripts/characterStepEvent", 200);
            if or(({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(8.0))?, obj::oLadder, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(8.0))?, obj::oLadderTop, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()) {
                // 202: yAcc -= climbAcc;
                g.at("scripts/characterStepEvent", 202);
                {
                    let old = g.get(cx.this, v::yAcc)?;
                    let x = lenient(Value::sub, old, &g.get(cx.this, v::climbAcc)?);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yAcc, x)?;
                }
                // 203: if (alarm[2] < 1)
                g.at("scripts/characterStepEvent", 203);
                if g.get_a(cx.this, v::alarm, 0, 2)?.gml_lt(&r(1.0))? {
                    // 203: alarm[2] = 8;
                    g.at("scripts/characterStepEvent", 203);
                    {
                        let x = r(8.0);
                        cx.ret = x.clone();
                        g.set_a(cx.this, v::alarm, 0, 2, x)?;
                    }
                }
            }
        } else {
            // 206: if (kDown)
            g.at("scripts/characterStepEvent", 206);
            if g.get(cx.this, v::kDown)?.is_truthy() {
                // 208: if (collision_point(x, y + 8, oLadder, 0, 0) || collision_point(x, y + 8, oLadderTop, 0, 0))
                g.at("scripts/characterStepEvent", 208);
                if or(({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oLadder, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(8.0))?, obj::oLadderTop, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()) {
                    // 210: yAcc += climbAcc;
                    g.at("scripts/characterStepEvent", 210);
                    {
                        let old = g.get(cx.this, v::yAcc)?;
                        let x = lenient(Value::add, old, &g.get(cx.this, v::climbAcc)?);
                        cx.ret = x.clone();
                        g.set(cx.this, v::yAcc, x)?;
                    }
                    // 211: if (alarm[2] < 1)
                    g.at("scripts/characterStepEvent", 211);
                    if g.get_a(cx.this, v::alarm, 0, 2)?.gml_lt(&r(1.0))? {
                        // 211: alarm[2] = 8;
                        g.at("scripts/characterStepEvent", 211);
                        {
                            let x = r(8.0);
                            cx.ret = x.clone();
                            g.set_a(cx.this, v::alarm, 0, 2, x)?;
                        }
                    }
                } else {
                    // 214: state = FALLING;
                    g.at("scripts/characterStepEvent", 214);
                    {
                        let x = g.get(cx.this, v::FALLING)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::state, x)?;
                    }
                }
                // 215: if (colBot)
                g.at("scripts/characterStepEvent", 215);
                if g.get(cx.this, v::colBot)?.is_truthy() {
                    // 215: state = STANDING;
                    g.at("scripts/characterStepEvent", 215);
                    {
                        let x = g.get(cx.this, v::STANDING)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::state, x)?;
                    }
                }
            }
        }
        // 218: if (kJumpPressed && !whipping)
        g.at("scripts/characterStepEvent", 218);
        if and(g.get(cx.this, v::kJumpPressed)?.is_truthy(), g.get(cx.this, v::whipping)?.not()?.is_truthy()) {
            // 220: if (kLeft)
            g.at("scripts/characterStepEvent", 220);
            if g.get(cx.this, v::kLeft)?.is_truthy() {
                // 221: xVel = -departLadderXVel;
                g.at("scripts/characterStepEvent", 221);
                {
                    let x = g.get(cx.this, v::departLadderXVel)?.neg()?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::xVel, x)?;
                }
            } else {
                // 222: if (kRight)
                g.at("scripts/characterStepEvent", 222);
                if g.get(cx.this, v::kRight)?.is_truthy() {
                    // 223: xVel = departLadderXVel;
                    g.at("scripts/characterStepEvent", 223);
                    {
                        let x = g.get(cx.this, v::departLadderXVel)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::xVel, x)?;
                    }
                } else {
                    // 225: xVel = 0;
                    g.at("scripts/characterStepEvent", 225);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::xVel, x)?;
                    }
                }
            }
            // 226: yAcc += departLadderYVel;
            g.at("scripts/characterStepEvent", 226);
            {
                let old = g.get(cx.this, v::yAcc)?;
                let x = lenient(Value::add, old, &g.get(cx.this, v::departLadderYVel)?);
                cx.ret = x.clone();
                g.set(cx.this, v::yAcc, x)?;
            }
            // 227: state = JUMPING;
            g.at("scripts/characterStepEvent", 227);
            {
                let x = g.get(cx.this, v::JUMPING)?;
                cx.ret = x.clone();
                g.set(cx.this, v::state, x)?;
            }
            // 228: jumpButtonReleased = 0;
            g.at("scripts/characterStepEvent", 228);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::jumpButtonReleased, x)?;
            }
            // 229: jumpTime = 0;
            g.at("scripts/characterStepEvent", 229);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::jumpTime, x)?;
            }
            // 230: ladderTimer = 5;
            g.at("scripts/characterStepEvent", 230);
            {
                let x = r(5.0);
                cx.ret = x.clone();
                g.set(cx.this, v::ladderTimer, x)?;
            }
        }
    } else {
        // 235: if (ladderTimer > 0)
        g.at("scripts/characterStepEvent", 235);
        if g.get(cx.this, v::ladderTimer)?.gml_gt(&r(0.0))? {
            // 235: ladderTimer -= 1;
            g.at("scripts/characterStepEvent", 235);
            {
                let old = g.get(cx.this, v::ladderTimer)?;
                let x = lenient(Value::sub, old, &r(1.0));
                cx.ret = x.clone();
                g.set(cx.this, v::ladderTimer, x)?;
            }
        }
    }
    // 238: if (platformCharacterIs(IN_AIR) && state != HANGING)
    g.at("scripts/characterStepEvent", 238);
    if and(({ let a = [g.get(cx.this, v::IN_AIR)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy(), g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::HANGING)?)?) {
        // 240: yAcc += gravityIntensity;
        g.at("scripts/characterStepEvent", 240);
        {
            let old = g.get(cx.this, v::yAcc)?;
            let x = lenient(Value::add, old, &g.get(cx.this, v::gravityIntensity)?);
            cx.ret = x.clone();
            g.set(cx.this, v::yAcc, x)?;
        }
    }
    // 244: if (colBot || colPlatBot && platformCharacterIs(IN_AIR) && yVel >= 0)
    g.at("scripts/characterStepEvent", 244);
    if and(and(or(g.get(cx.this, v::colBot)?.is_truthy(), g.get(cx.this, v::colPlatBot)?.is_truthy()), ({ let a = [g.get(cx.this, v::IN_AIR)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), g.get(cx.this, v::yVel)?.gml_ge(&r(0.0))?) {
        // 246: if (!colPlat || colBot)
        g.at("scripts/characterStepEvent", 246);
        if or(g.get(cx.this, v::colPlat)?.not()?.is_truthy(), g.get(cx.this, v::colBot)?.is_truthy()) {
            // 248: yVel = 0;
            g.at("scripts/characterStepEvent", 248);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::yVel, x)?;
            }
            // 249: yAcc = 0;
            g.at("scripts/characterStepEvent", 249);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::yAcc, x)?;
            }
            // 250: state = RUNNING;
            g.at("scripts/characterStepEvent", 250);
            {
                let x = g.get(cx.this, v::RUNNING)?;
                cx.ret = x.clone();
                g.set(cx.this, v::state, x)?;
            }
            // 251: jumps = 0;
            g.at("scripts/characterStepEvent", 251);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::jumps, x)?;
            }
        }
    }
    // 255: if (colBot || colPlatBot && !colPlat)
    g.at("scripts/characterStepEvent", 255);
    if and(or(g.get(cx.this, v::colBot)?.is_truthy(), g.get(cx.this, v::colPlatBot)?.is_truthy()), g.get(cx.this, v::colPlat)?.not()?.is_truthy()) {
        // 255: yVel = 0;
        g.at("scripts/characterStepEvent", 255);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::yVel, x)?;
        }
    }
    // 258: if (colBot == 0 && (!colPlatBot || colPlat) && platformCharacterIs(ON_GROUND))
    g.at("scripts/characterStepEvent", 258);
    if and(and(g.get(cx.this, v::colBot)?.gml_eq(&r(0.0))?, or(g.get(cx.this, v::colPlatBot)?.not()?.is_truthy(), g.get(cx.this, v::colPlat)?.is_truthy())), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()) {
        // 260: state = FALLING;
        g.at("scripts/characterStepEvent", 260);
        {
            let x = g.get(cx.this, v::FALLING)?;
            cx.ret = x.clone();
            g.set(cx.this, v::state, x)?;
        }
        // 261: yAcc += grav;
        g.at("scripts/characterStepEvent", 261);
        {
            let old = g.get(cx.this, v::yAcc)?;
            let x = lenient(Value::add, old, &g.get(cx.this, v::grav)?);
            cx.ret = x.clone();
            g.set(cx.this, v::yAcc, x)?;
        }
        // 262: kJumped = true;
        g.at("scripts/characterStepEvent", 262);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kJumped, x)?;
        }
        // 263: if (global.hasGloves)
        g.at("scripts/characterStepEvent", 263);
        if g.gget(gv::hasGloves)?.is_truthy() {
            // 263: hangCount = 5;
            g.at("scripts/characterStepEvent", 263);
            {
                let x = r(5.0);
                cx.ret = x.clone();
                g.set(cx.this, v::hangCount, x)?;
            }
        }
    }
    // 266: if (colTop)
    g.at("scripts/characterStepEvent", 266);
    if g.get(cx.this, v::colTop)?.is_truthy() {
        // 268: if (dead || stunned)
        g.at("scripts/characterStepEvent", 268);
        if or(g.get(cx.this, v::dead)?.is_truthy(), g.get(cx.this, v::stunned)?.is_truthy()) {
            // 268: yVel = -yVel * 0.8;
            g.at("scripts/characterStepEvent", 268);
            {
                let x = g.get(cx.this, v::yVel)?.neg()?.mul(&r(0.8))?;
                cx.ret = x.clone();
                g.set(cx.this, v::yVel, x)?;
            }
        } else {
            // 269: if (state == JUMPING)
            g.at("scripts/characterStepEvent", 269);
            if g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::JUMPING)?)? {
                // 269: yVel = abs(yVel * 0.3);
                g.at("scripts/characterStepEvent", 269);
                {
                    let x = { let a = [g.get(cx.this, v::yVel)?.mul(&r(0.3))?]; b::abs(g, cx, a)? };
                    cx.ret = x.clone();
                    g.set(cx.this, v::yVel, x)?;
                }
            }
        }
    }
    // 272: if (colLeft && facing == LEFT || (colRight && facing == RIGHT))
    g.at("scripts/characterStepEvent", 272);
    if or(and(g.get(cx.this, v::colLeft)?.is_truthy(), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)?), and(g.get(cx.this, v::colRight)?.is_truthy(), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)?)) {
        // 274: if (dead || stunned)
        g.at("scripts/characterStepEvent", 274);
        if or(g.get(cx.this, v::dead)?.is_truthy(), g.get(cx.this, v::stunned)?.is_truthy()) {
            // 274: xVel = -xVel * 0.5;
            g.at("scripts/characterStepEvent", 274);
            {
                let x = g.get(cx.this, v::xVel)?.neg()?.mul(&r(0.5))?;
                cx.ret = x.clone();
                g.set(cx.this, v::xVel, x)?;
            }
        } else {
            // 275: xVel = 0;
            g.at("scripts/characterStepEvent", 275);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::xVel, x)?;
            }
        }
    }
    // 284: if (kJumpReleased && platformCharacterIs(IN_AIR))
    g.at("scripts/characterStepEvent", 284);
    if and(g.get(cx.this, v::kJumpReleased)?.is_truthy(), ({ let a = [g.get(cx.this, v::IN_AIR)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()) {
        // 286: kJumped = true;
        g.at("scripts/characterStepEvent", 286);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kJumped, x)?;
        }
    } else {
        // 288: if (platformCharacterIs(ON_GROUND))
        g.at("scripts/characterStepEvent", 288);
        if ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy() {
            // 290: oCape.open = false;
            g.at("scripts/characterStepEvent", 290);
            {
                let t = obj::oCape;
                let x = r(0.0);
                cx.ret = x.clone();
                g.set_on(cx, &t, v::open, x)?;
            }
            // 291: kJumped = false;
            g.at("scripts/characterStepEvent", 291);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::kJumped, x)?;
            }
        }
    }
    // 294: if (kJumpPressed && collision_point(x, y, oWeb, 0, 0))
    g.at("scripts/characterStepEvent", 294);
    if and(g.get(cx.this, v::kJumpPressed)?.is_truthy(), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oWeb, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()) {
        // 296: obj = instance_place(x, y, oWeb);
        g.at("scripts/characterStepEvent", 296);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oWeb]; b::instance_place(g, cx, a)? };
            cx.ret = x.clone();
            g.set(cx.this, v::obj, x)?;
        }
        // 297: obj.life -= 1;
        g.at("scripts/characterStepEvent", 297);
        {
            let t = g.get(cx.this, v::obj)?;
            let old = { let t2 = g.get(cx.this, v::obj)?; g.get_on(cx, &t2, v::life)? };
            let x = lenient(Value::sub, old, &r(1.0));
            cx.ret = x.clone();
            g.set_on(cx, &t, v::life, x)?;
        }
        // 298: yAcc += initialJumpAcc * 2;
        g.at("scripts/characterStepEvent", 298);
        {
            let old = g.get(cx.this, v::yAcc)?;
            let x = lenient(Value::add, old, &g.get(cx.this, v::initialJumpAcc)?.mul(&r(2.0))?);
            cx.ret = x.clone();
            g.set(cx.this, v::yAcc, x)?;
        }
        // 299: yVel -= 3;
        g.at("scripts/characterStepEvent", 299);
        {
            let old = g.get(cx.this, v::yVel)?;
            let x = lenient(Value::sub, old, &r(3.0));
            cx.ret = x.clone();
            g.set(cx.this, v::yVel, x)?;
        }
        // 300: xAcc += xVel / 2;
        g.at("scripts/characterStepEvent", 300);
        {
            let old = g.get(cx.this, v::xAcc)?;
            let x = lenient(Value::add, old, &g.get(cx.this, v::xVel)?.div(&r(2.0))?);
            cx.ret = x.clone();
            g.set(cx.this, v::xAcc, x)?;
        }
        // 302: state = JUMPING;
        g.at("scripts/characterStepEvent", 302);
        {
            let x = g.get(cx.this, v::JUMPING)?;
            cx.ret = x.clone();
            g.set(cx.this, v::state, x)?;
        }
        // 303: jumpButtonReleased = 0;
        g.at("scripts/characterStepEvent", 303);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::jumpButtonReleased, x)?;
        }
        // 304: jumpTime = 0;
        g.at("scripts/characterStepEvent", 304);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::jumpTime, x)?;
        }
        // 306: grav = gravNorm;
        g.at("scripts/characterStepEvent", 306);
        {
            let x = g.get(cx.this, v::gravNorm)?;
            cx.ret = x.clone();
            g.set(cx.this, v::grav, x)?;
        }
    } else {
        // 308: if (kJumpPressed && colWaterTop)
        g.at("scripts/characterStepEvent", 308);
        if and(g.get(cx.this, v::kJumpPressed)?.is_truthy(), g.get(cx.this, v::colWaterTop)?.is_truthy()) {
            // 310: yAcc += initialJumpAcc * 2;
            g.at("scripts/characterStepEvent", 310);
            {
                let old = g.get(cx.this, v::yAcc)?;
                let x = lenient(Value::add, old, &g.get(cx.this, v::initialJumpAcc)?.mul(&r(2.0))?);
                cx.ret = x.clone();
                g.set(cx.this, v::yAcc, x)?;
            }
            // 311: yVel -= 3;
            g.at("scripts/characterStepEvent", 311);
            {
                let old = g.get(cx.this, v::yVel)?;
                let x = lenient(Value::sub, old, &r(3.0));
                cx.ret = x.clone();
                g.set(cx.this, v::yVel, x)?;
            }
            // 312: xAcc += xVel / 2;
            g.at("scripts/characterStepEvent", 312);
            {
                let old = g.get(cx.this, v::xAcc)?;
                let x = lenient(Value::add, old, &g.get(cx.this, v::xVel)?.div(&r(2.0))?);
                cx.ret = x.clone();
                g.set(cx.this, v::xAcc, x)?;
            }
            // 314: state = JUMPING;
            g.at("scripts/characterStepEvent", 314);
            {
                let x = g.get(cx.this, v::JUMPING)?;
                cx.ret = x.clone();
                g.set(cx.this, v::state, x)?;
            }
            // 315: jumpButtonReleased = 0;
            g.at("scripts/characterStepEvent", 315);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::jumpButtonReleased, x)?;
            }
            // 316: jumpTime = 0;
            g.at("scripts/characterStepEvent", 316);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::jumpTime, x)?;
            }
            // 318: grav = gravNorm;
            g.at("scripts/characterStepEvent", 318);
            {
                let x = g.get(cx.this, v::gravNorm)?;
                cx.ret = x.clone();
                g.set(cx.this, v::grav, x)?;
            }
        } else {
            // 320: if (global.hasCape && kJumpPressed && kJumped && platformCharacterIs(IN_AIR))
            g.at("scripts/characterStepEvent", 320);
            if and(and(and(g.gget(gv::hasCape)?.is_truthy(), g.get(cx.this, v::kJumpPressed)?.is_truthy()), g.get(cx.this, v::kJumped)?.is_truthy()), ({ let a = [g.get(cx.this, v::IN_AIR)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()) {
                // 322: if (!oCape.open)
                g.at("scripts/characterStepEvent", 322);
                if g.get_on(cx, &obj::oCape, v::open)?.not()?.is_truthy() {
                    // 322: oCape.open = true;
                    g.at("scripts/characterStepEvent", 322);
                    {
                        let t = obj::oCape;
                        let x = r(1.0);
                        cx.ret = x.clone();
                        g.set_on(cx, &t, v::open, x)?;
                    }
                } else {
                    // 323: oCape.open = false;
                    g.at("scripts/characterStepEvent", 323);
                    {
                        let t = obj::oCape;
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set_on(cx, &t, v::open, x)?;
                    }
                }
            } else {
                // 325: if (global.hasJetpack && kJump && kJumped && platformCharacterIs(IN_AIR) && jetpackFuel > 0)
                g.at("scripts/characterStepEvent", 325);
                if and(and(and(and(g.gget(gv::hasJetpack)?.is_truthy(), g.get(cx.this, v::kJump)?.is_truthy()), g.get(cx.this, v::kJumped)?.is_truthy()), ({ let a = [g.get(cx.this, v::IN_AIR)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), g.get(cx.this, v::jetpackFuel)?.gml_gt(&r(0.0))?) {
                    // 327: yAcc += initialJumpAcc;
                    g.at("scripts/characterStepEvent", 327);
                    {
                        let old = g.get(cx.this, v::yAcc)?;
                        let x = lenient(Value::add, old, &g.get(cx.this, v::initialJumpAcc)?);
                        cx.ret = x.clone();
                        g.set(cx.this, v::yAcc, x)?;
                    }
                    // 328: yVel = -1;
                    g.at("scripts/characterStepEvent", 328);
                    {
                        let x = r(-1.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::yVel, x)?;
                    }
                    // 329: jetpackFuel -= 1;
                    g.at("scripts/characterStepEvent", 329);
                    {
                        let old = g.get(cx.this, v::jetpackFuel)?;
                        let x = lenient(Value::sub, old, &r(1.0));
                        cx.ret = x.clone();
                        g.set(cx.this, v::jetpackFuel, x)?;
                    }
                    // 330: if (alarm[10] < 1)
                    g.at("scripts/characterStepEvent", 330);
                    if g.get_a(cx.this, v::alarm, 0, 10)?.gml_lt(&r(1.0))? {
                        // 330: alarm[10] = 3;
                        g.at("scripts/characterStepEvent", 330);
                        {
                            let x = r(3.0);
                            cx.ret = x.clone();
                            g.set_a(cx.this, v::alarm, 0, 10, x)?;
                        }
                    }
                    // 332: state = JUMPING;
                    g.at("scripts/characterStepEvent", 332);
                    {
                        let x = g.get(cx.this, v::JUMPING)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::state, x)?;
                    }
                    // 333: jumpButtonReleased = 0;
                    g.at("scripts/characterStepEvent", 333);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::jumpButtonReleased, x)?;
                    }
                    // 334: jumpTime = 0;
                    g.at("scripts/characterStepEvent", 334);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::jumpTime, x)?;
                    }
                    // 336: grav = 0;
                    g.at("scripts/characterStepEvent", 336);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::grav, x)?;
                    }
                } else {
                    // 338: if (platformCharacterIs(ON_GROUND) && kJumpPressed && fallTimer == 0)
                    g.at("scripts/characterStepEvent", 338);
                    if and(and(({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy(), g.get(cx.this, v::kJumpPressed)?.is_truthy()), g.get(cx.this, v::fallTimer)?.gml_eq(&r(0.0))?) {
                        // 340: if (xVel > 3 || xVel < -3)
                        g.at("scripts/characterStepEvent", 340);
                        if or(g.get(cx.this, v::xVel)?.gml_gt(&r(3.0))?, g.get(cx.this, v::xVel)?.gml_lt(&r(-3.0))?) {
                            // 342: yAcc += initialJumpAcc * 2;
                            g.at("scripts/characterStepEvent", 342);
                            {
                                let old = g.get(cx.this, v::yAcc)?;
                                let x = lenient(Value::add, old, &g.get(cx.this, v::initialJumpAcc)?.mul(&r(2.0))?);
                                cx.ret = x.clone();
                                g.set(cx.this, v::yAcc, x)?;
                            }
                            // 343: xAcc += xVel * 2;
                            g.at("scripts/characterStepEvent", 343);
                            {
                                let old = g.get(cx.this, v::xAcc)?;
                                let x = lenient(Value::add, old, &g.get(cx.this, v::xVel)?.mul(&r(2.0))?);
                                cx.ret = x.clone();
                                g.set(cx.this, v::xAcc, x)?;
                            }
                        } else {
                            // 347: yAcc += initialJumpAcc * 2;
                            g.at("scripts/characterStepEvent", 347);
                            {
                                let old = g.get(cx.this, v::yAcc)?;
                                let x = lenient(Value::add, old, &g.get(cx.this, v::initialJumpAcc)?.mul(&r(2.0))?);
                                cx.ret = x.clone();
                                g.set(cx.this, v::yAcc, x)?;
                            }
                            // 348: xAcc += xVel / 2;
                            g.at("scripts/characterStepEvent", 348);
                            {
                                let old = g.get(cx.this, v::xAcc)?;
                                let x = lenient(Value::add, old, &g.get(cx.this, v::xVel)?.div(&r(2.0))?);
                                cx.ret = x.clone();
                                g.set(cx.this, v::xAcc, x)?;
                            }
                        }
                        // 351: if (global.hasJordans)
                        g.at("scripts/characterStepEvent", 351);
                        if g.gget(gv::hasJordans)?.is_truthy() {
                            // 353: yAcc *= 3;
                            g.at("scripts/characterStepEvent", 353);
                            {
                                let old = g.get(cx.this, v::yAcc)?;
                                let x = lenient(Value::mul, old, &r(3.0));
                                cx.ret = x.clone();
                                g.set(cx.this, v::yAcc, x)?;
                            }
                            // 354: yAccLimit = 12;
                            g.at("scripts/characterStepEvent", 354);
                            {
                                let x = r(12.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::yAccLimit, x)?;
                            }
                            // 355: grav = 0.5;
                            g.at("scripts/characterStepEvent", 355);
                            {
                                let x = r(0.5);
                                cx.ret = x.clone();
                                g.set(cx.this, v::grav, x)?;
                            }
                        } else {
                            // 357: if (global.hasSpringShoes)
                            g.at("scripts/characterStepEvent", 357);
                            if g.gget(gv::hasSpringShoes)?.is_truthy() {
                                // 357: yAcc *= 1.5;
                                g.at("scripts/characterStepEvent", 357);
                                {
                                    let old = g.get(cx.this, v::yAcc)?;
                                    let x = lenient(Value::mul, old, &r(1.5));
                                    cx.ret = x.clone();
                                    g.set(cx.this, v::yAcc, x)?;
                                }
                            } else {
                                // 360: yAccLimit = 6;
                                g.at("scripts/characterStepEvent", 360);
                                {
                                    let x = r(6.0);
                                    cx.ret = x.clone();
                                    g.set(cx.this, v::yAccLimit, x)?;
                                }
                                // 361: grav = gravNorm;
                                g.at("scripts/characterStepEvent", 361);
                                {
                                    let x = g.get(cx.this, v::gravNorm)?;
                                    cx.ret = x.clone();
                                    g.set(cx.this, v::grav, x)?;
                                }
                            }
                        }
                        // 364: playSound(global.sndJump);
                        g.at("scripts/characterStepEvent", 364);
                        cx.ret = { let a = [g.gget(gv::sndJump)?]; scr::playSound(g, cx, &a)? };
                        // 366: pushTimer = 0;
                        g.at("scripts/characterStepEvent", 366);
                        {
                            let x = r(0.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::pushTimer, x)?;
                        }
                        // 369: state = FALLING;
                        g.at("scripts/characterStepEvent", 369);
                        {
                            let x = g.get(cx.this, v::FALLING)?;
                            cx.ret = x.clone();
                            g.set(cx.this, v::state, x)?;
                        }
                        // 371: jumpButtonReleased = 0;
                        g.at("scripts/characterStepEvent", 371);
                        {
                            let x = r(0.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::jumpButtonReleased, x)?;
                        }
                        // 372: jumpTime = 0;
                        g.at("scripts/characterStepEvent", 372);
                        {
                            let x = r(0.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::jumpTime, x)?;
                        }
                    }
                }
            }
        }
    }
    // 375: if (jumpTime < jumpTimeTotal)
    g.at("scripts/characterStepEvent", 375);
    if g.get(cx.this, v::jumpTime)?.gml_lt(&g.get(cx.this, v::jumpTimeTotal)?)? {
        // 375: jumpTime += 1;
        g.at("scripts/characterStepEvent", 375);
        {
            let old = g.get(cx.this, v::jumpTime)?;
            let x = lenient(Value::add, old, &r(1.0));
            cx.ret = x.clone();
            g.set(cx.this, v::jumpTime, x)?;
        }
    }
    // 377: if (kJump == 0)
    g.at("scripts/characterStepEvent", 377);
    if g.get(cx.this, v::kJump)?.gml_eq(&r(0.0))? {
        // 377: jumpButtonReleased = 1;
        g.at("scripts/characterStepEvent", 377);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::jumpButtonReleased, x)?;
        }
    }
    // 378: if (jumpButtonReleased)
    g.at("scripts/characterStepEvent", 378);
    if g.get(cx.this, v::jumpButtonReleased)?.is_truthy() {
        // 378: jumpTime = jumpTimeTotal;
        g.at("scripts/characterStepEvent", 378);
        {
            let x = g.get(cx.this, v::jumpTimeTotal)?;
            cx.ret = x.clone();
            g.set(cx.this, v::jumpTime, x)?;
        }
    }
    // 380: gravityIntensity = jumpTime / jumpTimeTotal * grav;
    g.at("scripts/characterStepEvent", 380);
    {
        let x = g.get(cx.this, v::jumpTime)?.div(&g.get(cx.this, v::jumpTimeTotal)?)?.mul(&g.get(cx.this, v::grav)?)?;
        cx.ret = x.clone();
        g.set(cx.this, v::gravityIntensity, x)?;
    }
    // 382: if (kUp && platformCharacterIs(ON_GROUND) && !colLadder)
    g.at("scripts/characterStepEvent", 382);
    if and(and(g.get(cx.this, v::kUp)?.is_truthy(), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), g.get(cx.this, v::colLadder)?.not()?.is_truthy()) {
        // 384: looking = UP;
        g.at("scripts/characterStepEvent", 384);
        {
            let x = g.get(cx.this, v::UP)?;
            cx.ret = x.clone();
            g.set(cx.this, v::looking, x)?;
        }
        // 385: if (xVel == 0 && xAcc == 0)
        g.at("scripts/characterStepEvent", 385);
        if and(g.get(cx.this, v::xVel)?.gml_eq(&r(0.0))?, g.get(cx.this, v::xAcc)?.gml_eq(&r(0.0))?) {
            // 385: state = LOOKING_UP;
            g.at("scripts/characterStepEvent", 385);
            {
                let x = g.get(cx.this, v::LOOKING_UP)?;
                cx.ret = x.clone();
                g.set(cx.this, v::state, x)?;
            }
        }
    } else {
        // 387: looking = 0;
        g.at("scripts/characterStepEvent", 387);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::looking, x)?;
        }
    }
    // 389: if (!kUp && state == LOOKING_UP)
    g.at("scripts/characterStepEvent", 389);
    if and(g.get(cx.this, v::kUp)?.not()?.is_truthy(), g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::LOOKING_UP)?)?) {
        // 391: state = STANDING;
        g.at("scripts/characterStepEvent", 391);
        {
            let x = g.get(cx.this, v::STANDING)?;
            cx.ret = x.clone();
            g.set(cx.this, v::state, x)?;
        }
    }
    // 400: if (!colTop)
    g.at("scripts/characterStepEvent", 400);
    if g.get(cx.this, v::colTop)?.not()?.is_truthy() {
        // 402: if (global.hasGloves && yVel > 0)
        g.at("scripts/characterStepEvent", 402);
        if and(g.gget(gv::hasGloves)?.is_truthy(), g.get(cx.this, v::yVel)?.gml_gt(&r(0.0))?) {
            // 404: if (hangCount == 0 && y > 16 && !platformCharacterIs(ON_GROUND) && kRight && colRight && (collision_
            g.at("scripts/characterStepEvent", 404);
            if and(and(and(and(and(g.get(cx.this, v::hangCount)?.gml_eq(&r(0.0))?, g.get(cx.this, v::y)?.gml_gt(&r(16.0))?), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).not()?.is_truthy()), g.get(cx.this, v::kRight)?.is_truthy()), g.get(cx.this, v::colRight)?.is_truthy()), or(({ let a = [g.get(cx.this, v::x)?.add(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(5.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), ({ let a = [g.get(cx.this, v::x)?.add(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(6.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy())) {
                // 407: state = HANGING;
                g.at("scripts/characterStepEvent", 407);
                {
                    let x = g.get(cx.this, v::HANGING)?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::state, x)?;
                }
                // 408: move_snap(1, 8);
                g.at("scripts/characterStepEvent", 408);
                cx.ret = b::move_snap(g, cx, [r(1.0), r(8.0)])?;
                // 409: yVel = 0;
                g.at("scripts/characterStepEvent", 409);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yVel, x)?;
                }
                // 410: yAcc = 0;
                g.at("scripts/characterStepEvent", 410);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yAcc, x)?;
                }
                // 411: grav = 0;
                g.at("scripts/characterStepEvent", 411);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::grav, x)?;
                }
            } else {
                // 413: if (hangCount == 0 && y > 16 && !platformCharacterIs(ON_GROUND) && kLeft && colLeft && (collision_po
                g.at("scripts/characterStepEvent", 413);
                if and(and(and(and(and(g.get(cx.this, v::hangCount)?.gml_eq(&r(0.0))?, g.get(cx.this, v::y)?.gml_gt(&r(16.0))?), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).not()?.is_truthy()), g.get(cx.this, v::kLeft)?.is_truthy()), g.get(cx.this, v::colLeft)?.is_truthy()), or(({ let a = [g.get(cx.this, v::x)?.sub(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(5.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), ({ let a = [g.get(cx.this, v::x)?.sub(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(6.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy())) {
                    // 416: state = HANGING;
                    g.at("scripts/characterStepEvent", 416);
                    {
                        let x = g.get(cx.this, v::HANGING)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::state, x)?;
                    }
                    // 417: move_snap(1, 8);
                    g.at("scripts/characterStepEvent", 417);
                    cx.ret = b::move_snap(g, cx, [r(1.0), r(8.0)])?;
                    // 418: yVel = 0;
                    g.at("scripts/characterStepEvent", 418);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::yVel, x)?;
                    }
                    // 419: yAcc = 0;
                    g.at("scripts/characterStepEvent", 419);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::yAcc, x)?;
                    }
                    // 420: grav = 0;
                    g.at("scripts/characterStepEvent", 420);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::grav, x)?;
                    }
                }
            }
        } else {
            // 423: if (hangCount == 0 && y > 16 && !platformCharacterIs(ON_GROUND) && kRight && colRight && (collision_
            g.at("scripts/characterStepEvent", 423);
            if and(and(and(and(and(g.get(cx.this, v::hangCount)?.gml_eq(&r(0.0))?, g.get(cx.this, v::y)?.gml_gt(&r(16.0))?), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).not()?.is_truthy()), g.get(cx.this, v::kRight)?.is_truthy()), g.get(cx.this, v::colRight)?.is_truthy()), or(({ let a = [g.get(cx.this, v::x)?.add(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(5.0))?, obj::oTree, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), ({ let a = [g.get(cx.this, v::x)?.add(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(6.0))?, obj::oTree, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy())) {
                // 426: state = HANGING;
                g.at("scripts/characterStepEvent", 426);
                {
                    let x = g.get(cx.this, v::HANGING)?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::state, x)?;
                }
                // 427: move_snap(1, 8);
                g.at("scripts/characterStepEvent", 427);
                cx.ret = b::move_snap(g, cx, [r(1.0), r(8.0)])?;
                // 428: yVel = 0;
                g.at("scripts/characterStepEvent", 428);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yVel, x)?;
                }
                // 429: yAcc = 0;
                g.at("scripts/characterStepEvent", 429);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yAcc, x)?;
                }
                // 430: grav = 0;
                g.at("scripts/characterStepEvent", 430);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::grav, x)?;
                }
            } else {
                // 432: if (hangCount == 0 && y > 16 && !platformCharacterIs(ON_GROUND) && kLeft && colLeft && (collision_po
                g.at("scripts/characterStepEvent", 432);
                if and(and(and(and(and(g.get(cx.this, v::hangCount)?.gml_eq(&r(0.0))?, g.get(cx.this, v::y)?.gml_gt(&r(16.0))?), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).not()?.is_truthy()), g.get(cx.this, v::kLeft)?.is_truthy()), g.get(cx.this, v::colLeft)?.is_truthy()), or(({ let a = [g.get(cx.this, v::x)?.sub(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(5.0))?, obj::oTree, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), ({ let a = [g.get(cx.this, v::x)?.sub(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(6.0))?, obj::oTree, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy())) {
                    // 435: state = HANGING;
                    g.at("scripts/characterStepEvent", 435);
                    {
                        let x = g.get(cx.this, v::HANGING)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::state, x)?;
                    }
                    // 436: move_snap(1, 8);
                    g.at("scripts/characterStepEvent", 436);
                    cx.ret = b::move_snap(g, cx, [r(1.0), r(8.0)])?;
                    // 437: yVel = 0;
                    g.at("scripts/characterStepEvent", 437);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::yVel, x)?;
                    }
                    // 438: yAcc = 0;
                    g.at("scripts/characterStepEvent", 438);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::yAcc, x)?;
                    }
                    // 439: grav = 0;
                    g.at("scripts/characterStepEvent", 439);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::grav, x)?;
                    }
                } else {
                    // 441: if (hangCount == 0 && y > 16 && !platformCharacterIs(ON_GROUND) && kRight && colRight && (collision_
                    g.at("scripts/characterStepEvent", 441);
                    if and(and(and(and(and(and(and(g.get(cx.this, v::hangCount)?.gml_eq(&r(0.0))?, g.get(cx.this, v::y)?.gml_gt(&r(16.0))?), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).not()?.is_truthy()), g.get(cx.this, v::kRight)?.is_truthy()), g.get(cx.this, v::colRight)?.is_truthy()), or(({ let a = [g.get(cx.this, v::x)?.add(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(5.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), ({ let a = [g.get(cx.this, v::x)?.add(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(6.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy())), ({ let a = [g.get(cx.this, v::x)?.add(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(9.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(9.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()) {
                        // 445: state = HANGING;
                        g.at("scripts/characterStepEvent", 445);
                        {
                            let x = g.get(cx.this, v::HANGING)?;
                            cx.ret = x.clone();
                            g.set(cx.this, v::state, x)?;
                        }
                        // 446: move_snap(1, 8);
                        g.at("scripts/characterStepEvent", 446);
                        cx.ret = b::move_snap(g, cx, [r(1.0), r(8.0)])?;
                        // 447: yVel = 0;
                        g.at("scripts/characterStepEvent", 447);
                        {
                            let x = r(0.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::yVel, x)?;
                        }
                        // 448: yAcc = 0;
                        g.at("scripts/characterStepEvent", 448);
                        {
                            let x = r(0.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::yAcc, x)?;
                        }
                        // 449: grav = 0;
                        g.at("scripts/characterStepEvent", 449);
                        {
                            let x = r(0.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::grav, x)?;
                        }
                    } else {
                        // 451: if (hangCount == 0 && y > 16 && !platformCharacterIs(ON_GROUND) && kLeft && colLeft && (collision_po
                        g.at("scripts/characterStepEvent", 451);
                        if and(and(and(and(and(and(and(g.get(cx.this, v::hangCount)?.gml_eq(&r(0.0))?, g.get(cx.this, v::y)?.gml_gt(&r(16.0))?), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).not()?.is_truthy()), g.get(cx.this, v::kLeft)?.is_truthy()), g.get(cx.this, v::colLeft)?.is_truthy()), or(({ let a = [g.get(cx.this, v::x)?.sub(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(5.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), ({ let a = [g.get(cx.this, v::x)?.sub(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(6.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy())), ({ let a = [g.get(cx.this, v::x)?.sub(&r(9.0))?, g.get(cx.this, v::y)?.sub(&r(9.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(9.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()) {
                            // 455: state = HANGING;
                            g.at("scripts/characterStepEvent", 455);
                            {
                                let x = g.get(cx.this, v::HANGING)?;
                                cx.ret = x.clone();
                                g.set(cx.this, v::state, x)?;
                            }
                            // 456: move_snap(1, 8);
                            g.at("scripts/characterStepEvent", 456);
                            cx.ret = b::move_snap(g, cx, [r(1.0), r(8.0)])?;
                            // 457: yVel = 0;
                            g.at("scripts/characterStepEvent", 457);
                            {
                                let x = r(0.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::yVel, x)?;
                            }
                            // 458: yAcc = 0;
                            g.at("scripts/characterStepEvent", 458);
                            {
                                let x = r(0.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::yAcc, x)?;
                            }
                            // 459: grav = 0;
                            g.at("scripts/characterStepEvent", 459);
                            {
                                let x = r(0.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::grav, x)?;
                            }
                        }
                    }
                }
            }
        }
        // 462: if (hangCount == 0 && y > 16 && !platformCharacterIs(ON_GROUND) && state == FALLING && (collision_po
        g.at("scripts/characterStepEvent", 462);
        if and(and(and(and(and(and(g.get(cx.this, v::hangCount)?.gml_eq(&r(0.0))?, g.get(cx.this, v::y)?.gml_gt(&r(16.0))?), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).not()?.is_truthy()), g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::FALLING)?)?), or(({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(5.0))?, obj::oArrow, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(6.0))?, obj::oArrow, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy())), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(9.0))?, obj::oArrow, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(9.0))?, obj::oArrow, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()) {
            // 466: obj = instance_nearest(x, y - 5, oArrow);
            g.at("scripts/characterStepEvent", 466);
            {
                let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(5.0))?, obj::oArrow]; b::instance_nearest(g, cx, a)? };
                cx.ret = x.clone();
                g.set(cx.this, v::obj, x)?;
            }
            // 467: if (obj.stuck)
            g.at("scripts/characterStepEvent", 467);
            if g.get_on(cx, &g.get(cx.this, v::obj)?, v::stuck)?.is_truthy() {
                // 469: state = HANGING;
                g.at("scripts/characterStepEvent", 469);
                {
                    let x = g.get(cx.this, v::HANGING)?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::state, x)?;
                }
                // 471: yVel = 0;
                g.at("scripts/characterStepEvent", 471);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yVel, x)?;
                }
                // 472: yAcc = 0;
                g.at("scripts/characterStepEvent", 472);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yAcc, x)?;
                }
                // 473: grav = 0;
                g.at("scripts/characterStepEvent", 473);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::grav, x)?;
                }
            }
        }
    }
    // 492: if (hangCount > 0)
    g.at("scripts/characterStepEvent", 492);
    if g.get(cx.this, v::hangCount)?.gml_gt(&r(0.0))? {
        // 492: hangCount -= 1;
        g.at("scripts/characterStepEvent", 492);
        {
            let old = g.get(cx.this, v::hangCount)?;
            let x = lenient(Value::sub, old, &r(1.0));
            cx.ret = x.clone();
            g.set(cx.this, v::hangCount, x)?;
        }
    }
    // 494: if (state == HANGING)
    g.at("scripts/characterStepEvent", 494);
    if g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::HANGING)?)? {
        // 496: if (instance_exists(oCape))
        g.at("scripts/characterStepEvent", 496);
        if b::instance_exists(g, cx, [obj::oCape])?.is_truthy() {
            // 496: oCape.open = false;
            g.at("scripts/characterStepEvent", 496);
            {
                let t = obj::oCape;
                let x = r(0.0);
                cx.ret = x.clone();
                g.set_on(cx, &t, v::open, x)?;
            }
        }
        // 497: kJumped = false;
        g.at("scripts/characterStepEvent", 497);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::kJumped, x)?;
        }
        // 499: if (kDown && kJumpPressed)
        g.at("scripts/characterStepEvent", 499);
        if and(g.get(cx.this, v::kDown)?.is_truthy(), g.get(cx.this, v::kJumpPressed)?.is_truthy()) {
            // 501: grav = gravNorm;
            g.at("scripts/characterStepEvent", 501);
            {
                let x = g.get(cx.this, v::gravNorm)?;
                cx.ret = x.clone();
                g.set(cx.this, v::grav, x)?;
            }
            // 502: state = FALLING;
            g.at("scripts/characterStepEvent", 502);
            {
                let x = g.get(cx.this, v::FALLING)?;
                cx.ret = x.clone();
                g.set(cx.this, v::state, x)?;
            }
            // 503: yAcc -= grav;
            g.at("scripts/characterStepEvent", 503);
            {
                let old = g.get(cx.this, v::yAcc)?;
                let x = lenient(Value::sub, old, &g.get(cx.this, v::grav)?);
                cx.ret = x.clone();
                g.set(cx.this, v::yAcc, x)?;
            }
            // 504: hangCount = 5;
            g.at("scripts/characterStepEvent", 504);
            {
                let x = r(5.0);
                cx.ret = x.clone();
                g.set(cx.this, v::hangCount, x)?;
            }
            // 505: if (global.hasGloves)
            g.at("scripts/characterStepEvent", 505);
            if g.gget(gv::hasGloves)?.is_truthy() {
                // 505: hangCount = 10;
                g.at("scripts/characterStepEvent", 505);
                {
                    let x = r(10.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::hangCount, x)?;
                }
            }
        } else {
            // 507: if (kJumpPressed)
            g.at("scripts/characterStepEvent", 507);
            if g.get(cx.this, v::kJumpPressed)?.is_truthy() {
                // 509: grav = gravNorm;
                g.at("scripts/characterStepEvent", 509);
                {
                    let x = g.get(cx.this, v::gravNorm)?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::grav, x)?;
                }
                // 510: if (facing == RIGHT && kLeft || (facing == LEFT && kRight))
                g.at("scripts/characterStepEvent", 510);
                if or(and(g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)?, g.get(cx.this, v::kLeft)?.is_truthy()), and(g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)?, g.get(cx.this, v::kRight)?.is_truthy())) {
                    // 512: state = FALLING;
                    g.at("scripts/characterStepEvent", 512);
                    {
                        let x = g.get(cx.this, v::FALLING)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::state, x)?;
                    }
                    // 513: yAcc -= grav;
                    g.at("scripts/characterStepEvent", 513);
                    {
                        let old = g.get(cx.this, v::yAcc)?;
                        let x = lenient(Value::sub, old, &g.get(cx.this, v::grav)?);
                        cx.ret = x.clone();
                        g.set(cx.this, v::yAcc, x)?;
                    }
                } else {
                    // 517: state = JUMPING;
                    g.at("scripts/characterStepEvent", 517);
                    {
                        let x = g.get(cx.this, v::JUMPING)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::state, x)?;
                    }
                    // 518: yAcc += initialJumpAcc * 2;
                    g.at("scripts/characterStepEvent", 518);
                    {
                        let old = g.get(cx.this, v::yAcc)?;
                        let x = lenient(Value::add, old, &g.get(cx.this, v::initialJumpAcc)?.mul(&r(2.0))?);
                        cx.ret = x.clone();
                        g.set(cx.this, v::yAcc, x)?;
                    }
                    // 519: if (facing == RIGHT)
                    g.at("scripts/characterStepEvent", 519);
                    if g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)? {
                        // 519: x -= 2;
                        g.at("scripts/characterStepEvent", 519);
                        {
                            let old = g.get(cx.this, v::x)?;
                            let x = old.sub(&r(2.0))?;
                            cx.ret = x.clone();
                            g.set(cx.this, v::x, x)?;
                        }
                    } else {
                        // 520: x += 2;
                        g.at("scripts/characterStepEvent", 520);
                        {
                            let old = g.get(cx.this, v::x)?;
                            let x = old.add(&r(2.0))?;
                            cx.ret = x.clone();
                            g.set(cx.this, v::x, x)?;
                        }
                    }
                }
                // 522: hangCount = hangCountMax;
                g.at("scripts/characterStepEvent", 522);
                {
                    let x = g.get(cx.this, v::hangCountMax)?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::hangCount, x)?;
                }
            }
        }
        // 525: if (facing == LEFT && !isCollisionLeft(2) || (facing == RIGHT && !isCollisionRight(2)))
        g.at("scripts/characterStepEvent", 525);
        if or(and(g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)?, scr::isCollisionLeft(g, cx, &[r(2.0)])?.not()?.is_truthy()), and(g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)?, scr::isCollisionRight(g, cx, &[r(2.0)])?.not()?.is_truthy())) {
            // 528: grav = gravNorm;
            g.at("scripts/characterStepEvent", 528);
            {
                let x = g.get(cx.this, v::gravNorm)?;
                cx.ret = x.clone();
                g.set(cx.this, v::grav, x)?;
            }
            // 529: state = FALLING;
            g.at("scripts/characterStepEvent", 529);
            {
                let x = g.get(cx.this, v::FALLING)?;
                cx.ret = x.clone();
                g.set(cx.this, v::state, x)?;
            }
            // 530: yAcc -= grav;
            g.at("scripts/characterStepEvent", 530);
            {
                let old = g.get(cx.this, v::yAcc)?;
                let x = lenient(Value::sub, old, &g.get(cx.this, v::grav)?);
                cx.ret = x.clone();
                g.set(cx.this, v::yAcc, x)?;
            }
            // 531: hangCount = 4;
            g.at("scripts/characterStepEvent", 531);
            {
                let x = r(4.0);
                cx.ret = x.clone();
                g.set(cx.this, v::hangCount, x)?;
            }
        }
    } else {
        // 536: grav = gravNorm;
        g.at("scripts/characterStepEvent", 536);
        {
            let x = g.get(cx.this, v::gravNorm)?;
            cx.ret = x.clone();
            g.set(cx.this, v::grav, x)?;
        }
    }
    // 540: if (kDown && platformCharacterIs(ON_GROUND) && !whipping)
    g.at("scripts/characterStepEvent", 540);
    if and(and(g.get(cx.this, v::kDown)?.is_truthy(), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), g.get(cx.this, v::whipping)?.not()?.is_truthy()) {
        // 542: if (colBot)
        g.at("scripts/characterStepEvent", 542);
        if g.get(cx.this, v::colBot)?.is_truthy() {
            // 544: state = DUCKING;
            g.at("scripts/characterStepEvent", 544);
            {
                let x = g.get(cx.this, v::DUCKING)?;
                cx.ret = x.clone();
                g.set(cx.this, v::state, x)?;
            }
        } else {
            // 546: if (colPlatBot)
            g.at("scripts/characterStepEvent", 546);
            if g.get(cx.this, v::colPlatBot)?.is_truthy() {
                // 549: fallTimer = 0;
                g.at("scripts/characterStepEvent", 549);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::fallTimer, x)?;
                }
                // 550: if (!colBot)
                g.at("scripts/characterStepEvent", 550);
                if g.get(cx.this, v::colBot)?.not()?.is_truthy() {
                    // 552: ladder = 0;
                    g.at("scripts/characterStepEvent", 552);
                    {
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::ladder, x)?;
                    }
                    // 553: ladder = instance_place(x, y + 16, oLadder);
                    g.at("scripts/characterStepEvent", 553);
                    {
                        let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(16.0))?, obj::oLadder]; b::instance_place(g, cx, a)? };
                        cx.ret = x.clone();
                        g.set(cx.this, v::ladder, x)?;
                    }
                    // 554: if (instance_exists(ladder))
                    g.at("scripts/characterStepEvent", 554);
                    if ({ let a = [g.get(cx.this, v::ladder)?]; b::instance_exists(g, cx, a)? }).is_truthy() {
                        // 556: if (abs(x - (ladder.x + 8)) < 4)
                        g.at("scripts/characterStepEvent", 556);
                        if ({ let a = [g.get(cx.this, v::x)?.sub(&g.get_on(cx, &g.get(cx.this, v::ladder)?, v::x)?.add(&r(8.0))?)?]; b::abs(g, cx, a)? }).gml_lt(&r(4.0))? {
                            // 558: x = ladder.x + 8;
                            g.at("scripts/characterStepEvent", 558);
                            {
                                let x = g.get_on(cx, &g.get(cx.this, v::ladder)?, v::x)?.add(&r(8.0))?;
                                cx.ret = x.clone();
                                g.set(cx.this, v::x, x)?;
                            }
                            // 560: xVel = 0;
                            g.at("scripts/characterStepEvent", 560);
                            {
                                let x = r(0.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::xVel, x)?;
                            }
                            // 561: yVel = 0;
                            g.at("scripts/characterStepEvent", 561);
                            {
                                let x = r(0.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::yVel, x)?;
                            }
                            // 562: xAcc = 0;
                            g.at("scripts/characterStepEvent", 562);
                            {
                                let x = r(0.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::xAcc, x)?;
                            }
                            // 563: yAcc = 0;
                            g.at("scripts/characterStepEvent", 563);
                            {
                                let x = r(0.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::yAcc, x)?;
                            }
                            // 564: state = CLIMBING;
                            g.at("scripts/characterStepEvent", 564);
                            {
                                let x = g.get(cx.this, v::CLIMBING)?;
                                cx.ret = x.clone();
                                g.set(cx.this, v::state, x)?;
                            }
                        }
                    } else {
                        // 569: y += 1;
                        g.at("scripts/characterStepEvent", 569);
                        {
                            let old = g.get(cx.this, v::y)?;
                            let x = old.add(&r(1.0))?;
                            cx.ret = x.clone();
                            g.set(cx.this, v::y, x)?;
                        }
                        // 570: state = FALLING;
                        g.at("scripts/characterStepEvent", 570);
                        {
                            let x = g.get(cx.this, v::FALLING)?;
                            cx.ret = x.clone();
                            g.set(cx.this, v::state, x)?;
                        }
                        // 571: yAcc += grav;
                        g.at("scripts/characterStepEvent", 571);
                        {
                            let old = g.get(cx.this, v::yAcc)?;
                            let x = lenient(Value::add, old, &g.get(cx.this, v::grav)?);
                            cx.ret = x.clone();
                            g.set(cx.this, v::yAcc, x)?;
                        }
                    }
                } else {
                    // 577: state = RUNNING;
                    g.at("scripts/characterStepEvent", 577);
                    {
                        let x = g.get(cx.this, v::RUNNING)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::state, x)?;
                    }
                }
            }
        }
    }
    // 581: if (!kDown && state == DUCKING)
    g.at("scripts/characterStepEvent", 581);
    if and(g.get(cx.this, v::kDown)?.not()?.is_truthy(), g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::DUCKING)?)?) {
        // 583: state = STANDING;
        g.at("scripts/characterStepEvent", 583);
        {
            let x = g.get(cx.this, v::STANDING)?;
            cx.ret = x.clone();
            g.set(cx.this, v::state, x)?;
        }
        // 584: xVel = 0;
        g.at("scripts/characterStepEvent", 584);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::xVel, x)?;
        }
        // 585: xAcc = 0;
        g.at("scripts/characterStepEvent", 585);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::xAcc, x)?;
        }
    }
    // 587: if (xVel == 0 && xAcc == 0 && state == RUNNING)
    g.at("scripts/characterStepEvent", 587);
    if and(and(g.get(cx.this, v::xVel)?.gml_eq(&r(0.0))?, g.get(cx.this, v::xAcc)?.gml_eq(&r(0.0))?), g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::RUNNING)?)?) {
        // 589: state = STANDING;
        g.at("scripts/characterStepEvent", 589);
        {
            let x = g.get(cx.this, v::STANDING)?;
            cx.ret = x.clone();
            g.set(cx.this, v::state, x)?;
        }
    }
    // 591: if (xAcc != 0 && state == STANDING)
    g.at("scripts/characterStepEvent", 591);
    if and(g.get(cx.this, v::xAcc)?.gml_ne(&r(0.0))?, g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::STANDING)?)?) {
        // 593: state = RUNNING;
        g.at("scripts/characterStepEvent", 593);
        {
            let x = g.get(cx.this, v::RUNNING)?;
            cx.ret = x.clone();
            g.set(cx.this, v::state, x)?;
        }
    }
    // 595: if (yVel < 0 && platformCharacterIs(IN_AIR) && state != HANGING)
    g.at("scripts/characterStepEvent", 595);
    if and(and(g.get(cx.this, v::yVel)?.gml_lt(&r(0.0))?, ({ let a = [g.get(cx.this, v::IN_AIR)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::HANGING)?)?) {
        // 597: state = JUMPING;
        g.at("scripts/characterStepEvent", 597);
        {
            let x = g.get(cx.this, v::JUMPING)?;
            cx.ret = x.clone();
            g.set(cx.this, v::state, x)?;
        }
    }
    // 599: if (yVel > 0 && platformCharacterIs(IN_AIR) && state != HANGING)
    g.at("scripts/characterStepEvent", 599);
    if and(and(g.get(cx.this, v::yVel)?.gml_gt(&r(0.0))?, ({ let a = [g.get(cx.this, v::IN_AIR)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::HANGING)?)?) {
        // 601: state = FALLING;
        g.at("scripts/characterStepEvent", 601);
        {
            let x = g.get(cx.this, v::FALLING)?;
            cx.ret = x.clone();
            g.set(cx.this, v::state, x)?;
        }
        // 602: setCollisionBounds(-5, -6, 5, 8);
        g.at("scripts/characterStepEvent", 602);
        cx.ret = scr::setCollisionBounds(g, cx, &[r(-5.0), r(-6.0), r(5.0), r(8.0)])?;
    } else {
        // 604: setCollisionBounds(-5, -8, 5, 8);
        g.at("scripts/characterStepEvent", 604);
        cx.ret = scr::setCollisionBounds(g, cx, &[r(-5.0), r(-8.0), r(5.0), r(8.0)])?;
    }
    // 607: colPointLadder = collision_point(x, y, oLadder, 0, 0) || collision_point(x, y, oLadderTop, 0, 0);
    g.at("scripts/characterStepEvent", 607);
    {
        let x = b2v(or(({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLadder, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy(), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLadderTop, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()));
        cx.ret = x.clone();
        g.set(cx.this, v::colPointLadder, x)?;
    }
    // 609: if (kUp && platformCharacterIs(IN_AIR) && collision_point(x, y - 8, oLadder, 0, 0) && ladderTimer ==
    g.at("scripts/characterStepEvent", 609);
    if or(or(and(and(and(g.get(cx.this, v::kUp)?.is_truthy(), ({ let a = [g.get(cx.this, v::IN_AIR)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(8.0))?, obj::oLadder, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()), g.get(cx.this, v::ladderTimer)?.gml_eq(&r(0.0))?), and(and(g.get(cx.this, v::kUp)?.is_truthy(), g.get(cx.this, v::colPointLadder)?.is_truthy()), g.get(cx.this, v::ladderTimer)?.gml_eq(&r(0.0))?)), and(and(and(and(and(g.get(cx.this, v::kDown)?.is_truthy(), g.get(cx.this, v::colPointLadder)?.is_truthy()), g.get(cx.this, v::ladderTimer)?.gml_eq(&r(0.0))?), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(9.0))?, obj::oLadderTop, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()), g.get(cx.this, v::xVel)?.gml_eq(&r(0.0))?)) {
        // 613: ladder = 0;
        g.at("scripts/characterStepEvent", 613);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::ladder, x)?;
        }
        // 614: ladder = instance_place(x, y - 8, oLadder);
        g.at("scripts/characterStepEvent", 614);
        {
            let x = { let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(8.0))?, obj::oLadder]; b::instance_place(g, cx, a)? };
            cx.ret = x.clone();
            g.set(cx.this, v::ladder, x)?;
        }
        // 615: if (instance_exists(ladder))
        g.at("scripts/characterStepEvent", 615);
        if ({ let a = [g.get(cx.this, v::ladder)?]; b::instance_exists(g, cx, a)? }).is_truthy() {
            // 617: if (abs(x - (ladder.x + 8)) < 4)
            g.at("scripts/characterStepEvent", 617);
            if ({ let a = [g.get(cx.this, v::x)?.sub(&g.get_on(cx, &g.get(cx.this, v::ladder)?, v::x)?.add(&r(8.0))?)?]; b::abs(g, cx, a)? }).gml_lt(&r(4.0))? {
                // 619: x = ladder.x + 8;
                g.at("scripts/characterStepEvent", 619);
                {
                    let x = g.get_on(cx, &g.get(cx.this, v::ladder)?, v::x)?.add(&r(8.0))?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::x, x)?;
                }
                // 620: if (!collision_point(x, y, oLadder, 0, 0) && !collision_point(x, y, oLadderTop, 0, 0))
                g.at("scripts/characterStepEvent", 620);
                if and(({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLadder, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy(), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oLadderTop, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()) {
                    // 623: y = ladder.y + 14;
                    g.at("scripts/characterStepEvent", 623);
                    {
                        let x = g.get_on(cx, &g.get(cx.this, v::ladder)?, v::y)?.add(&r(14.0))?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::y, x)?;
                    }
                }
                // 626: xVel = 0;
                g.at("scripts/characterStepEvent", 626);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::xVel, x)?;
                }
                // 627: yVel = 0;
                g.at("scripts/characterStepEvent", 627);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yVel, x)?;
                }
                // 628: xAcc = 0;
                g.at("scripts/characterStepEvent", 628);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::xAcc, x)?;
                }
                // 629: yAcc = 0;
                g.at("scripts/characterStepEvent", 629);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yAcc, x)?;
                }
                // 630: state = CLIMBING;
                g.at("scripts/characterStepEvent", 630);
                {
                    let x = g.get(cx.this, v::CLIMBING)?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::state, x)?;
                }
            }
        }
    }
    // 678: if (state == CLIMBING)
    g.at("scripts/characterStepEvent", 678);
    if g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::CLIMBING)?)? {
        // 680: xFric = frictionClimbingX;
        g.at("scripts/characterStepEvent", 680);
        {
            let x = g.get(cx.this, v::frictionClimbingX)?;
            cx.ret = x.clone();
            g.set(cx.this, v::xFric, x)?;
        }
        // 681: yFric = frictionClimbingY;
        g.at("scripts/characterStepEvent", 681);
        {
            let x = g.get(cx.this, v::frictionClimbingY)?;
            cx.ret = x.clone();
            g.set(cx.this, v::yFric, x)?;
        }
    } else {
        // 685: if (runKey && platformCharacterIs(ON_GROUND) && runHeld >= 10)
        g.at("scripts/characterStepEvent", 685);
        if and(and(g.get(cx.this, v::runKey)?.is_truthy(), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), g.get(cx.this, v::runHeld)?.gml_ge(&r(10.0))?) {
            // 687: if (kLeft)
            g.at("scripts/characterStepEvent", 687);
            if g.get(cx.this, v::kLeft)?.is_truthy() {
                // 689: xVel -= 0.1;
                g.at("scripts/characterStepEvent", 689);
                {
                    let old = g.get(cx.this, v::xVel)?;
                    let x = lenient(Value::sub, old, &r(0.1));
                    cx.ret = x.clone();
                    g.set(cx.this, v::xVel, x)?;
                }
                // 690: xVelLimit = 6;
                g.at("scripts/characterStepEvent", 690);
                {
                    let x = r(6.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::xVelLimit, x)?;
                }
                // 691: xFric = frictionRunningFastX;
                g.at("scripts/characterStepEvent", 691);
                {
                    let x = g.get(cx.this, v::frictionRunningFastX)?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::xFric, x)?;
                }
            } else {
                // 693: if (kRight)
                g.at("scripts/characterStepEvent", 693);
                if g.get(cx.this, v::kRight)?.is_truthy() {
                    // 695: xVel += 0.1;
                    g.at("scripts/characterStepEvent", 695);
                    {
                        let old = g.get(cx.this, v::xVel)?;
                        let x = lenient(Value::add, old, &r(0.1));
                        cx.ret = x.clone();
                        g.set(cx.this, v::xVel, x)?;
                    }
                    // 696: xVelLimit = 6;
                    g.at("scripts/characterStepEvent", 696);
                    {
                        let x = r(6.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::xVelLimit, x)?;
                    }
                    // 697: xFric = frictionRunningFastX;
                    g.at("scripts/characterStepEvent", 697);
                    {
                        let x = g.get(cx.this, v::frictionRunningFastX)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::xFric, x)?;
                    }
                }
            }
        } else {
            // 700: if (state == DUCKING)
            g.at("scripts/characterStepEvent", 700);
            if g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::DUCKING)?)? {
                // 702: if (xVel < 2 && xVel > -2)
                g.at("scripts/characterStepEvent", 702);
                if and(g.get(cx.this, v::xVel)?.gml_lt(&r(2.0))?, g.get(cx.this, v::xVel)?.gml_gt(&r(-2.0))?) {
                    // 704: xFric = 0.2;
                    g.at("scripts/characterStepEvent", 704);
                    {
                        let x = r(0.2);
                        cx.ret = x.clone();
                        g.set(cx.this, v::xFric, x)?;
                    }
                    // 705: xVelLimit = 3;
                    g.at("scripts/characterStepEvent", 705);
                    {
                        let x = r(3.0);
                        cx.ret = x.clone();
                        g.set(cx.this, v::xVelLimit, x)?;
                    }
                    // 706: image_speed = 0.8;
                    g.at("scripts/characterStepEvent", 706);
                    {
                        let x = r(0.8);
                        cx.ret = x.clone();
                        g.set(cx.this, v::image_speed, x)?;
                    }
                } else {
                    // 708: if (kLeft && global.downToRun)
                    g.at("scripts/characterStepEvent", 708);
                    if and(g.get(cx.this, v::kLeft)?.is_truthy(), g.gget(gv::downToRun)?.is_truthy()) {
                        // 710: xVel -= 0.1;
                        g.at("scripts/characterStepEvent", 710);
                        {
                            let old = g.get(cx.this, v::xVel)?;
                            let x = lenient(Value::sub, old, &r(0.1));
                            cx.ret = x.clone();
                            g.set(cx.this, v::xVel, x)?;
                        }
                        // 711: xVelLimit = 6;
                        g.at("scripts/characterStepEvent", 711);
                        {
                            let x = r(6.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::xVelLimit, x)?;
                        }
                        // 712: xFric = frictionRunningFastX;
                        g.at("scripts/characterStepEvent", 712);
                        {
                            let x = g.get(cx.this, v::frictionRunningFastX)?;
                            cx.ret = x.clone();
                            g.set(cx.this, v::xFric, x)?;
                        }
                    } else {
                        // 714: if (kRight && global.downToRun)
                        g.at("scripts/characterStepEvent", 714);
                        if and(g.get(cx.this, v::kRight)?.is_truthy(), g.gget(gv::downToRun)?.is_truthy()) {
                            // 716: xVel += 0.1;
                            g.at("scripts/characterStepEvent", 716);
                            {
                                let old = g.get(cx.this, v::xVel)?;
                                let x = lenient(Value::add, old, &r(0.1));
                                cx.ret = x.clone();
                                g.set(cx.this, v::xVel, x)?;
                            }
                            // 717: xVelLimit = 6;
                            g.at("scripts/characterStepEvent", 717);
                            {
                                let x = r(6.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::xVelLimit, x)?;
                            }
                            // 718: xFric = frictionRunningFastX;
                            g.at("scripts/characterStepEvent", 718);
                            {
                                let x = g.get(cx.this, v::frictionRunningFastX)?;
                                cx.ret = x.clone();
                                g.set(cx.this, v::xFric, x)?;
                            }
                        } else {
                            // 722: xVel *= 0.8;
                            g.at("scripts/characterStepEvent", 722);
                            {
                                let old = g.get(cx.this, v::xVel)?;
                                let x = lenient(Value::mul, old, &r(0.8));
                                cx.ret = x.clone();
                                g.set(cx.this, v::xVel, x)?;
                            }
                            // 723: if (xVel < 0.5)
                            g.at("scripts/characterStepEvent", 723);
                            if g.get(cx.this, v::xVel)?.gml_lt(&r(0.5))? {
                                // 723: xVel = 0;
                                g.at("scripts/characterStepEvent", 723);
                                {
                                    let x = r(0.0);
                                    cx.ret = x.clone();
                                    g.set(cx.this, v::xVel, x)?;
                                }
                            }
                            // 724: xFric = 0.2;
                            g.at("scripts/characterStepEvent", 724);
                            {
                                let x = r(0.2);
                                cx.ret = x.clone();
                                g.set(cx.this, v::xFric, x)?;
                            }
                            // 725: xVelLimit = 3;
                            g.at("scripts/characterStepEvent", 725);
                            {
                                let x = r(3.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::xVelLimit, x)?;
                            }
                            // 726: image_speed = 0.8;
                            g.at("scripts/characterStepEvent", 726);
                            {
                                let x = r(0.8);
                                cx.ret = x.clone();
                                g.set(cx.this, v::image_speed, x)?;
                            }
                        }
                    }
                }
            } else {
                // 732: if (platformCharacterIs(IN_AIR))
                g.at("scripts/characterStepEvent", 732);
                if ({ let a = [g.get(cx.this, v::IN_AIR)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy() {
                    // 734: if (dead || stunned)
                    g.at("scripts/characterStepEvent", 734);
                    if or(g.get(cx.this, v::dead)?.is_truthy(), g.get(cx.this, v::stunned)?.is_truthy()) {
                        // 734: xFric = 1;
                        g.at("scripts/characterStepEvent", 734);
                        {
                            let x = r(1.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::xFric, x)?;
                        }
                    } else {
                        // 735: xFric = 0.8;
                        g.at("scripts/characterStepEvent", 735);
                        {
                            let x = r(0.8);
                            cx.ret = x.clone();
                            g.set(cx.this, v::xFric, x)?;
                        }
                    }
                } else {
                    // 739: xFric = frictionRunningX;
                    g.at("scripts/characterStepEvent", 739);
                    {
                        let x = g.get(cx.this, v::frictionRunningX)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::xFric, x)?;
                    }
                }
            }
        }
        // 744: if (collision_point(x, y, oWeb, 0, 0))
        g.at("scripts/characterStepEvent", 744);
        if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oWeb, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
            // 746: xFric = 0.2;
            g.at("scripts/characterStepEvent", 746);
            {
                let x = r(0.2);
                cx.ret = x.clone();
                g.set(cx.this, v::xFric, x)?;
            }
            // 747: yFric = 0.2;
            g.at("scripts/characterStepEvent", 747);
            {
                let x = r(0.2);
                cx.ret = x.clone();
                g.set(cx.this, v::yFric, x)?;
            }
            // 748: fallTimer = 0;
            g.at("scripts/characterStepEvent", 748);
            {
                let x = r(0.0);
                cx.ret = x.clone();
                g.set(cx.this, v::fallTimer, x)?;
            }
        } else {
            // 750: if (collision_point(x, y, oWater, -1, -1))
            g.at("scripts/characterStepEvent", 750);
            if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?, obj::oWater, r(-1.0), r(-1.0)]; b::collision_point(g, cx, a)? }).is_truthy() {
                // 752: if (instance_exists(oCape))
                g.at("scripts/characterStepEvent", 752);
                if b::instance_exists(g, cx, [obj::oCape])?.is_truthy() {
                    // 752: oCape.open = false;
                    g.at("scripts/characterStepEvent", 752);
                    {
                        let t = obj::oCape;
                        let x = r(0.0);
                        cx.ret = x.clone();
                        g.set_on(cx, &t, v::open, x)?;
                    }
                }
                // 754: if (state == FALLING && yVel > 0)
                g.at("scripts/characterStepEvent", 754);
                if and(g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::FALLING)?)?, g.get(cx.this, v::yVel)?.gml_gt(&r(0.0))?) {
                    // 756: yFric = 0.5;
                    g.at("scripts/characterStepEvent", 756);
                    {
                        let x = r(0.5);
                        cx.ret = x.clone();
                        g.set(cx.this, v::yFric, x)?;
                    }
                } else {
                    // 758: if (!collision_point(x, y - 9, oWater, -1, -1))
                    g.at("scripts/characterStepEvent", 758);
                    if ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.sub(&r(9.0))?, obj::oWater, r(-1.0), r(-1.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy() {
                        // 760: yFric = 1;
                        g.at("scripts/characterStepEvent", 760);
                        {
                            let x = r(1.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::yFric, x)?;
                        }
                    } else {
                        // 764: yFric = 0.9;
                        g.at("scripts/characterStepEvent", 764);
                        {
                            let x = r(0.9);
                            cx.ret = x.clone();
                            g.set(cx.this, v::yFric, x)?;
                        }
                    }
                }
            } else {
                // 769: swimming = false;
                g.at("scripts/characterStepEvent", 769);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::swimming, x)?;
                }
                // 770: yFric = 1;
                g.at("scripts/characterStepEvent", 770);
                {
                    let x = r(1.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yFric, x)?;
                }
            }
        }
    }
    // 774: if (colIceBot && state != DUCKING && !global.hasSpikeShoes)
    g.at("scripts/characterStepEvent", 774);
    if and(and(g.get(cx.this, v::colIceBot)?.is_truthy(), g.get(cx.this, v::state)?.gml_ne(&g.get(cx.this, v::DUCKING)?)?), g.gget(gv::hasSpikeShoes)?.not()?.is_truthy()) {
        // 776: xFric = 0.98;
        g.at("scripts/characterStepEvent", 776);
        {
            let x = r(0.98);
            cx.ret = x.clone();
            g.set(cx.this, v::xFric, x)?;
        }
        // 777: yFric = 1;
        g.at("scripts/characterStepEvent", 777);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::yFric, x)?;
        }
    }
    // 782: if (platformCharacterIs(ON_GROUND))
    g.at("scripts/characterStepEvent", 782);
    if ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy() {
        // 784: if (state == RUNNING && kLeft && colLeft)
        g.at("scripts/characterStepEvent", 784);
        if and(and(g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::RUNNING)?)?, g.get(cx.this, v::kLeft)?.is_truthy()), g.get(cx.this, v::colLeft)?.is_truthy()) {
            // 786: pushTimer += 1;
            g.at("scripts/characterStepEvent", 786);
            {
                let old = g.get(cx.this, v::pushTimer)?;
                let x = lenient(Value::add, old, &r(1.0));
                cx.ret = x.clone();
                g.set(cx.this, v::pushTimer, x)?;
            }
        } else {
            // 788: if (state == RUNNING && kRight && colRight)
            g.at("scripts/characterStepEvent", 788);
            if and(and(g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::RUNNING)?)?, g.get(cx.this, v::kRight)?.is_truthy()), g.get(cx.this, v::colRight)?.is_truthy()) {
                // 790: pushTimer += 1;
                g.at("scripts/characterStepEvent", 790);
                {
                    let old = g.get(cx.this, v::pushTimer)?;
                    let x = lenient(Value::add, old, &r(1.0));
                    cx.ret = x.clone();
                    g.set(cx.this, v::pushTimer, x)?;
                }
            } else {
                // 794: pushTimer = 0;
                g.at("scripts/characterStepEvent", 794);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::pushTimer, x)?;
                }
            }
        }
        // 797: if (platformCharacterIs(ON_GROUND) && !kJump && !kDown && !runKey)
        g.at("scripts/characterStepEvent", 797);
        if and(and(and(({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy(), g.get(cx.this, v::kJump)?.not()?.is_truthy()), g.get(cx.this, v::kDown)?.not()?.is_truthy()), g.get(cx.this, v::runKey)?.not()?.is_truthy()) {
            // 799: xVelLimit = 3;
            g.at("scripts/characterStepEvent", 799);
            {
                let x = r(3.0);
                cx.ret = x.clone();
                g.set(cx.this, v::xVelLimit, x)?;
            }
        }
        // 804: if (state == DUCKING && abs(xVel) < 3 && facing == LEFT && collision_point(x, y + 9, oSolid, 0, 0) &
        g.at("scripts/characterStepEvent", 804);
        if and(and(and(and(and(g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::DUCKING)?)?, ({ let a = [g.get(cx.this, v::xVel)?]; b::abs(g, cx, a)? }).gml_lt(&r(3.0))?), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::LEFT)?)?), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(9.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()), ({ let a = [g.get(cx.this, v::x)?.sub(&r(1.0))?, g.get(cx.this, v::y)?.add(&r(9.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()), g.get(cx.this, v::kLeft)?.is_truthy()) {
            // 807: state = DUCKTOHANG;
            g.at("scripts/characterStepEvent", 807);
            {
                let x = g.get(cx.this, v::DUCKTOHANG)?;
                cx.ret = x.clone();
                g.set(cx.this, v::state, x)?;
            }
            // 809: if (holdItem)
            g.at("scripts/characterStepEvent", 809);
            if g.get(cx.this, v::holdItem)?.is_truthy() {
                // 811: holdItem.held = false;
                g.at("scripts/characterStepEvent", 811);
                {
                    let t = g.get(cx.this, v::holdItem)?;
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set_on(cx, &t, v::held, x)?;
                }
                // 812: if (holdItem.type == "Gold Idol")
                g.at("scripts/characterStepEvent", 812);
                if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Gold Idol"))? {
                    // 812: holdItem.y -= 8;
                    g.at("scripts/characterStepEvent", 812);
                    {
                        let t = g.get(cx.this, v::holdItem)?;
                        let old = { let t2 = g.get(cx.this, v::holdItem)?; g.get_on(cx, &t2, v::y)? };
                        let x = old.sub(&r(8.0))?;
                        cx.ret = x.clone();
                        g.set_on(cx, &t, v::y, x)?;
                    }
                }
                // 813: scrDropItem(-1, -4);
                g.at("scripts/characterStepEvent", 813);
                cx.ret = scr::scrDropItem(g, cx, &[r(-1.0), r(-4.0)])?;
            }
            // 816: with (oMonkey)
            g.at("scripts/characterStepEvent", 816);
            {
                let t = obj::oMonkey;
                let mut it1 = g.with_iter(cx, &t)?;
                let outer3 = (cx.this, cx.other);
                'with1: while let Some(h2) = it1.next(g) {
                    (cx.this, cx.other) = (h2, outer3.0);
                    // 819: if (status == 7)
                    g.at("scripts/characterStepEvent", 819);
                    if g.get(cx.this, v::status)?.gml_eq(&r(7.0))? {
                        // 821: xVel = -1;
                        g.at("scripts/characterStepEvent", 821);
                        {
                            let x = r(-1.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::xVel, x)?;
                        }
                        // 822: yVel = -4;
                        g.at("scripts/characterStepEvent", 822);
                        {
                            let x = r(-4.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::yVel, x)?;
                        }
                        // 823: status = 1;
                        g.at("scripts/characterStepEvent", 823);
                        {
                            let x = r(1.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::status, x)?;
                        }
                        // 824: vineCounter = 20;
                        g.at("scripts/characterStepEvent", 824);
                        {
                            let x = r(20.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::vineCounter, x)?;
                        }
                        // 825: grabCounter = 60;
                        g.at("scripts/characterStepEvent", 825);
                        {
                            let x = r(60.0);
                            cx.ret = x.clone();
                            g.set(cx.this, v::grabCounter, x)?;
                        }
                    }
                }
                (cx.this, cx.other) = outer3;
            }
        } else {
            // 829: if (state == DUCKING && abs(xVel) < 3 && facing == RIGHT && collision_point(x, y + 9, oSolid, 0, 0) 
            g.at("scripts/characterStepEvent", 829);
            if and(and(and(and(and(g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::DUCKING)?)?, ({ let a = [g.get(cx.this, v::xVel)?]; b::abs(g, cx, a)? }).gml_lt(&r(3.0))?), g.get(cx.this, v::facing)?.gml_eq(&g.get(cx.this, v::RIGHT)?)?), ({ let a = [g.get(cx.this, v::x)?, g.get(cx.this, v::y)?.add(&r(9.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).is_truthy()), ({ let a = [g.get(cx.this, v::x)?.add(&r(1.0))?, g.get(cx.this, v::y)?.add(&r(9.0))?, obj::oSolid, r(0.0), r(0.0)]; b::collision_point(g, cx, a)? }).not()?.is_truthy()), g.get(cx.this, v::kRight)?.is_truthy()) {
                // 832: state = DUCKTOHANG;
                g.at("scripts/characterStepEvent", 832);
                {
                    let x = g.get(cx.this, v::DUCKTOHANG)?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::state, x)?;
                }
                // 834: if (holdItem)
                g.at("scripts/characterStepEvent", 834);
                if g.get(cx.this, v::holdItem)?.is_truthy() {
                    // 837: if (holdItem.type == "Gold Idol")
                    g.at("scripts/characterStepEvent", 837);
                    if g.get_on(cx, &g.get(cx.this, v::holdItem)?, v::r#type)?.gml_eq(&s!("Gold Idol"))? {
                        // 837: holdItem.y -= 8;
                        g.at("scripts/characterStepEvent", 837);
                        {
                            let t = g.get(cx.this, v::holdItem)?;
                            let old = { let t2 = g.get(cx.this, v::holdItem)?; g.get_on(cx, &t2, v::y)? };
                            let x = old.sub(&r(8.0))?;
                            cx.ret = x.clone();
                            g.set_on(cx, &t, v::y, x)?;
                        }
                    }
                    // 838: scrDropItem(1, -4);
                    g.at("scripts/characterStepEvent", 838);
                    cx.ret = scr::scrDropItem(g, cx, &[r(1.0), r(-4.0)])?;
                }
                // 841: with (oMonkey)
                g.at("scripts/characterStepEvent", 841);
                {
                    let t = obj::oMonkey;
                    let mut it4 = g.with_iter(cx, &t)?;
                    let outer6 = (cx.this, cx.other);
                    'with2: while let Some(h5) = it4.next(g) {
                        (cx.this, cx.other) = (h5, outer6.0);
                        // 844: if (status == 7)
                        g.at("scripts/characterStepEvent", 844);
                        if g.get(cx.this, v::status)?.gml_eq(&r(7.0))? {
                            // 846: xVel = 1;
                            g.at("scripts/characterStepEvent", 846);
                            {
                                let x = r(1.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::xVel, x)?;
                            }
                            // 847: yVel = -4;
                            g.at("scripts/characterStepEvent", 847);
                            {
                                let x = r(-4.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::yVel, x)?;
                            }
                            // 848: status = 1;
                            g.at("scripts/characterStepEvent", 848);
                            {
                                let x = r(1.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::status, x)?;
                            }
                            // 849: vineCounter = 20;
                            g.at("scripts/characterStepEvent", 849);
                            {
                                let x = r(20.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::vineCounter, x)?;
                            }
                            // 850: grabCounter = 60;
                            g.at("scripts/characterStepEvent", 850);
                            {
                                let x = r(60.0);
                                cx.ret = x.clone();
                                g.set(cx.this, v::grabCounter, x)?;
                            }
                        }
                    }
                    (cx.this, cx.other) = outer6;
                }
            }
        }
    }
    // 856: if (state == DUCKTOHANG)
    g.at("scripts/characterStepEvent", 856);
    if g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::DUCKTOHANG)?)? {
        // 858: x = xPrev;
        g.at("scripts/characterStepEvent", 858);
        {
            let x = g.get(cx.this, v::xPrev)?;
            cx.ret = x.clone();
            g.set(cx.this, v::x, x)?;
        }
        // 859: y = yPrev;
        g.at("scripts/characterStepEvent", 859);
        {
            let x = g.get(cx.this, v::yPrev)?;
            cx.ret = x.clone();
            g.set(cx.this, v::y, x)?;
        }
        // 860: xVel = 0;
        g.at("scripts/characterStepEvent", 860);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::xVel, x)?;
        }
        // 861: yVel = 0;
        g.at("scripts/characterStepEvent", 861);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::yVel, x)?;
        }
        // 862: xAcc = 0;
        g.at("scripts/characterStepEvent", 862);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::xAcc, x)?;
        }
        // 863: yAcc = 0;
        g.at("scripts/characterStepEvent", 863);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::yAcc, x)?;
        }
        // 864: grav = 0;
        g.at("scripts/characterStepEvent", 864);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::grav, x)?;
        }
    }
    // 868: if (instance_exists(oParachute))
    g.at("scripts/characterStepEvent", 868);
    if b::instance_exists(g, cx, [obj::oParachute])?.is_truthy() {
        // 870: yFric = 0.5;
        g.at("scripts/characterStepEvent", 870);
        {
            let x = r(0.5);
            cx.ret = x.clone();
            g.set(cx.this, v::yFric, x)?;
        }
    }
    // 872: if (instance_exists(oCape))
    g.at("scripts/characterStepEvent", 872);
    if b::instance_exists(g, cx, [obj::oCape])?.is_truthy() {
        // 874: if (oCape.open)
        g.at("scripts/characterStepEvent", 874);
        if g.get_on(cx, &obj::oCape, v::open)?.is_truthy() {
            // 874: yFric = 0.5;
            g.at("scripts/characterStepEvent", 874);
            {
                let x = r(0.5);
                cx.ret = x.clone();
                g.set(cx.this, v::yFric, x)?;
            }
        }
    }
    // 877: if (pushTimer > 100)
    g.at("scripts/characterStepEvent", 877);
    if g.get(cx.this, v::pushTimer)?.gml_gt(&r(100.0))? {
        // 877: pushTimer = 100;
        g.at("scripts/characterStepEvent", 877);
        {
            let x = r(100.0);
            cx.ret = x.clone();
            g.set(cx.this, v::pushTimer, x)?;
        }
    }
    // 880: if (xAcc > xAccLimit)
    g.at("scripts/characterStepEvent", 880);
    if g.get(cx.this, v::xAcc)?.gml_gt(&g.get(cx.this, v::xAccLimit)?)? {
        // 880: xAcc = xAccLimit;
        g.at("scripts/characterStepEvent", 880);
        {
            let x = g.get(cx.this, v::xAccLimit)?;
            cx.ret = x.clone();
            g.set(cx.this, v::xAcc, x)?;
        }
    } else {
        // 881: if (xAcc < -xAccLimit)
        g.at("scripts/characterStepEvent", 881);
        if g.get(cx.this, v::xAcc)?.gml_lt(&g.get(cx.this, v::xAccLimit)?.neg()?)? {
            // 881: xAcc = -xAccLimit;
            g.at("scripts/characterStepEvent", 881);
            {
                let x = g.get(cx.this, v::xAccLimit)?.neg()?;
                cx.ret = x.clone();
                g.set(cx.this, v::xAcc, x)?;
            }
        }
    }
    // 882: if (yAcc > yAccLimit)
    g.at("scripts/characterStepEvent", 882);
    if g.get(cx.this, v::yAcc)?.gml_gt(&g.get(cx.this, v::yAccLimit)?)? {
        // 882: yAcc = yAccLimit;
        g.at("scripts/characterStepEvent", 882);
        {
            let x = g.get(cx.this, v::yAccLimit)?;
            cx.ret = x.clone();
            g.set(cx.this, v::yAcc, x)?;
        }
    } else {
        // 883: if (yAcc < -yAccLimit)
        g.at("scripts/characterStepEvent", 883);
        if g.get(cx.this, v::yAcc)?.gml_lt(&g.get(cx.this, v::yAccLimit)?.neg()?)? {
            // 883: yAcc = -yAccLimit;
            g.at("scripts/characterStepEvent", 883);
            {
                let x = g.get(cx.this, v::yAccLimit)?.neg()?;
                cx.ret = x.clone();
                g.set(cx.this, v::yAcc, x)?;
            }
        }
    }
    // 886: xVel += xAcc;
    g.at("scripts/characterStepEvent", 886);
    {
        let old = g.get(cx.this, v::xVel)?;
        let x = lenient(Value::add, old, &g.get(cx.this, v::xAcc)?);
        cx.ret = x.clone();
        g.set(cx.this, v::xVel, x)?;
    }
    // 887: if (dead || stunned)
    g.at("scripts/characterStepEvent", 887);
    if or(g.get(cx.this, v::dead)?.is_truthy(), g.get(cx.this, v::stunned)?.is_truthy()) {
        // 887: yVel += 0.6;
        g.at("scripts/characterStepEvent", 887);
        {
            let old = g.get(cx.this, v::yVel)?;
            let x = lenient(Value::add, old, &r(0.6));
            cx.ret = x.clone();
            g.set(cx.this, v::yVel, x)?;
        }
    } else {
        // 888: yVel += yAcc;
        g.at("scripts/characterStepEvent", 888);
        {
            let old = g.get(cx.this, v::yVel)?;
            let x = lenient(Value::add, old, &g.get(cx.this, v::yAcc)?);
            cx.ret = x.clone();
            g.set(cx.this, v::yVel, x)?;
        }
    }
    // 891: xAcc = 0;
    g.at("scripts/characterStepEvent", 891);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::xAcc, x)?;
    }
    // 892: yAcc = 0;
    g.at("scripts/characterStepEvent", 892);
    {
        let x = r(0.0);
        cx.ret = x.clone();
        g.set(cx.this, v::yAcc, x)?;
    }
    // 895: xVel *= xFric;
    g.at("scripts/characterStepEvent", 895);
    {
        let old = g.get(cx.this, v::xVel)?;
        let x = lenient(Value::mul, old, &g.get(cx.this, v::xFric)?);
        cx.ret = x.clone();
        g.set(cx.this, v::xVel, x)?;
    }
    // 896: yVel *= yFric;
    g.at("scripts/characterStepEvent", 896);
    {
        let old = g.get(cx.this, v::yVel)?;
        let x = lenient(Value::mul, old, &g.get(cx.this, v::yFric)?);
        cx.ret = x.clone();
        g.set(cx.this, v::yVel, x)?;
    }
    // 899: if (instance_exists(oBall))
    g.at("scripts/characterStepEvent", 899);
    if b::instance_exists(g, cx, [obj::oBall])?.is_truthy() {
        // 901: if (distance_to_object(oBall) >= 24)
        g.at("scripts/characterStepEvent", 901);
        if b::distance_to_object(g, cx, [obj::oBall])?.gml_ge(&r(24.0))? {
            // 903: if (xVel > 0 && oBall.x < x && abs(oBall.x - x) > 24)
            g.at("scripts/characterStepEvent", 903);
            if and(and(g.get(cx.this, v::xVel)?.gml_gt(&r(0.0))?, g.get_on(cx, &obj::oBall, v::x)?.gml_lt(&g.get(cx.this, v::x)?)?), ({ let a = [g.get_on(cx, &obj::oBall, v::x)?.sub(&g.get(cx.this, v::x)?)?]; b::abs(g, cx, a)? }).gml_gt(&r(24.0))?) {
                // 903: xVel = 0;
                g.at("scripts/characterStepEvent", 903);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::xVel, x)?;
                }
            }
            // 904: if (xVel < 0 && oBall.x > x && abs(oBall.x - x) > 24)
            g.at("scripts/characterStepEvent", 904);
            if and(and(g.get(cx.this, v::xVel)?.gml_lt(&r(0.0))?, g.get_on(cx, &obj::oBall, v::x)?.gml_gt(&g.get(cx.this, v::x)?)?), ({ let a = [g.get_on(cx, &obj::oBall, v::x)?.sub(&g.get(cx.this, v::x)?)?]; b::abs(g, cx, a)? }).gml_gt(&r(24.0))?) {
                // 904: xVel = 0;
                g.at("scripts/characterStepEvent", 904);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::xVel, x)?;
                }
            }
            // 905: if (yVel > 0 && oBall.y < y && abs(oBall.y - y) > 24)
            g.at("scripts/characterStepEvent", 905);
            if and(and(g.get(cx.this, v::yVel)?.gml_gt(&r(0.0))?, g.get_on(cx, &obj::oBall, v::y)?.gml_lt(&g.get(cx.this, v::y)?)?), ({ let a = [g.get_on(cx, &obj::oBall, v::y)?.sub(&g.get(cx.this, v::y)?)?]; b::abs(g, cx, a)? }).gml_gt(&r(24.0))?) {
                // 907: if (abs(oBall.x - x) < 1)
                g.at("scripts/characterStepEvent", 907);
                if ({ let a = [g.get_on(cx, &obj::oBall, v::x)?.sub(&g.get(cx.this, v::x)?)?]; b::abs(g, cx, a)? }).gml_lt(&r(1.0))? {
                    // 909: x = oBall.x;
                    g.at("scripts/characterStepEvent", 909);
                    {
                        let x = g.get_on(cx, &obj::oBall, v::x)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::x, x)?;
                    }
                } else {
                    // 911: if (oBall.x < x && !kRight)
                    g.at("scripts/characterStepEvent", 911);
                    if and(g.get_on(cx, &obj::oBall, v::x)?.gml_lt(&g.get(cx.this, v::x)?)?, g.get(cx.this, v::kRight)?.not()?.is_truthy()) {
                        // 913: if (xVel > 0)
                        g.at("scripts/characterStepEvent", 913);
                        if g.get(cx.this, v::xVel)?.gml_gt(&r(0.0))? {
                            // 913: xVel *= -0.25;
                            g.at("scripts/characterStepEvent", 913);
                            {
                                let old = g.get(cx.this, v::xVel)?;
                                let x = lenient(Value::mul, old, &r(-0.25));
                                cx.ret = x.clone();
                                g.set(cx.this, v::xVel, x)?;
                            }
                        } else {
                            // 914: if (xVel == 0)
                            g.at("scripts/characterStepEvent", 914);
                            if g.get(cx.this, v::xVel)?.gml_eq(&r(0.0))? {
                                // 914: xVel -= 1;
                                g.at("scripts/characterStepEvent", 914);
                                {
                                    let old = g.get(cx.this, v::xVel)?;
                                    let x = lenient(Value::sub, old, &r(1.0));
                                    cx.ret = x.clone();
                                    g.set(cx.this, v::xVel, x)?;
                                }
                            }
                        }
                    } else {
                        // 916: if (oBall.x > x && !kLeft)
                        g.at("scripts/characterStepEvent", 916);
                        if and(g.get_on(cx, &obj::oBall, v::x)?.gml_gt(&g.get(cx.this, v::x)?)?, g.get(cx.this, v::kLeft)?.not()?.is_truthy()) {
                            // 918: if (xVel < 0)
                            g.at("scripts/characterStepEvent", 918);
                            if g.get(cx.this, v::xVel)?.gml_lt(&r(0.0))? {
                                // 918: xVel *= -0.25;
                                g.at("scripts/characterStepEvent", 918);
                                {
                                    let old = g.get(cx.this, v::xVel)?;
                                    let x = lenient(Value::mul, old, &r(-0.25));
                                    cx.ret = x.clone();
                                    g.set(cx.this, v::xVel, x)?;
                                }
                            } else {
                                // 919: if (xVel == 0)
                                g.at("scripts/characterStepEvent", 919);
                                if g.get(cx.this, v::xVel)?.gml_eq(&r(0.0))? {
                                    // 919: xVel += 1;
                                    g.at("scripts/characterStepEvent", 919);
                                    {
                                        let old = g.get(cx.this, v::xVel)?;
                                        let x = lenient(Value::add, old, &r(1.0));
                                        cx.ret = x.clone();
                                        g.set(cx.this, v::xVel, x)?;
                                    }
                                }
                            }
                        }
                    }
                }
                // 921: yVel = 0;
                g.at("scripts/characterStepEvent", 921);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yVel, x)?;
                }
                // 922: fallTimer = 0;
                g.at("scripts/characterStepEvent", 922);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::fallTimer, x)?;
                }
            }
            // 924: if (yVel < 0 && oBall.y > y && abs(oBall.y - y) > 24)
            g.at("scripts/characterStepEvent", 924);
            if and(and(g.get(cx.this, v::yVel)?.gml_lt(&r(0.0))?, g.get_on(cx, &obj::oBall, v::y)?.gml_gt(&g.get(cx.this, v::y)?)?), ({ let a = [g.get_on(cx, &obj::oBall, v::y)?.sub(&g.get(cx.this, v::y)?)?]; b::abs(g, cx, a)? }).gml_gt(&r(24.0))?) {
                // 924: yVel = 0;
                g.at("scripts/characterStepEvent", 924);
                {
                    let x = r(0.0);
                    cx.ret = x.clone();
                    g.set(cx.this, v::yVel, x)?;
                }
            }
        }
    }
    // 929: if (!dead && !stunned)
    g.at("scripts/characterStepEvent", 929);
    if and(g.get(cx.this, v::dead)?.not()?.is_truthy(), g.get(cx.this, v::stunned)?.not()?.is_truthy()) {
        // 931: if (xVel > xVelLimit)
        g.at("scripts/characterStepEvent", 931);
        if g.get(cx.this, v::xVel)?.gml_gt(&g.get(cx.this, v::xVelLimit)?)? {
            // 931: xVel = xVelLimit;
            g.at("scripts/characterStepEvent", 931);
            {
                let x = g.get(cx.this, v::xVelLimit)?;
                cx.ret = x.clone();
                g.set(cx.this, v::xVel, x)?;
            }
        } else {
            // 932: if (xVel < -xVelLimit)
            g.at("scripts/characterStepEvent", 932);
            if g.get(cx.this, v::xVel)?.gml_lt(&g.get(cx.this, v::xVelLimit)?.neg()?)? {
                // 932: xVel = -xVelLimit;
                g.at("scripts/characterStepEvent", 932);
                {
                    let x = g.get(cx.this, v::xVelLimit)?.neg()?;
                    cx.ret = x.clone();
                    g.set(cx.this, v::xVel, x)?;
                }
            }
        }
    }
    // 934: if (yVel > yVelLimit)
    g.at("scripts/characterStepEvent", 934);
    if g.get(cx.this, v::yVel)?.gml_gt(&g.get(cx.this, v::yVelLimit)?)? {
        // 934: yVel = yVelLimit;
        g.at("scripts/characterStepEvent", 934);
        {
            let x = g.get(cx.this, v::yVelLimit)?;
            cx.ret = x.clone();
            g.set(cx.this, v::yVel, x)?;
        }
    } else {
        // 935: if (yVel < -yVelLimit)
        g.at("scripts/characterStepEvent", 935);
        if g.get(cx.this, v::yVel)?.gml_lt(&g.get(cx.this, v::yVelLimit)?.neg()?)? {
            // 935: yVel = -yVelLimit;
            g.at("scripts/characterStepEvent", 935);
            {
                let x = g.get(cx.this, v::yVelLimit)?.neg()?;
                cx.ret = x.clone();
                g.set(cx.this, v::yVel, x)?;
            }
        }
    }
    // 938: if (approximatelyZero(xVel))
    g.at("scripts/characterStepEvent", 938);
    if ({ let a = [g.get(cx.this, v::xVel)?]; scr::approximatelyZero(g, cx, &a)? }).is_truthy() {
        // 938: xVel = 0;
        g.at("scripts/characterStepEvent", 938);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::xVel, x)?;
        }
    }
    // 939: if (approximatelyZero(yVel))
    g.at("scripts/characterStepEvent", 939);
    if ({ let a = [g.get(cx.this, v::yVel)?]; scr::approximatelyZero(g, cx, &a)? }).is_truthy() {
        // 939: yVel = 0;
        g.at("scripts/characterStepEvent", 939);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::yVel, x)?;
        }
    }
    // 940: if (approximatelyZero(xAcc))
    g.at("scripts/characterStepEvent", 940);
    if ({ let a = [g.get(cx.this, v::xAcc)?]; scr::approximatelyZero(g, cx, &a)? }).is_truthy() {
        // 940: xAcc = 0;
        g.at("scripts/characterStepEvent", 940);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::xAcc, x)?;
        }
    }
    // 941: if (approximatelyZero(yAcc))
    g.at("scripts/characterStepEvent", 941);
    if ({ let a = [g.get(cx.this, v::yAcc)?]; scr::approximatelyZero(g, cx, &a)? }).is_truthy() {
        // 941: yAcc = 0;
        g.at("scripts/characterStepEvent", 941);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::yAcc, x)?;
        }
    }
    // 946: if (maxSlope > 0 && platformCharacterIs(ON_GROUND) && xVel != 0)
    g.at("scripts/characterStepEvent", 946);
    if and(and(g.get(cx.this, v::maxSlope)?.gml_gt(&r(0.0))?, ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()), g.get(cx.this, v::xVel)?.gml_ne(&r(0.0))?) {
        // 948: slopeYPrev = y;
        g.at("scripts/characterStepEvent", 948);
        {
            let x = g.get(cx.this, v::y)?;
            cx.ret = x.clone();
            g.set(cx.this, v::slopeYPrev, x)?;
        }
        // 949: for (...; y >= slopeYPrev - maxSlope; ...)
        g.at("scripts/characterStepEvent", 949);
        // 949: y = y;
        g.at("scripts/characterStepEvent", 949);
        {
            let x = g.get(cx.this, v::y)?;
            cx.ret = x.clone();
            g.set(cx.this, v::y, x)?;
        }
        'for3: loop {
            if !(g.get(cx.this, v::y)?.gml_ge(&g.get(cx.this, v::slopeYPrev)?.sub(&g.get(cx.this, v::maxSlope)?)?)?) { break 'for3; }
            'body4: {
                // 950: if (colTop)
                g.at("scripts/characterStepEvent", 950);
                if g.get(cx.this, v::colTop)?.is_truthy() {
                    // 951: break;
                    g.at("scripts/characterStepEvent", 951);
                    break 'for3;
                }
            }
            // 949: y -= 1;
            g.at("scripts/characterStepEvent", 949);
            {
                let old = g.get(cx.this, v::y)?;
                let x = old.sub(&r(1.0))?;
                cx.ret = x.clone();
                g.set(cx.this, v::y, x)?;
            }
        }
        // 952: slopeChangeInY = slopeYPrev - y;
        g.at("scripts/characterStepEvent", 952);
        {
            let x = g.get(cx.this, v::slopeYPrev)?.sub(&g.get(cx.this, v::y)?)?;
            cx.ret = x.clone();
            g.set(cx.this, v::slopeChangeInY, x)?;
        }
    } else {
        // 955: slopeChangeInY = 0;
        g.at("scripts/characterStepEvent", 955);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::slopeChangeInY, x)?;
        }
    }
    // 959: if (maxSlope * abs(xVel) > 0 && platformCharacterIs(ON_GROUND))
    g.at("scripts/characterStepEvent", 959);
    if and(g.get(cx.this, v::maxSlope)?.mul(&{ let a = [g.get(cx.this, v::xVel)?]; b::abs(g, cx, a)? })?.gml_gt(&r(0.0))?, ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()) {
        // 962: xPrev = x;
        g.at("scripts/characterStepEvent", 962);
        {
            let x = g.get(cx.this, v::x)?;
            cx.ret = x.clone();
            g.set(cx.this, v::xPrev, x)?;
        }
        // 963: yPrev = slopeYPrev;
        g.at("scripts/characterStepEvent", 963);
        {
            let x = g.get(cx.this, v::slopeYPrev)?;
            cx.ret = x.clone();
            g.set(cx.this, v::yPrev, x)?;
        }
        // 964: yPrevHigh = y;
        g.at("scripts/characterStepEvent", 964);
        {
            let x = g.get(cx.this, v::y)?;
            cx.ret = x.clone();
            g.set(cx.this, v::yPrevHigh, x)?;
        }
        // 965: moveTo(xVel, yVel + slopeChangeInY);
        g.at("scripts/characterStepEvent", 965);
        cx.ret = { let a = [g.get(cx.this, v::xVel)?, g.get(cx.this, v::yVel)?.add(&g.get(cx.this, v::slopeChangeInY)?)?]; scr::moveTo(g, cx, &a)? };
        // 966: dist = point_distance(xPrev, yPrev, x, y);
        g.at("scripts/characterStepEvent", 966);
        {
            let x = { let a = [g.get(cx.this, v::xPrev)?, g.get(cx.this, v::yPrev)?, g.get(cx.this, v::x)?, g.get(cx.this, v::y)?]; b::point_distance(g, cx, a)? };
            cx.ret = x.clone();
            g.set(cx.this, v::dist, x)?;
        }
        // 968: if (dist > abs(xVelInteger))
        g.at("scripts/characterStepEvent", 968);
        if g.get(cx.this, v::dist)?.gml_gt(&{ let a = [g.get(cx.this, v::xVelInteger)?]; b::abs(g, cx, a)? })? {
            // 971: excess = dist - abs(xVelInteger);
            g.at("scripts/characterStepEvent", 971);
            {
                let x = g.get(cx.this, v::dist)?.sub(&{ let a = [g.get(cx.this, v::xVelInteger)?]; b::abs(g, cx, a)? })?;
                cx.ret = x.clone();
                g.set(cx.this, v::excess, x)?;
            }
            // 972: if (xVelInteger < 0)
            g.at("scripts/characterStepEvent", 972);
            if g.get(cx.this, v::xVelInteger)?.gml_lt(&r(0.0))? {
                // 973: excess *= -1;
                g.at("scripts/characterStepEvent", 973);
                {
                    let old = g.get(cx.this, v::excess)?;
                    let x = lenient(Value::mul, old, &r(-1.0));
                    cx.ret = x.clone();
                    g.set(cx.this, v::excess, x)?;
                }
            }
            // 975: x = xPrev;
            g.at("scripts/characterStepEvent", 975);
            {
                let x = g.get(cx.this, v::xPrev)?;
                cx.ret = x.clone();
                g.set(cx.this, v::x, x)?;
            }
            // 976: y = yPrevHigh;
            g.at("scripts/characterStepEvent", 976);
            {
                let x = g.get(cx.this, v::yPrevHigh)?;
                cx.ret = x.clone();
                g.set(cx.this, v::y, x)?;
            }
            // 979: ratio = abs(xVelInteger) / dist * 0.9;
            g.at("scripts/characterStepEvent", 979);
            {
                let x = ({ let a = [g.get(cx.this, v::xVelInteger)?]; b::abs(g, cx, a)? }).div(&g.get(cx.this, v::dist)?)?.mul(&r(0.9))?;
                cx.ret = x.clone();
                g.set(cx.this, v::ratio, x)?;
            }
            // 980: moveTo(round(xVelInteger * ratio), round(yVelInteger * ratio + slopeChangeInY));
            g.at("scripts/characterStepEvent", 980);
            cx.ret = { let a = [{ let a = [g.get(cx.this, v::xVelInteger)?.mul(&g.get(cx.this, v::ratio)?)?]; b::round(g, cx, a)? }, { let a = [g.get(cx.this, v::yVelInteger)?.mul(&g.get(cx.this, v::ratio)?)?.add(&g.get(cx.this, v::slopeChangeInY)?)?]; b::round(g, cx, a)? }]; scr::moveTo(g, cx, &a)? };
        }
    } else {
        // 986: moveTo(xVel, yVel);
        g.at("scripts/characterStepEvent", 986);
        cx.ret = { let a = [g.get(cx.this, v::xVel)?, g.get(cx.this, v::yVel)?]; scr::moveTo(g, cx, &a)? };
    }
    // 990: if (!colBot && maxDownSlope > 0 && xVelInteger != 0 && platformCharacterIs(ON_GROUND))
    g.at("scripts/characterStepEvent", 990);
    if and(and(and(g.get(cx.this, v::colBot)?.not()?.is_truthy(), g.get(cx.this, v::maxDownSlope)?.gml_gt(&r(0.0))?), g.get(cx.this, v::xVelInteger)?.gml_ne(&r(0.0))?), ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy()) {
        // 993: upYPrev = y;
        g.at("scripts/characterStepEvent", 993);
        {
            let x = g.get(cx.this, v::y)?;
            cx.ret = x.clone();
            g.set(cx.this, v::upYPrev, x)?;
        }
        // 994: for (...; y <= upYPrev + maxDownSlope; ...)
        g.at("scripts/characterStepEvent", 994);
        // 994: y = y;
        g.at("scripts/characterStepEvent", 994);
        {
            let x = g.get(cx.this, v::y)?;
            cx.ret = x.clone();
            g.set(cx.this, v::y, x)?;
        }
        'for5: loop {
            if !(g.get(cx.this, v::y)?.gml_le(&g.get(cx.this, v::upYPrev)?.add(&g.get(cx.this, v::maxDownSlope)?)?)?) { break 'for5; }
            'body6: {
                // 995: if (colBot)
                g.at("scripts/characterStepEvent", 995);
                if g.get(cx.this, v::colBot)?.is_truthy() {
                    // 997: upYPrev = y;
                    g.at("scripts/characterStepEvent", 997);
                    {
                        let x = g.get(cx.this, v::y)?;
                        cx.ret = x.clone();
                        g.set(cx.this, v::upYPrev, x)?;
                    }
                    // 998: break;
                    g.at("scripts/characterStepEvent", 998);
                    break 'for5;
                }
            }
            // 994: y += 1;
            g.at("scripts/characterStepEvent", 994);
            {
                let old = g.get(cx.this, v::y)?;
                let x = old.add(&r(1.0))?;
                cx.ret = x.clone();
                g.set(cx.this, v::y, x)?;
            }
        }
        // 1000: y = upYPrev;
        g.at("scripts/characterStepEvent", 1000);
        {
            let x = g.get(cx.this, v::upYPrev)?;
            cx.ret = x.clone();
            g.set(cx.this, v::y, x)?;
        }
    }
    // 1004: characterSprite();
    g.at("scripts/characterStepEvent", 1004);
    cx.ret = scr::characterSprite(g, cx, &[])?;
    // 1007: statePrevPrev = statePrev;
    g.at("scripts/characterStepEvent", 1007);
    {
        let x = g.get(cx.this, v::statePrev)?;
        cx.ret = x.clone();
        g.set(cx.this, v::statePrevPrev, x)?;
    }
    // 1008: statePrev = state;
    g.at("scripts/characterStepEvent", 1008);
    {
        let x = g.get(cx.this, v::state)?;
        cx.ret = x.clone();
        g.set(cx.this, v::statePrev, x)?;
    }
    // 1011: if (state == RUNNING || state == DUCKING || state == LOOKING_UP)
    g.at("scripts/characterStepEvent", 1011);
    if or(or(g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::RUNNING)?)?, g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::DUCKING)?)?), g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::LOOKING_UP)?)?) {
        // 1013: if (state == RUNNING || state == LOOKING_UP)
        g.at("scripts/characterStepEvent", 1013);
        if or(g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::RUNNING)?)?, g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::LOOKING_UP)?)?) {
            // 1013: image_speed = abs(xVel) * runAnimSpeed + 0.1;
            g.at("scripts/characterStepEvent", 1013);
            {
                let x = ({ let a = [g.get(cx.this, v::xVel)?]; b::abs(g, cx, a)? }).mul(&g.get(cx.this, v::runAnimSpeed)?)?.add(&r(0.1))?;
                cx.ret = x.clone();
                g.set(cx.this, v::image_speed, x)?;
            }
        }
    }
    // 1016: if (state == CLIMBING)
    g.at("scripts/characterStepEvent", 1016);
    if g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::CLIMBING)?)? {
        // 1016: image_speed = sqrt(sqr(abs(xVel)) + sqr(abs(yVel))) * climbAnimSpeed;
        g.at("scripts/characterStepEvent", 1016);
        {
            let x = ({ let a = [({ let a = [{ let a = [g.get(cx.this, v::xVel)?]; b::abs(g, cx, a)? }]; b::sqr(g, cx, a)? }).add(&{ let a = [{ let a = [g.get(cx.this, v::yVel)?]; b::abs(g, cx, a)? }]; b::sqr(g, cx, a)? })?]; b::sqrt(g, cx, a)? }).mul(&g.get(cx.this, v::climbAnimSpeed)?)?;
            cx.ret = x.clone();
            g.set(cx.this, v::image_speed, x)?;
        }
    }
    // 1017: if (xVel >= 4 || xVel <= -4)
    g.at("scripts/characterStepEvent", 1017);
    if or(g.get(cx.this, v::xVel)?.gml_ge(&r(4.0))?, g.get(cx.this, v::xVel)?.gml_le(&r(-4.0))?) {
        // 1019: image_speed = 1;
        g.at("scripts/characterStepEvent", 1019);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::image_speed, x)?;
        }
        // 1020: if (platformCharacterIs(ON_GROUND))
        g.at("scripts/characterStepEvent", 1020);
        if ({ let a = [g.get(cx.this, v::ON_GROUND)?]; scr::platformCharacterIs(g, cx, &a)? }).is_truthy() {
            // 1020: setCollisionBounds(-8, -8, 8, 8);
            g.at("scripts/characterStepEvent", 1020);
            cx.ret = scr::setCollisionBounds(g, cx, &[r(-8.0), r(-8.0), r(8.0), r(8.0)])?;
        } else {
            // 1021: setCollisionBounds(-5, -8, 5, 8);
            g.at("scripts/characterStepEvent", 1021);
            cx.ret = scr::setCollisionBounds(g, cx, &[r(-5.0), r(-8.0), r(5.0), r(8.0)])?;
        }
    } else {
        // 1023: setCollisionBounds(-5, -8, 5, 8);
        g.at("scripts/characterStepEvent", 1023);
        cx.ret = scr::setCollisionBounds(g, cx, &[r(-5.0), r(-8.0), r(5.0), r(8.0)])?;
    }
    // 1024: if (whipping)
    g.at("scripts/characterStepEvent", 1024);
    if g.get(cx.this, v::whipping)?.is_truthy() {
        // 1024: image_speed = 1;
        g.at("scripts/characterStepEvent", 1024);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::image_speed, x)?;
        }
    }
    // 1025: if (state == DUCKTOHANG)
    g.at("scripts/characterStepEvent", 1025);
    if g.get(cx.this, v::state)?.gml_eq(&g.get(cx.this, v::DUCKTOHANG)?)? {
        // 1027: image_index = 0;
        g.at("scripts/characterStepEvent", 1027);
        {
            let x = r(0.0);
            cx.ret = x.clone();
            g.set(cx.this, v::image_index, x)?;
        }
        // 1028: image_speed = 0.8;
        g.at("scripts/characterStepEvent", 1028);
        {
            let x = r(0.8);
            cx.ret = x.clone();
            g.set(cx.this, v::image_speed, x)?;
        }
    }
    // 1031: if (image_speed > 1)
    g.at("scripts/characterStepEvent", 1031);
    if g.get(cx.this, v::image_speed)?.gml_gt(&r(1.0))? {
        // 1031: image_speed = 1;
        g.at("scripts/characterStepEvent", 1031);
        {
            let x = r(1.0);
            cx.ret = x.clone();
            g.set(cx.this, v::image_speed, x)?;
        }
    }
    Ok(cx.ret.clone())
}
