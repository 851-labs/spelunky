// Hand-port prototype of oArrowTrapRight (all events), for docs/design/translation.md. Illustrative only.
// The trap itself never fires from its step event (the firing code is commented out in the GML); alarm 1
// creates an invisible oArrowTrapTest whose *collision events* set the trap's alarm 0.

pub fn create(w: &mut World, me: Inst) {
    solid::create(w, me); // action_inherited: oSolid -> oDrawnSprite
    let t = w.trap_mut(me);
    t.kind = "Arrow Trap";
    t.facing = 1;
    t.fired = 0;
    t.invincible = false;
    t.x_act = 0.0;
    // GM8 alarm: decremented in the alarm phase of the *next* frame, reaching 0 there, so alarm 1 runs on
    // the first step after creation (runner rule: OpenGMK game/events.rs run_alarms).
    w.set_alarm(me, 1, 1);
}

pub fn alarm_0(w: &mut World, me: Inst) {
    let (x, y) = w.pos(me);
    let arrow = w.instance_create(x + 16.0, y + 4.0, Obj::oArrow);
    // FIDELITY: `arrow` is an *instance variable* of the trap in GML (no `var`), so it persists and is
    // visible to other code as `trap.arrow`. A Rust local loses that; harmless only if nothing reads it.
    w.trap_mut(me).arrow = arrow;
    w.item_mut(arrow).x_vel = 5.0;
}

pub fn alarm_1(w: &mut World, me: Inst) {
    if is_room(w, "rLevelEditor") {
        return;
    }
    let (x, y) = w.pos(me);
    let mut x_act = x + 16.0;
    let mut n = 100;
    // FIDELITY: `not collision_point(...) && n > 0`: both operands are evaluated each iteration in GM8.
    // Equivalent here (no side effects), but only after checking.
    while w.collision_point(x_act, y + 8.0, Obj::oSolid, false, false).is_none() && n > 0 {
        if x_act - x > 96.0 {
            break;
        }
        x_act += 1.0;
        n -= 1;
    }
    x_act -= x + 8.0;
    if x_act < 32.0 {
        x_act = 32.0;
    }
    let test = w.instance_create(x + 16.0, y, Obj::oArrowTrapTest);
    // FIDELITY: image_xscale changes the instance's bounding box, which the test object's collision
    // events depend on; `ceil` must be GM8's.
    w.inst_mut(test).image_xscale = ((x_act - 16.0) / 16.0).ceil();
    let id = w.inst(me).id;
    w.arrow_test_mut(test).trap_id = id;
    // FIDELITY: xAct and n are instance variables in GML too (written into the trap), dropped here.
}

pub fn destroy(w: &mut World, me: Inst) {
    if w.trap(me).clean_death || w.globals.clean_solids {
        return;
    }
    let (x, y) = w.pos(me);
    // FIDELITY: the four rand() calls per instance must run in GML's left-to-right order (x terms, then
    // y terms) to keep the RNG stream; rewriting as `let dx = rand(0,8) - rand(0,8)` is fine, reordering
    // the x and y lines is not.
    for (obj, spr) in [(Obj::oRubble, Spr::sRubbleTan), (Obj::oRubbleSmall, Spr::sRubbleTanSmall),
                       (Obj::oRubbleSmall, Spr::sRubbleTanSmall)] {
        let rx = x + 8.0 + rand(w, me, 0, 8) as f64 - rand(w, me, 0, 8) as f64;
        let ry = y + 8.0 + rand(w, me, 0, 8) as f64 - rand(w, me, 0, 8) as f64;
        let rubble = w.instance_create(rx, ry, obj);
        w.set_sprite(rubble, spr);
    }
    if w.trap(me).fired == 0 {
        w.instance_create(x + 8.0, y + 8.0, Obj::oArrow);
    }
}

pub fn step(w: &mut World, me: Inst) {
    // The GML body is `firing = false; if (fired == 0) { /* commented out */ }`.
    w.trap_mut(me).firing = false;
}
