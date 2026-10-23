/// GML: `oScreen/create#0`.
pub fn create_a0(g: &mut World, cx: &mut Ctx) -> R<()> {
    // 1: scrReadStats();
    g.at("oScreen/create#0", 1);
    scr::scrReadStats(g, cx, &[])?;
    // 2: scrInit();
    g.at("oScreen/create#0", 2);
    scr::scrInit(g, cx, &[])?;
    Ok(())
}

/// GML: `oScreen/create#5`.
pub fn create_a5(g: &mut World, cx: &mut Ctx) -> R<()> {
    let mut l_h = Local::default();
    let mut l_w = Local::default();
    // 1: py = 0;
    g.at("oScreen/create#5", 1);
    g.set(cx.this, v::py, r(0.0))?;
    // 4: screen_x = 0;
    g.at("oScreen/create#5", 4);
    g.set(cx.this, v::screen_x, r(0.0))?;
    // 5: screen_y = 0;
    g.at("oScreen/create#5", 5);
    g.set(cx.this, v::screen_y, r(0.0))?;
    // 6: screen_w = 320;
    g.at("oScreen/create#5", 6);
    g.set(cx.this, v::screen_w, r(320.0))?;
    // 7: screen_h = 240;
    g.at("oScreen/create#5", 7);
    g.set(cx.this, v::screen_h, r(240.0))?;
    // 8: screen_scale = global.screenScale;
    g.at("oScreen/create#5", 8);
    g.set(cx.this, v::screen_scale, g.gget(gv::screenScale)?)?;
    // 10: enabled = true;
    g.at("oScreen/create#5", 10);
    g.set(cx.this, v::enabled, r(1.0))?;
    // 13: pSurf = surface_create(screen_w, screen_h);
    g.at("oScreen/create#5", 13);
    {
        let x = { let a = [g.get(cx.this, v::screen_w)?, g.get(cx.this, v::screen_h)?]; b::surface_create(g, cx, a)? };
        g.set(cx.this, v::pSurf, x)?;
    }
    // 14: canPause = true;
    g.at("oScreen/create#5", 14);
    g.set(cx.this, v::canPause, r(1.0))?;
    // 15: paused = false;
    g.at("oScreen/create#5", 15);
    g.set(cx.this, v::paused, r(0.0))?;
    // 18: screen = surface_create(screen_w, screen_h);
    g.at("oScreen/create#5", 18);
    {
        let x = { let a = [g.get(cx.this, v::screen_w)?, g.get(cx.this, v::screen_h)?]; b::surface_create(g, cx, a)? };
        g.set(cx.this, v::screen, x)?;
    }
    // 19: darkSurf = surface_create(screen_w, screen_h);
    g.at("oScreen/create#5", 19);
    {
        let x = { let a = [g.get(cx.this, v::screen_w)?, g.get(cx.this, v::screen_h)?]; b::surface_create(g, cx, a)? };
        g.set(cx.this, v::darkSurf, x)?;
    }
    // 21: if (screen)
    g.at("oScreen/create#5", 21);
    if g.get(cx.this, v::screen)?.is_truthy() {
        // 23: surface_set_target(screen);
        g.at("oScreen/create#5", 23);
        { let a = [g.get(cx.this, v::screen)?]; b::surface_set_target(g, cx, a)? };
        // 24: draw_clear(c_black);
        g.at("oScreen/create#5", 24);
        b::draw_clear(g, cx, [k::c_black])?;
    }
    // 29: w = 320 * screen_scale;
    g.at("oScreen/create#5", 29);
    l_w.set(r(320.0).mul(&g.get(cx.this, v::screen_scale)?)?);
    // 30: h = 240 * screen_scale;
    g.at("oScreen/create#5", 30);
    l_h.set(r(240.0).mul(&g.get(cx.this, v::screen_scale)?)?);
    // 31: room_set_view(rIntro, 0, true, 0, 0, 320, 240, 0, 0, w, h, 128, 0, -1, -1, oPDummy3);
    g.at("oScreen/create#5", 31);
    b::room_set_view(g, cx, [rm::rIntro, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(128.0), r(0.0), r(-1.0), r(-1.0), obj::oPDummy3])?;
    // 32: room_set_view(rCredits1, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 32);
    b::room_set_view(g, cx, [rm::rCredits1, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 33: room_set_view(rCredits2, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 33);
    b::room_set_view(g, cx, [rm::rCredits2, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 34: room_set_view(rTitle, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 34);
    b::room_set_view(g, cx, [rm::rTitle, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 35: room_set_view(rHighscores, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 35);
    b::room_set_view(g, cx, [rm::rHighscores, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 36: room_set_view(rSun, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 36);
    b::room_set_view(g, cx, [rm::rSun, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 37: room_set_view(rMoon, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 37);
    b::room_set_view(g, cx, [rm::rMoon, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 38: room_set_view(rStars, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 38);
    b::room_set_view(g, cx, [rm::rStars, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 39: room_set_view(rLevelEditor, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, -1, -1, noone);
    g.at("oScreen/create#5", 39);
    b::room_set_view(g, cx, [rm::rLevelEditor, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(-1.0), r(-1.0), r(-4.0)])?;
    // 40: room_set_view(rLoadLevel, 0, true, 0, 0, 320, 240, 0, 0, w, h, 128, 96, -1, -1, oPlayer1);
    g.at("oScreen/create#5", 40);
    b::room_set_view(g, cx, [rm::rLoadLevel, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(128.0), r(96.0), r(-1.0), r(-1.0), obj::oPlayer1])?;
    // 41: room_set_view(rTutorial, 0, true, 0, 0, 320, 240, 0, 0, w, h, 128, 96, -1, -1, oPlayer1);
    g.at("oScreen/create#5", 41);
    b::room_set_view(g, cx, [rm::rTutorial, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(128.0), r(96.0), r(-1.0), r(-1.0), obj::oPlayer1])?;
    // 42: room_set_view(rLevel, 0, true, 0, 0, 320, 240, 0, 0, w, h, 128, 96, -1, -1, oPlayer1);
    g.at("oScreen/create#5", 42);
    b::room_set_view(g, cx, [rm::rLevel, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(128.0), r(96.0), r(-1.0), r(-1.0), obj::oPlayer1])?;
    // 43: room_set_view(rLevel2, 0, true, 0, 0, 320, 240, 0, 0, w, h, 128, 96, -1, -1, oPlayer1);
    g.at("oScreen/create#5", 43);
    b::room_set_view(g, cx, [rm::rLevel2, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(128.0), r(96.0), r(-1.0), r(-1.0), obj::oPlayer1])?;
    // 44: room_set_view(rLevel3, 0, true, 0, 0, 320, 240, 0, 0, w, h, 128, 96, -1, -1, oPlayer1);
    g.at("oScreen/create#5", 44);
    b::room_set_view(g, cx, [rm::rLevel3, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(128.0), r(96.0), r(-1.0), r(-1.0), obj::oPlayer1])?;
    // 45: room_set_view(rOlmec, 0, true, 0, 0, 320, 240, 0, 0, w, h, 128, 96, -1, -1, oPlayer1);
    g.at("oScreen/create#5", 45);
    b::room_set_view(g, cx, [rm::rOlmec, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(128.0), r(96.0), r(-1.0), r(-1.0), obj::oPlayer1])?;
    // 46: room_set_view(rTransition1, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 46);
    b::room_set_view(g, cx, [rm::rTransition1, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 47: room_set_view(rTransition1x, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 47);
    b::room_set_view(g, cx, [rm::rTransition1x, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 48: room_set_view(rTransition2, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 48);
    b::room_set_view(g, cx, [rm::rTransition2, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 49: room_set_view(rTransition2x, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 49);
    b::room_set_view(g, cx, [rm::rTransition2x, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 50: room_set_view(rTransition3, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 50);
    b::room_set_view(g, cx, [rm::rTransition3, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 51: room_set_view(rTransition3x, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 51);
    b::room_set_view(g, cx, [rm::rTransition3x, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 52: room_set_view(rTransition4, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 52);
    b::room_set_view(g, cx, [rm::rTransition4, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 53: room_set_view(rEnd, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, oPDummy);
    g.at("oScreen/create#5", 53);
    b::room_set_view(g, cx, [rm::rEnd, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), obj::oPDummy])?;
    // 54: room_set_view(rEnd2, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 54);
    b::room_set_view(g, cx, [rm::rEnd2, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 55: room_set_view(rEnd3, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 55);
    b::room_set_view(g, cx, [rm::rEnd3, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 56: room_set_view(rEndCustom, 0, true, 0, 0, 320, 240, 0, 0, w, h, 0, 0, 0, 0, noone);
    g.at("oScreen/create#5", 56);
    b::room_set_view(g, cx, [rm::rEndCustom, r(0.0), r(1.0), r(0.0), r(0.0), r(320.0), r(240.0), r(0.0), r(0.0), l_w.get()?, l_h.get()?, r(0.0), r(0.0), r(0.0), r(0.0), r(-4.0)])?;
    // 59: if (screen == -1)
    g.at("oScreen/create#5", 59);
    if g.get(cx.this, v::screen)?.gml_eq(&r(-1.0))? {
        // 59: game_end();
        g.at("oScreen/create#5", 59);
        b::game_end(g, cx, [])?;
    } else {
        // 60: room_goto_next();
        g.at("oScreen/create#5", 60);
        b::room_goto_next(g, cx, [])?;
    }
    Ok(())
}

/// Event `create` of `oScreen`: its actions in order.
pub fn create(g: &mut World, cx: &mut Ctx) -> R<()> {
    create_a0(g, cx)?;
    // #1 action_if(global.fullscreen)
    // #2 action_fullscreen(2)
    if { g.at("oScreen/create#1", 0); g.gget(gv::fullscreen)?.is_truthy() } { g.at("oScreen/create#2", 0); b::action_fullscreen(g, cx, [r(2.0)])?; }
    // #3 not action_if(global.fullscreen)
    // #4 action_fullscreen(1)
    if { g.at("oScreen/create#3", 0); !g.gget(gv::fullscreen)?.is_truthy() } { g.at("oScreen/create#4", 0); b::action_fullscreen(g, cx, [r(1.0)])?; }
    create_a5(g, cx)?;
    Ok(())
}

