//! Логіка Crystal Arkanoid: м'ячі, цеглини, рівні, combo, power-upи, shake.
//! Платформонезалежна — не знає про wgpu/winit.

mod constants;
mod levels;
mod types;

pub use constants::{
    BALL_R, BRICK_H, BRICK_W, FIXED_DT, PADDLE_H, PADDLE_Y, WALL, WORLD_H, WORLD_W, MAX_LEVEL,
};
pub use types::{Ball, Brick, Drop, GameState, Input, Laser, Powerup};

use engine::math::{circle_aabb, clamp, Aabb, Vec2};
use crate::particles::ParticleSystem;
use constants::{
    BALL_SPEED_INC, BALL_SPEED_MAX, BALL_SPEED_START, BRICK_COLS, BRICK_GAP, BRICK_HPAD, BRICK_ROWS,
    BRICK_TOP, BRICK_VGAP, DROP_CHANCE, DROP_SPEED, EXPAND_TIME, LASER_INTERVAL, LASER_SPEED,
    LASER_TIME, MAX_BALLS, MAX_LIVES, MAX_STORED_LIVES, PADDLE_SPEED, PADDLE_W, PADDLE_W_WIDE,
    PIERCE_TIME, SLOW_TIME,
};
use engine::audio::Sfx;

pub struct Game {
    pub state: GameState,
    pub score: u32,
    pub lives: u32,
    pub level: u32,
    pub combo: u32,
    pub paddle_x: f32,
    pub balls: Vec<Ball>,
    pub bricks: Vec<Brick>,
    pub drops: Vec<Drop>,
    pub lasers: Vec<Laser>,
    pub particles: ParticleSystem,
    pub shake: f32,
    pub time: f32,
    pub fps: f32,
    pub expand_t: f32,
    pub slow_t: f32,
    pub pierce_t: f32,
    pub laser_t: f32,
    pub immortal: bool,
    pub cam_lean_x: f32,
    pub cam_lean_y: f32,
    hit_stop: f32,
    trail_acc: f32,
    laser_cd: f32,
    rng_state: u32,
    sfx: Vec<Sfx>,
}

impl Game {
    pub fn new() -> Self {
        let mut g = Self {
            state: GameState::Ready,
            score: 0,
            lives: MAX_LIVES,
            level: 1,
            combo: 0,
            paddle_x: WORLD_W * 0.5,
            balls: Vec::with_capacity(MAX_BALLS),
            bricks: Vec::with_capacity(BRICK_COLS * BRICK_ROWS),
            drops: Vec::new(),
            lasers: Vec::new(),
            particles: ParticleSystem::new(),
            shake: 0.0,
            time: 0.0,
            fps: 0.0,
            expand_t: 0.0,
            slow_t: 0.0,
            pierce_t: 0.0,
            laser_t: 0.0,
            immortal: false,
            cam_lean_x: 0.0,
            cam_lean_y: 0.0,
            hit_stop: 0.0,
            trail_acc: 0.0,
            laser_cd: 0.0,
            rng_state: 0x1234_5678,
            sfx: Vec::new(),
        };
        g.load_level(1);
        g.stick_balls();
        g
    }

    pub fn paddle_w(&self) -> f32 {
        if self.expand_t > 0.0 {
            PADDLE_W_WIDE
        } else {
            PADDLE_W
        }
    }

    fn next_rand(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        (x >> 8) as f32 / 16_777_216.0
    }

    fn impulse(&mut self, amount: f32) {
        self.shake = (self.shake + amount).min(1.4);
    }

    fn push_sfx(&mut self, s: Sfx) {
        let same = self.sfx.iter().filter(|x| **x == s).count();
        if same < 3 && self.sfx.len() < 16 {
            self.sfx.push(s);
        }
    }

    pub fn drain_sfx(&mut self) -> Vec<Sfx> {
        std::mem::take(&mut self.sfx)
    }

    fn load_level(&mut self, level: u32) {
        self.level = level.clamp(1, MAX_LEVEL);
        self.bricks.clear();
        self.drops.clear();
        self.lasers.clear();
        self.combo = 0;
        self.expand_t = 0.0;
        self.slow_t = 0.0;
        self.pierce_t = 0.0;
        self.laser_t = 0.0;
        for row in 0..BRICK_ROWS {
            for col in 0..BRICK_COLS {
                let cell = levels::level_cell(self.level, row, col);
                if cell == 0 {
                    continue;
                }
                let unbreakable = cell == 9;
                let hp = if unbreakable { 1 } else { cell };
                self.bricks.push(Brick {
                    x: BRICK_HPAD + col as f32 * (BRICK_W + BRICK_GAP) + BRICK_W * 0.5,
                    y: BRICK_TOP + row as f32 * (BRICK_H + BRICK_VGAP) + BRICK_H * 0.5,
                    hp,
                    max_hp: hp,
                    alive: true,
                    unbreakable,
                    flash: 0.0,
                });
            }
        }
    }

    /// Читерський стрибок на рівень (клавіші 1–5). Рахунок і життя лишаються.
    pub fn goto_level(&mut self, level: u32) {
        self.particles.clear();
        self.shake = 0.0;
        self.load_level(level);
        self.state = GameState::Ready;
        self.stick_balls();
    }

    fn stick_balls(&mut self) {
        self.balls.clear();
        self.balls.push(Ball {
            pos: Vec2::new(self.paddle_x, PADDLE_Y - PADDLE_H * 0.5 - BALL_R - 1.0),
            vel: Vec2::ZERO,
        });
    }

    fn launch_balls(&mut self) {
        if self.balls.is_empty() {
            self.stick_balls();
        }
        let speed = self.ball_speed();
        let n = self.balls.len();
        for i in 0..n {
            if self.balls[i].vel.length() < 1.0 {
                let wobble = (i as f32 - 0.5) * 12.0;
                let r = self.next_rand();
                let angle = (-80.0 + r * 40.0 + wobble).to_radians();
                self.balls[i].vel = Vec2::new(angle.cos(), angle.sin()).scale(speed);
            }
        }
        self.state = GameState::Playing;
        self.push_sfx(Sfx::Launch);
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn overlay_msg(&self) -> &'static str {
        match self.state {
            GameState::Ready if self.immortal => "PRACTICE  —  tap / space to launch",
            GameState::Ready => "TAP / SPACE  —  launch",
            GameState::Playing => "",
            GameState::GameOver => "GAME OVER  —  tap to retry",
            GameState::Win => "YOU WIN  —  tap to play again",
        }
    }

    pub fn toggle_immortal(&mut self) {
        self.immortal = !self.immortal;
        if self.immortal && self.state == GameState::GameOver {
            self.state = GameState::Ready;
            if self.lives == 0 {
                self.lives = 1;
            }
            self.stick_balls();
        }
    }

    fn ball_speed(&self) -> f32 {
        let lvl_bonus = (self.level.saturating_sub(1) as f32) * 28.0;
        let s = BALL_SPEED_START + BALL_SPEED_INC * (self.score / 10) as f32 + lvl_bonus;
        let s = clamp(s, BALL_SPEED_START, BALL_SPEED_MAX);
        if self.slow_t > 0.0 {
            s * 0.55
        } else {
            s
        }
    }

    fn lose_last_ball(&mut self) {
        self.combo = 0;
        self.push_sfx(Sfx::LifeLost);
        if self.lives > 0 {
            self.lives -= 1;
        }
        self.drops.clear();
        if self.lives == 0 {
            self.state = GameState::GameOver;
        } else {
            self.state = GameState::Ready;
            self.stick_balls();
        }
    }

    pub fn restart(&mut self) {
        self.score = 0;
        self.lives = MAX_LIVES;
        self.state = GameState::Ready;
        self.paddle_x = WORLD_W * 0.5;
        self.shake = 0.0;
        self.particles.clear();
        self.load_level(1);
        self.stick_balls();
    }

    pub fn update(&mut self, dt: f32, input: &Input) {
        self.apply_input(dt, input);
        let k = 1.0 - (-10.0 * dt).exp();
        self.cam_lean_x += (input.tilt_x - self.cam_lean_x) * k;
        self.cam_lean_y += (input.tilt_y - self.cam_lean_y) * k;
        self.time += dt;
        self.shake = (self.shake * (-10.0 * dt).exp()).max(0.0);
        for b in &mut self.bricks {
            b.flash = (b.flash - dt * 4.5).max(0.0);
        }

        self.expand_t = (self.expand_t - dt).max(0.0);
        self.slow_t = (self.slow_t - dt).max(0.0);
        self.pierce_t = (self.pierce_t - dt).max(0.0);
        self.laser_t = (self.laser_t - dt).max(0.0);
        self.laser_cd = (self.laser_cd - dt).max(0.0);

        if self.state == GameState::Win {
            self.particles.fireworks(dt);
        }

        if self.hit_stop > 0.0 {
            self.hit_stop -= dt;
            return;
        }

        match self.state {
            GameState::Ready => {
                self.stick_balls();
            }
            GameState::Playing => {
                self.update_drops(dt);
                self.update_lasers(dt);
                self.trail_acc += dt;
                if self.trail_acc >= 0.018 {
                    self.trail_acc = 0.0;
                    let pierce = self.pierce_t > 0.0;
                    for b in &self.balls {
                        let c = if pierce {
                            [0.95, 0.55, 1.0, 1.0]
                        } else {
                            [0.75, 0.95, 1.0, 1.0]
                        };
                        self.particles.trail(b.pos.x, b.pos.y, c);
                    }
                }
                let n = self.balls.len();
                for i in 0..n {
                    if i >= self.balls.len() {
                        break;
                    }
                    let speed = self.balls[i].vel.length();
                    let steps = ((speed * dt) / (BALL_R * 0.5)).ceil().max(1.0) as u32;
                    let sub = dt / steps as f32;
                    for _ in 0..steps {
                        if i >= self.balls.len() {
                            break;
                        }
                        self.step_ball(i, sub);
                        if self.state != GameState::Playing {
                            break;
                        }
                    }
                    if self.state != GameState::Playing {
                        break;
                    }
                }
                if self.state == GameState::Playing && self.balls.is_empty() {
                    self.lose_last_ball();
                }
            }
            GameState::GameOver | GameState::Win => {}
        }
    }

    pub fn handle_launch(&mut self, launch: bool) {
        if !launch {
            return;
        }
        match self.state {
            GameState::Ready => self.launch_balls(),
            GameState::Playing => {}
            GameState::GameOver | GameState::Win => self.restart(),
        }
    }

    fn apply_input(&mut self, dt: f32, input: &Input) {
        let half = self.paddle_w() * 0.5;
        let min_x = WALL + half;
        let max_x = WORLD_W - WALL - half;

        if let Some(px) = input.pointer_x {
            self.paddle_x = clamp(px, min_x, max_x);
        } else if input.tilt_x.abs() > 0.06 {
            let target = WORLD_W * 0.5 + input.tilt_x * (WORLD_W * 0.5 - WALL - half);
            let k = 1.0 - (-8.0 * dt).exp();
            self.paddle_x = clamp(self.paddle_x + (target - self.paddle_x) * k, min_x, max_x);
        }
        if input.axis.abs() > 0.01 {
            self.paddle_x = clamp(
                self.paddle_x + input.axis * PADDLE_SPEED * dt,
                min_x,
                max_x,
            );
        }
        self.handle_launch(input.launch);
    }

    fn update_drops(&mut self, dt: f32) {
        let half = self.paddle_w() * 0.5;
        let paddle = Aabb::new(
            Vec2::new(self.paddle_x, PADDLE_Y),
            Vec2::new(half, PADDLE_H * 0.5),
        );
        let mut i = 0;
        while i < self.drops.len() {
            self.drops[i].pos.y += DROP_SPEED * dt;
            let d = self.drops[i];
            if d.pos.y > WORLD_H + 20.0 {
                self.drops.swap_remove(i);
                continue;
            }
            let hit = circle_aabb(d.pos, 9.0, paddle).is_some();
            if hit {
                let kind = d.kind;
                let pos = d.pos;
                self.drops.swap_remove(i);
                self.apply_powerup(kind, pos);
                continue;
            }
            i += 1;
        }
    }

    fn apply_powerup(&mut self, kind: Powerup, pos: Vec2) {
        self.particles.pickup_burst(pos.x, pos.y, kind.color());
        self.push_sfx(Sfx::Pickup);
        match kind {
            Powerup::Expand => self.expand_t = EXPAND_TIME,
            Powerup::Slow => self.slow_t = SLOW_TIME,
            Powerup::Pierce => self.pierce_t = PIERCE_TIME,
            Powerup::ExtraLife => self.lives = (self.lives + 1).min(MAX_STORED_LIVES),
            Powerup::Multiball => self.spawn_multiball(),
            Powerup::Laser => self.laser_t = LASER_TIME,
        }
    }

    fn spawn_multiball(&mut self) {
        if self.balls.is_empty() {
            return;
        }
        let speed = self.ball_speed().max(BALL_SPEED_START);
        let sources = self.balls.clone();
        for k in 0..3 {
            if self.balls.len() >= MAX_BALLS {
                break;
            }
            let src = sources[k % sources.len()];
            let a = (-108.0 + k as f32 * 22.0 + self.next_rand() * 10.0).to_radians();
            self.balls.push(Ball {
                pos: src.pos,
                vel: Vec2::new(a.cos(), a.sin()).scale(speed),
            });
        }
    }

    fn maybe_drop(&mut self, x: f32, y: f32) {
        if self.next_rand() > DROP_CHANCE {
            return;
        }
        let r = self.next_rand();
        let kind = if r < 0.12 {
            Powerup::ExtraLife
        } else if r < 0.30 {
            Powerup::Multiball
        } else if r < 0.46 {
            Powerup::Expand
        } else if r < 0.62 {
            Powerup::Laser
        } else if r < 0.80 {
            Powerup::Pierce
        } else {
            Powerup::Slow
        };
        self.drops.push(Drop {
            pos: Vec2::new(x, y),
            kind,
        });
    }

    fn update_lasers(&mut self, dt: f32) {
        if self.laser_t > 0.0 && self.laser_cd <= 0.0 && self.lasers.len() < 28 {
            let half = self.paddle_w() * 0.5 - 14.0;
            let y = PADDLE_Y - PADDLE_H * 0.5 - 8.0;
            self.lasers.push(Laser {
                x: self.paddle_x - half,
                y,
            });
            self.lasers.push(Laser {
                x: self.paddle_x + half,
                y,
            });
            self.laser_cd = LASER_INTERVAL;
            self.push_sfx(Sfx::Laser);
            self.particles.sparks(self.paddle_x, y, [1.0, 0.82, 0.35, 1.0], 40, 90.0);
        }

        let mut i = 0;
        while i < self.lasers.len() {
            self.lasers[i].y -= LASER_SPEED * dt;
            let lx = self.lasers[i].x;
            let ly = self.lasers[i].y;
            if ly < WALL {
                self.lasers.swap_remove(i);
                continue;
            }
            let mut hit_i = None;
            for (bi, brick) in self.bricks.iter().enumerate() {
                if !brick.alive {
                    continue;
                }
                let aabb = Aabb::new(
                    Vec2::new(brick.x, brick.y),
                    Vec2::new(BRICK_W * 0.5, BRICK_H * 0.5),
                );
                if circle_aabb(Vec2::new(lx, ly), 5.0, aabb).is_some() {
                    hit_i = Some(bi);
                    break;
                }
            }
            if let Some(bi) = hit_i {
                self.lasers.swap_remove(i);
                self.damage_brick(bi);
                continue;
            }
            i += 1;
        }
    }

    fn step_ball(&mut self, i: usize, dt: f32) {
        {
            let b = &mut self.balls[i];
            b.pos = b.pos.add(b.vel.scale(dt));
        }
        let pos = self.balls[i].pos;

        if pos.x < WALL + BALL_R {
            self.balls[i].pos.x = WALL + BALL_R;
            self.balls[i].vel.x = -self.balls[i].vel.x;
            self.particles.sparks(pos.x, pos.y, [0.7, 0.9, 1.0, 1.0], 60, 140.0);
            self.push_sfx(Sfx::Wall);
        } else if pos.x > WORLD_W - WALL - BALL_R {
            self.balls[i].pos.x = WORLD_W - WALL - BALL_R;
            self.balls[i].vel.x = -self.balls[i].vel.x;
            self.particles.sparks(pos.x, pos.y, [0.7, 0.9, 1.0, 1.0], 60, 140.0);
            self.push_sfx(Sfx::Wall);
        }
        if self.balls[i].pos.y < WALL + BALL_R {
            self.balls[i].pos.y = WALL + BALL_R;
            self.balls[i].vel.y = -self.balls[i].vel.y;
            self.particles.sparks(
                self.balls[i].pos.x,
                self.balls[i].pos.y,
                [0.7, 0.9, 1.0, 1.0],
                60,
                140.0,
            );
            self.push_sfx(Sfx::Wall);
        }

        if self.balls[i].pos.y > WORLD_H - WALL - BALL_R {
            if self.immortal {
                self.balls[i].pos.y = WORLD_H - WALL - BALL_R;
                self.balls[i].vel.y = -self.balls[i].vel.y.abs();
                self.particles.sparks(
                    self.balls[i].pos.x,
                    self.balls[i].pos.y,
                    [0.95, 0.82, 0.35, 1.0],
                    50,
                    120.0,
                );
            } else if self.balls[i].pos.y > WORLD_H + BALL_R * 2.0 {
                self.balls.swap_remove(i);
                return;
            }
        }

        let half = self.paddle_w() * 0.5;
        let paddle = Aabb::new(
            Vec2::new(self.paddle_x, PADDLE_Y),
            Vec2::new(half, PADDLE_H * 0.5),
        );
        if self.balls[i].vel.y > 0.0 {
            let p = self.balls[i].pos;
            if let Some(hit) = circle_aabb(p, BALL_R, paddle) {
                self.combo = 0;
                self.balls[i].pos = p.add(hit.normal.scale(hit.depth + 0.5));
                let offset = clamp((self.balls[i].pos.x - self.paddle_x) / half, -1.0, 1.0);
                let speed = self.balls[i].vel.length().max(self.ball_speed());
                let angle = (-90.0 + offset * 55.0).to_radians();
                self.balls[i].vel = Vec2::new(angle.cos(), angle.sin()).scale(speed);
                self.particles.sparks(
                    self.balls[i].pos.x,
                    self.balls[i].pos.y,
                    [0.85, 0.95, 1.0, 1.0],
                    100,
                    180.0,
                );
                self.impulse(0.12);
                self.push_sfx(Sfx::Paddle);
            }
        }

        let pierce = self.pierce_t > 0.0;
        let mut hit_i: Option<usize> = None;
        let p = self.balls[i].pos;
        for (bi, brick) in self.bricks.iter().enumerate() {
            if !brick.alive {
                continue;
            }
            let aabb = Aabb::new(
                Vec2::new(brick.x, brick.y),
                Vec2::new(BRICK_W * 0.5, BRICK_H * 0.5),
            );
            if circle_aabb(p, BALL_R, aabb).is_some() {
                hit_i = Some(bi);
                break;
            }
        }
        if let Some(bi) = hit_i {
            let brick = self.bricks[bi];
            let aabb = Aabb::new(
                Vec2::new(brick.x, brick.y),
                Vec2::new(BRICK_W * 0.5, BRICK_H * 0.5),
            );
            if let Some(hit) = circle_aabb(p, BALL_R, aabb) {
                if !pierce || brick.unbreakable {
                    self.balls[i].pos = p.add(hit.normal.scale(hit.depth + 0.5));
                    let dot = self.balls[i].vel.dot(hit.normal);
                    self.balls[i].vel = self.balls[i]
                        .vel
                        .sub(hit.normal.scale(2.0 * dot))
                        .normalized()
                        .scale(self.ball_speed());
                }
            }

            if brick.unbreakable {
                self.bricks[bi].flash = 1.0;
                self.particles.sparks(brick.x, brick.y, brick.color(), 80, 120.0);
                self.push_sfx(Sfx::Wall);
                return;
            }

            self.damage_brick(bi);

            if self.state != GameState::Playing {
                return;
            }
        }
    }

    fn damage_brick(&mut self, bi: usize) {
        if bi >= self.bricks.len() || !self.bricks[bi].alive {
            return;
        }
        if self.bricks[bi].unbreakable {
            self.bricks[bi].flash = 1.0;
            self.particles
                .sparks(self.bricks[bi].x, self.bricks[bi].y, self.bricks[bi].color(), 60, 100.0);
            self.push_sfx(Sfx::Wall);
            return;
        }
        let brick = self.bricks[bi];
        self.bricks[bi].hp = brick.hp.saturating_sub(1);
        self.bricks[bi].flash = 1.0;
        if self.bricks[bi].hp == 0 {
            self.bricks[bi].alive = false;
            self.combo = (self.combo + 1).min(8);
            self.score += 10 * self.combo;
            self.particles.shatter(brick.x, brick.y, brick.color(), true);
            self.impulse(0.28);
            self.hit_stop = 0.006;
            self.maybe_drop(brick.x, brick.y);
            self.push_sfx(Sfx::Shatter);
        } else {
            self.combo = (self.combo + 1).min(8);
            self.score += 3 * self.combo;
            self.particles.sparks(brick.x, brick.y, brick.color(), 80, 160.0);
            self.impulse(0.08);
            self.push_sfx(Sfx::Brick);
        }
        if self.bricks.iter().all(|b| !b.alive || b.unbreakable) {
            self.on_level_clear();
        }
    }

    fn on_level_clear(&mut self) {
        self.combo = 0;
        self.drops.clear();
        self.lasers.clear();
        if self.level >= MAX_LEVEL {
            self.state = GameState::Win;
            self.impulse(0.6);
            self.push_sfx(Sfx::Win);
        } else {
            self.load_level(self.level + 1);
            self.state = GameState::Ready;
            self.stick_balls();
            self.impulse(0.35);
        }
    }
}
