// ==== action 0: lib 1 id 603: code ====
scrReadStats();
scrInit();
// ==== action 1: lib 1 id 408: action_if(global.fullscreen) ====
// ==== action 2: lib 1 id 531: action_fullscreen(2) ====
// ==== action 3: lib 1 id 408: NOT action_if(global.fullscreen) ====
// ==== action 4: lib 1 id 531: action_fullscreen(1) ====
// ==== action 5: lib 1 id 603: code ====
py = 0; // player Y coord

// screen base(view_wview and view_hview)
screen_x = 0;
screen_y = 0;
screen_w = 320;
screen_h = 240;
screen_scale = global.screenScale;

enabled = true;

// pause
pSurf = surface_create(screen_w,screen_h);
canPause = true;
paused = false;

// create a surface for the whole screen to be drawn on
screen = surface_create(screen_w,screen_h);
darkSurf = surface_create(screen_w,screen_h);

if (screen)
{
    surface_set_target(screen);
    draw_clear(c_black);
}

// set up rooms
var w,h;
w = 320 * screen_scale;
h = 240 * screen_scale;
room_set_view(rIntro,0,true,0,0,320,240,0,0,w,h,128,0,-1,-1,oPDummy3);
room_set_view(rCredits1,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rCredits2,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rTitle,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rHighscores,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rSun,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rMoon,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rStars,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rLevelEditor,0,true,0,0,320,240,0,0,w,h,0,0,-1,-1,noone);
room_set_view(rLoadLevel,0,true,0,0,320,240,0,0,w,h,128,96,-1,-1,oPlayer1);
room_set_view(rTutorial,0,true,0,0,320,240,0,0,w,h,128,96,-1,-1,oPlayer1);
room_set_view(rLevel,0,true,0,0,320,240,0,0,w,h,128,96,-1,-1,oPlayer1);
room_set_view(rLevel2,0,true,0,0,320,240,0,0,w,h,128,96,-1,-1,oPlayer1);
room_set_view(rLevel3,0,true,0,0,320,240,0,0,w,h,128,96,-1,-1,oPlayer1);
room_set_view(rOlmec,0,true,0,0,320,240,0,0,w,h,128,96,-1,-1,oPlayer1);
room_set_view(rTransition1,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rTransition1x,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rTransition2,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rTransition2x,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rTransition3,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rTransition3x,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rTransition4,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rEnd,0,true,0,0,320,240,0,0,w,h,0,0,0,0,oPDummy);
room_set_view(rEnd2,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rEnd3,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);
room_set_view(rEndCustom,0,true,0,0,320,240,0,0,w,h,0,0,0,0,noone);

// this will destroy the screen object if surfaces are not supported on the graphics card, reverting to the viewport method
if (screen == -1) game_end();
else room_goto_next();
