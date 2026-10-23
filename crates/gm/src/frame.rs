//! One GM8 frame (runtime.md section 1). Stage order and per-stage rules follow OpenGMK `game.rs`
//! (`Game::frame`), `game/events.rs`, `game/movement.rs` and `game/draw.rs`.

use crate::error::{GmlError, R};
use crate::input::KeyState;
use crate::instance::ALARMS;
use crate::list::Inst;
use crate::math;
use crate::program::Event;
use crate::world::World;
use trace::replay::Event as InputEvent;

impl World {
    /// Runs one frame with this frame's input events (applied in order first, as `trace::replay` gives
    /// them). On an error the frame stops where it was; the world is then mid-frame and shouldn't be
    /// stepped further.
    ///
    /// If a stage requests a room change, the frame stops after that stage. The change itself happens
    /// between frames (851-3128; until then the host performs it, see [`World::take_scene_change`]). GM8
    /// never runs a frame with a change still pending; if the host steps anyway, the frame's input is still
    /// applied and the frame still counts, so input and frame numbers stay in step with the replay, but no
    /// stage runs.
    pub fn step<'a>(
        &mut self,
        input: impl IntoIterator<Item = &'a InputEvent>,
    ) -> Result<(), GmlError> {
        self.input.begin_frame();
        for e in input {
            self.input.apply(e);
        }
        let r = if self.pending.is_some() {
            Ok(())
        } else {
            self.frame()
        };
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
            let i = self.insts.get_mut(h);
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
        // TODO(851-3128): mouse events, which need the view mapping. The per-instance ones test the cursor
        // as a precise `collision_point` would. Spelunky uses them in menus and the level editor.
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
        // simply don't run while a change is pending.
        self.redraw()?;
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
                    let a = &mut self.insts.get_mut(h).alarms[n];
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
            let i = self.insts.get_mut(h);
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

    /// Outside-room and intersect-boundary events: the bounding box against the room, or the position if
    /// the instance has no mask. OpenGMK `game/events.rs` (`run_bound_events`).
    // TODO(851-3128): outside-view and intersect-view events (other 40-47, 50-57) once views exist.
    fn run_bound_events(&mut self) -> R<()> {
        let (w, hgt) = (self.room.width, self.room.height);
        // Without a mask, both the floor and the ceiling of the position must be inside.
        let point_outside = |x: f64, y: f64| {
            (x.floor() as i32) < 0
                || (y.floor() as i32) < 0
                || (x.ceil() as i32) > w
                || (y.ceil() as i32) > hgt
        };
        for (event, intersect) in [
            (Event::OUTSIDE_ROOM, false),
            (Event::INTERSECT_BOUNDARY, true),
        ] {
            for &(object, handler) in self.program.holders(event) {
                let mut it = self.insts.iter_object(object);
                while let Some(h) = it.next(&self.insts) {
                    let hit = match self.bbox(h) {
                        // GM8's "intersect" test is "not entirely inside", so it also fires for
                        // instances entirely outside the room (OpenGMK `instance_intersect_rect`).
                        Some(b) if intersect => {
                            b.left < 0 || b.top < 0 || b.right > w || b.bottom > hgt
                        }
                        Some(b) => b.right < 0 || b.bottom < 0 || b.left > w || b.top > hgt,
                        None => {
                            let i = self.instance(h);
                            point_outside(i.x, i.y)
                        }
                    };
                    if hit {
                        self.run_handler(h, h, handler)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// The draw stage, also `screen_redraw()`: sorts the instance list by depth (which changes iteration
    /// order everywhere until the next sort), then runs each visible instance's draw event, or draws its
    /// sprite if its object has none. OpenGMK `game/draw.rs` (`Game::draw`, `draw_view`).
    // TODO(851-3128): views follow their targets first, and the instances are drawn once per visible view.
    pub fn redraw(&mut self) -> R<()> {
        self.insts.draw_sort();
        let program = self.program;
        let mut it = self.insts.iter_drawing();
        while let Some(h) = it.next(&self.insts) {
            let i = self.insts.get(h);
            if !i.visible {
                continue;
            }
            match program.handler(i.object, Event::Draw) {
                Some(&handler) => self.run_handler(h, h, handler)?,
                None if self.recording_draw() && self.assets.sprite(i.sprite_index).is_some() => {
                    // The default draw: the sprite at floor(image_index) with the instance's transform.
                    // OpenGMK `game/draw.rs` (`draw_instance_default`). It isn't an event, so it happens
                    // even after a draw event requested a room change; GM8 then doesn't show the frame
                    // (`World::presents`).
                    let cmd = draw::Sprite {
                        sprite: i.sprite_index as u32,
                        frame: i.image_index.floor() as i64 as i32,
                        x: i.x,
                        y: i.y,
                        xscale: i.image_xscale,
                        yscale: i.image_yscale,
                        angle: i.image_angle,
                        blend: draw::Color(i.image_blend as u32),
                        alpha: i.image_alpha,
                    };
                    self.push_draw(draw::Command::Sprite(cmd));
                }
                None => {}
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
            let i = self.insts.get_mut(h);
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
