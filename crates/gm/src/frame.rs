//! One GM8 frame (runtime.md section 1). Stage order and per-stage rules follow OpenGMK `game.rs`
//! (`Game::frame`), `game/events.rs`, `game/movement.rs` and `game/draw.rs`.

use crate::error::{GmlError, R};
use crate::input::KeyState;
use crate::instance::ALARMS;
use crate::list::Inst;
use crate::math;
use crate::program::Event;
use crate::room::VIEWS;
use crate::world::World;
use trace::replay::Event as InputEvent;

impl World {
    /// Runs one frame with this frame's input events (applied in order first, as `trace::replay` gives
    /// them), then performs the room change the frame requested, if any. On an error the frame stops where
    /// it was; the world is then mid-frame and shouldn't be stepped further.
    ///
    /// If a stage requests a room change, the frame stops after that stage and the room loads
    /// (`room.rs` `load_room`), frame-0 draw included. A change requested while a room was loading is left
    /// pending for the next step, as in GM8: that frame then only updates the previous positions (its
    /// first stage runs no events) before the room changes. After `game_end` a step only applies input
    /// and counts the frame, so input and frame numbers stay in step with the replay.
    pub fn step<'a>(
        &mut self,
        input: impl IntoIterator<Item = &'a InputEvent>,
    ) -> Result<(), GmlError> {
        self.input.begin_frame();
        for e in input {
            self.input.apply(e);
        }
        // GM8 shows nothing for a tick that ends with a room change pending, or that failed.
        self.presented = false;
        let r = if self.ended {
            Ok(())
        } else {
            self.frame().and_then(|()| {
                self.presented = self.pending.is_none();
                self.change_scene()
            })
        };
        if r.is_err() {
            self.presented = false;
        }
        self.frames += 1;
        r.map_err(|e| self.located(e))
    }

    /// [`step`](Self::step), recording everything the tick draws into `frame` (cleared first). Pass the
    /// same frame every tick to reuse its buffer.
    pub fn step_drawing<'a>(
        &mut self,
        input: impl IntoIterator<Item = &'a InputEvent>,
        frame: &mut draw::Frame,
    ) -> Result<(), GmlError> {
        self.begin_draw_frame(frame);
        self.draw = Some(std::mem::take(frame));
        let r = self.step(input);
        // The frame keeps the size and state it started with; a room change that resizes the window records
        // `draw::Command::ResizeScreen`.
        *frame = self.draw.take().expect("lent for the tick");
        r
    }

    fn frame(&mut self) -> R<()> {
        macro_rules! stage {
            ($e:expr) => {
                $e?;
                if self.pending.is_some() {
                    return Ok(());
                }
            };
        }
        // 1. Previous positions.
        let mut it = self.insts.iter_drawing();
        while let Some(h) = it.next(&self.insts) {
            let i = self.insts.get_mut_still(h);
            i.xprevious = i.x;
            i.yprevious = i.y;
        }
        // 2. Begin step. Spelunky has no triggers, and (3) no timelines.
        stage!(self.run_object_event(Event::BeginStep));
        // 4. Alarms.
        stage!(self.run_alarms());
        // 5. Keyboard: held, pressed, released.
        stage!(self.run_key_events(KeyState::Held));
        stage!(self.run_key_events(KeyState::Pressed));
        stage!(self.run_key_events(KeyState::Released));
        // 6. Mouse.
        // TODO(851-3178): mouse events, through `mouse_in_room`. The per-instance ones test the cursor as a
        // precise `collision_point` would. Spelunky uses them in menus and the level editor.
        // 7. Step.
        stage!(self.run_object_event(Event::Step));
        // 8. Motion. Runs no GML (Spelunky has no paths, so no path end events).
        self.motion();
        // 9. Outside room and intersect boundary.
        stage!(self.run_bound_events());
        // 10. Collisions.
        stage!((self.program.collision_stage())(self));
        // 11. End step.
        stage!(self.run_object_event(Event::EndStep));
        // 12. Cleanup.
        self.insts.remove_deleted(self.program);
        // 13. Draw. No room-change check after it or after animation: the frame is nearly over, and events
        // simply don't run while a change is pending. Then the room's background layers scroll.
        self.redraw()?;
        for b in &mut self.room.backgrounds {
            b.x += b.hspeed;
            b.y += b.vspeed;
        }
        // 14. Animation.
        self.animate()?;
        self.input.end_frame();
        Ok(())
    }

    /// Runs `e` for every holder object in id order, and each object's instances in list order.
    /// OpenGMK `game/events.rs` (`run_object_event`).
    pub(crate) fn run_object_event(&mut self, e: Event) -> R<()> {
        for &(object, handler) in self.program.holders(e) {
            let mut it = self.insts.iter_object(object);
            while let Some(h) = it.next(&self.insts) {
                self.run_handler(h, h, handler)?;
            }
        }
        Ok(())
    }

    /// For each alarm in order, for each holder of that alarm event, for each instance: count a set alarm
    /// down and fire it when it reaches 0. So `alarm[n] = 1` fires next frame, `alarm[n] = 0` never fires,
    /// deactivated instances' alarms are frozen, and an alarm of an object without that alarm event never
    /// counts down. OpenGMK `game/events.rs` (`run_alarms`).
    fn run_alarms(&mut self) -> R<()> {
        for n in 0..ALARMS {
            for &(object, handler) in self.program.holders(Event::Alarm(n as u8)) {
                let mut it = self.insts.iter_object(object);
                while let Some(h) = it.next(&self.insts) {
                    let a = &mut self.insts.get_mut_still(h).alarms[n];
                    if *a >= 0 {
                        *a -= 1;
                        if *a == 0 {
                            self.run_handler(h, h, handler)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// One keyboard stage: the events of every key in this state, by key code, then "any key" or "no key".
    /// The key-code pass includes the "no key" (0) and "any key" (1) events too, so those run twice.
    /// OpenGMK does exactly this (`game/events.rs` `run_keyboard_events` and siblings); verify against
    /// the oracle. Spelunky only has key-press events on real keys.
    fn run_key_events(&mut self, state: KeyState) -> R<()> {
        let event: fn(u8) -> Event = match state {
            KeyState::Held => Event::Keyboard,
            KeyState::Pressed => Event::KeyPress,
            KeyState::Released => Event::KeyRelease,
        };
        let program = self.program;
        for (e, holders) in program.holders_range(event(0), event(u8::MAX)) {
            let (Event::Keyboard(vk) | Event::KeyPress(vk) | Event::KeyRelease(vk)) = e else {
                unreachable!("key events only");
            };
            if !self.input.check(state, vk) {
                continue;
            }
            for &(object, handler) in holders {
                let mut it = self.insts.iter_object(object);
                while let Some(h) = it.next(&self.insts) {
                    self.run_handler(h, h, handler)?;
                }
            }
        }
        let sub = if self.input.any(state) {
            crate::input::VK_ANYKEY
        } else {
            crate::input::VK_NOKEY
        };
        self.run_object_event(event(sub))
    }

    /// Friction and gravity for every instance, then every instance moves by hspeed/vspeed. OpenGMK
    /// `game/movement.rs` (`process_speeds`, `apply_speeds`).
    fn motion(&mut self) {
        let mut it = self.insts.iter_drawing();
        while let Some(h) = it.next(&self.insts) {
            let i = self.insts.get_mut_still(h);
            if i.friction != 0.0 {
                // Friction pulls speed towards 0 without crossing it. Exactly OpenGMK's branches, so a
                // negative friction does nothing at speed 0 but speeds up a moving instance.
                let (s, f) = (i.speed, i.friction);
                if s >= 0.0 {
                    if f > s {
                        i.set_speed(0.0);
                    } else if s != 0.0 {
                        i.set_speed(s - f);
                    }
                } else if f > -s {
                    i.set_speed(0.0);
                } else {
                    i.set_speed(s + f);
                }
            }
            if i.gravity != 0.0 {
                let d = i.gravity_direction.to_radians();
                let (hs, vs) = (
                    i.hspeed + math::cos(d) * i.gravity,
                    i.vspeed - math::sin(d) * i.gravity,
                );
                i.set_hvspeed(hs, vs);
            }
        }
        let mut it = self.insts.iter_drawing();
        while let Some(h) = it.next(&self.insts) {
            self.apply_speeds(h);
        }
    }

    /// `x += hspeed; y += vspeed`, if either is nonzero. Also used by the collision stage's solid rule.
    /// OpenGMK `game/movement.rs` (`apply_speeds`; Spelunky has no paths).
    pub(crate) fn apply_speeds(&mut self, h: Inst) {
        let i = self.insts.get(h);
        if i.hspeed != 0.0 || i.vspeed != 0.0 {
            let i = self.insts.get_mut(h);
            i.x += i.hspeed;
            i.y += i.vspeed;
        }
    }

    /// Outside-room and intersect-boundary events against the room, then outside-view and intersect-view
    /// events against each of the 8 views (visible or not, views on or not), in that order. OpenGMK
    /// `game/events.rs` (`run_bound_events`).
    fn run_bound_events(&mut self) -> R<()> {
        let room = |w: &World| (0, 0, w.room.width, w.room.height);
        self.run_rect_events(Event::OUTSIDE_ROOM, false, room)?;
        self.run_rect_events(Event::INTERSECT_BOUNDARY, true, room)?;
        let view = |n: usize| {
            move |w: &World| {
                let v = &w.room.views[n];
                (v.xview, v.yview, v.xview + v.wview, v.yview + v.hview)
            }
        };
        for n in 0..VIEWS {
            self.run_rect_events(Event::outside_view(n as u8), false, view(n))?;
        }
        for n in 0..VIEWS {
            self.run_rect_events(Event::intersect_view(n as u8), true, view(n))?;
        }
        Ok(())
    }

    /// Runs `event` for each instance outside the rectangle `(x1, y1, x2, y2)` (read again for each
    /// instance), or with `intersect`, not entirely inside it: GM8's "intersect" test, so it also fires for
    /// instances entirely outside (OpenGMK `instance_intersect_rect`). Without a mask, both tests check
    /// that the floor and the ceiling of the position are inside.
    fn run_rect_events(
        &mut self,
        event: Event,
        intersect: bool,
        rect: impl Fn(&World) -> (i32, i32, i32, i32),
    ) -> R<()> {
        for &(object, handler) in self.program.holders(event) {
            let mut it = self.insts.iter_object(object);
            while let Some(h) = it.next(&self.insts) {
                let (x1, y1, x2, y2) = rect(self);
                let hit = if !self.has_mask(h) {
                    let i = self.instance(h);
                    (i.x.floor() as i32) < x1
                        || (i.y.floor() as i32) < y1
                        || (i.x.ceil() as i32) > x2
                        || (i.y.ceil() as i32) > y2
                } else {
                    let b = self.bbox(h);
                    if intersect {
                        b.left < x1 || b.top < y1 || b.right > x2 || b.bottom > y2
                    } else {
                        b.right < x1 || b.bottom < y1 || b.left > x2 || b.top > y2
                    }
                };
                if hit {
                    self.run_handler(h, h, handler)?;
                }
            }
        }
        Ok(())
    }

    /// Advances every instance's animation; one that reaches its sprite's frame count wraps by
    /// subtracting it and gets its animation end event. Without a sprite the frame count is 0, so a
    /// non-negative `image_speed` fires animation end every frame. OpenGMK `game.rs` (`Game::frame`, after
    /// `draw`).
    fn animate(&mut self) -> R<()> {
        let mut it = self.insts.iter_drawing();
        while let Some(h) = it.next(&self.insts) {
            let frames = self.frame_count(self.insts.get(h).sprite_index) as f64;
            let i = self.insts.get_mut_still(h);
            let next = i.image_index + i.image_speed;
            i.image_index = next;
            if next >= frames {
                i.image_index = next - frames;
                self.run_event(h, h, Event::ANIMATION_END)?;
            }
        }
        Ok(())
    }
}
