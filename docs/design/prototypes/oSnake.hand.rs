// Hand-port prototype of oSnake (create + step), for the comparison in docs/design/translation.md.
// Illustrative only: not compiled. It targets a typed, idiomatic API (`World`, typed instance structs)
// the way a careful human port would, and marks with FIDELITY every place where the idiomatic version
// silently differs from GM8 or needs the porter to know a runner rule.

/// Per-snake state. Fields inherited from oEnemy's create event live in `enemy`.
#[derive(Clone)]
pub struct Snake {
    pub enemy: Enemy,
    pub status: SnakeStatus,
    pub counter: i32,
    pub facing: Facing,
    pub bounced: bool,
    pub dead: bool,
    pub shake_counter: i32,
    pub shake_toggle: i32,
}

// FIDELITY: in GML these are *instance variables* (`IDLE = 0; ... DEAD = 99;` in the create event), not
// constants. Other objects read them through instance references (oPlayer1 reads `obj.facing == 18` on
// held items, whose LEFT/RIGHT differ per object family). Turning them into a Rust enum is only correct
// after proving no code writes or cross-reads them. The transpiler keeps them as variables.
#[derive(Clone, Copy, PartialEq)]
pub enum SnakeStatus {
    Idle = 0,
    Walk = 1,
    Attack = 2,
    Stunned = 98,
    Dead = 99,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Facing {
    Left = 0,
    Right = 1,
}

pub fn create(w: &mut World, me: Inst) {
    // FIDELITY: action 0 is `action_inherited` -> oEnemy create -> oDrawnSprite create. The order of the
    // parent's assignments vs ours matters (we override hp, type, LEFT/RIGHT...).
    enemy::create(w, me);
    make_active(w, me);
    set_collision_bounds(w, me, 2.0, 0.0, 14.0, 16.0);
    let s = w.snake_mut(me);
    s.enemy.x_vel = 2.5;
    s.enemy.base.image_speed = 0.4;
    s.enemy.kind = "Snake";
    s.enemy.hp = 1.0;
    s.enemy.invincible = false;
    s.status = SnakeStatus::Idle;
    s.bounced = false;
    s.dead = false;
    s.counter = 0;
    s.facing = Facing::Right;
    s.shake_counter = 0;
    s.shake_toggle = 1;
}

pub fn step(w: &mut World, me: Inst) {
    enemy::step(w, me); // action_inherited
    let view = w.view(0);
    let (x, y) = w.pos(me);
    // FIDELITY: GM8 compares reals with an epsilon (`gml_gt` is `diff >= CMP_EPSILON`), so `x > a` is
    // false when x exceeds a by less than the epsilon. Plain f64 `>` differs at the boundary.
    if !(x > view.x - 20.0 && x < view.x + view.w + 4.0 && y > view.y - 20.0 && y < view.y + view.h + 4.0) {
        return;
    }
    let (xv, yv) = { let s = w.snake(me); (s.enemy.x_vel, s.enemy.y_vel) };
    move_to(w, me, xv, yv);
    {
        let s = w.snake_mut(me);
        s.enemy.y_vel += s.enemy.my_grav;
        if s.enemy.y_vel > s.enemy.y_vel_limit {
            s.enemy.y_vel = s.enemy.y_vel_limit;
        }
    }
    let (x, y) = w.pos(me);
    if w.collision_point(x + 8.0, y + 8.0, Obj::oSolid, false, false).is_some() {
        w.snake_mut(me).enemy.hp = 0.0;
    }
    if w.snake(me).enemy.hp < 1.0 {
        scr_create_blood(w, me, x + 8.0, y + 8.0, 3);
        if w.snake(me).enemy.counts_as_kill {
            if is_real_level(w) {
                w.globals.enemy_kills[1] += 1.0;
            }
            w.globals.snakes += 1.0;
            w.globals.kills += 1.0;
        }
        // FIDELITY: GM8 runs the destroy event and marks the instance deleted, but this event keeps
        // running: the rest of the step (status logic, sprite updates, more RNG draws) still executes
        // on a dead snake. An idiomatic port tends to `return` here, which skips the `rand(1,100)` below
        // and shifts the global RNG stream.
        w.instance_destroy(me);
    }
    // FIDELITY: `isCollisionBottom(1) and status != STUNNED`: GM8 evaluates both operands. Harmless here
    // (the script call is on the left), but the porter has to check every `and`/`or` in the game.
    if is_collision_bottom(w, me, 1.0) && w.snake(me).status != SnakeStatus::Stunned {
        w.snake_mut(me).enemy.y_vel = 0.0;
    }
    match w.snake(me).status {
        SnakeStatus::Idle => {
            if w.snake(me).counter > 0 {
                w.snake_mut(me).counter -= 1;
            } else {
                // FIDELITY: `rand` is a script that also writes `randNum` on the calling instance.
                let f = rand(w, me, 0, 1);
                let s = w.snake_mut(me);
                s.facing = if f == 0 { Facing::Left } else { Facing::Right };
                s.status = SnakeStatus::Walk;
            }
        },
        SnakeStatus::Walk => {
            if is_collision_left(w, me, 1.0) | is_collision_right(w, me, 1.0) {
                let s = w.snake_mut(me);
                s.facing = if s.facing == Facing::Left { Facing::Right } else { Facing::Left };
            }
            let (x, y) = w.pos(me);
            // FIDELITY: precise = -1 is *false* in GM8 (a real below 0.5), so these are bounding-box
            // checks. A porter reading "-1" as "true/precise" gets per-pixel checks.
            let solid = |w: &World, px: f64, py: f64| w.collision_point(px, py, Obj::oSolid, false, false).is_some();
            if w.snake(me).facing == Facing::Left && !solid(w, x - 1.0, y + 16.0) {
                w.snake_mut(me).facing = Facing::Right;
            } else if w.snake(me).facing == Facing::Right && !solid(w, x + 16.0, y + 16.0) {
                w.snake_mut(me).facing = Facing::Left;
            }
            if (!solid(w, x - 1.0, y + 16.0) || solid(w, x - 1.0, y))
                && (!solid(w, x + 16.0, y + 16.0) || solid(w, x + 16.0, y))
            {
                let wall_left = solid(w, x - 1.0, y);
                let s = w.snake_mut(me);
                s.facing = if wall_left { Facing::Right } else { Facing::Left };
                s.enemy.x_vel = 0.0;
            } else if w.snake(me).facing == Facing::Left {
                w.snake_mut(me).enemy.x_vel = -1.0;
            } else {
                w.snake_mut(me).enemy.x_vel = 1.0;
            }
            if rand(w, me, 1, 100) == 1 {
                let c = rand(w, me, 20, 50);
                let s = w.snake_mut(me);
                s.status = SnakeStatus::Idle;
                s.counter = c;
                s.enemy.x_vel = 0.0;
            }
        },
        _ => {},
    }
    if is_collision_solid(w, me) {
        w.pos_mut(me).1 -= 2.0;
    }
    let s = w.snake_mut(me);
    if s.status != SnakeStatus::Stunned {
        s.enemy.base.image_speed = if s.enemy.x_vel == 0.0 { 0.2 } else { 0.4 };
        // FIDELITY: assigning sprite_index is a runner operation (it can change image_number and the
        // collision mask), not a plain field write.
        w.set_sprite(me, Spr::sSnakeWalkL);
    }
}
