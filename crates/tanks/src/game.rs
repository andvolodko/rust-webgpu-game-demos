//! Бій 200 на 200: AI шукає ворога, стріляє ядрами, дерева як укриття.

use crate::catalog::{Catalog, Team};
use crate::particles::{ParticleInstance, Particles};
use engine::audio::Sfx;
use engine::math::{clamp, Mat4, Vec3};

pub const FIXED_DT: f32 = 1.0 / 120.0;
pub const MAP_HALF: f32 = 260.0;
const TANKS_PER_TEAM: usize = 200;
const TREE_COUNT: usize = 280;
const FIRE_RANGE: f32 = 42.0;
const SIGHT_RANGE: f32 = 120.0;
const TANK_SPEED: f32 = 9.2;
const TURN_RATE: f32 = 1.85;
const SHELL_SPEED: f32 = 58.0;
const MAX_HP: i32 = 3;

#[derive(Clone, Copy, Default)]
pub struct Input {
    pub pan_x: f32,
    pub pan_z: f32,
    pub orbit_dx: f32,
    pub orbit_dy: f32,
    pub zoom: f32,
    pub pointer_down: bool,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub transform: [f32; 16],
    pub color: [f32; 4],
}

pub struct DrawList {
    pub green: Vec<Instance>,
    pub steel: Vec<Instance>,
    pub trees: [Vec<Instance>; 4],
    pub shells: Vec<Instance>,
    pub particles: Vec<ParticleInstance>,
}

struct Tank {
    team: Team,
    x: f32,
    z: f32,
    yaw: f32,
    hp: i32,
    fire_cd: f32,
    dust_t: f32,
    speed: f32,
}

struct Tree {
    kind: usize,
    x: f32,
    z: f32,
    yaw: f32,
    scale: f32,
    radius: f32,
}

struct Shell {
    pos: Vec3,
    vel: Vec3,
    team: Team,
    life: f32,
    from: usize,
}

pub struct Game {
    pub cam_at: Vec3,
    pub cam_yaw: f32,
    pub cam_pitch: f32,
    pub cam_dist: f32,
    pub overlay: String,
    pub source: String,
    pub fps: f32,
    pub ready: bool,
    tanks: Vec<Tank>,
    trees: Vec<Tree>,
    shells: Vec<Shell>,
    particles: Particles,
    tank_radius: f32,
    rng: u32,
    time: f32,
    catalog_ok: bool,
    sfx: Vec<Sfx>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            cam_at: Vec3::new(0.0, 0.0, 0.0),
            cam_yaw: 0.35,
            cam_pitch: 0.72,
            cam_dist: 148.0,
            overlay: "Loading models…".into(),
            source: "…".into(),
            fps: 0.0,
            ready: false,
            tanks: Vec::new(),
            trees: Vec::new(),
            shells: Vec::new(),
            particles: Particles::new(),
            tank_radius: 2.0,
            rng: 0xC0FF_EE11,
            time: 0.0,
            catalog_ok: false,
            sfx: Vec::new(),
        }
    }

    pub fn start(&mut self, cat: &Catalog) {
        self.tank_radius = cat.tank_radius;
        self.source = cat.source.clone();
        self.catalog_ok = true;
        self.restart(cat);
    }

    pub fn restart(&mut self, cat: &Catalog) {
        self.tanks.clear();
        self.trees.clear();
        self.shells.clear();
        self.particles.clear();
        self.time = 0.0;
        self.ready = true;
        self.spawn_trees(cat);
        self.spawn_tanks(TANKS_PER_TEAM);
        self.overlay = "GREEN 200   —   STEEL 200".into();
    }

    /// Додає `count` живих танків у кожну команду (клавіша 1).
    pub fn add_tanks_per_team(&mut self, count: usize) {
        if !self.ready || count == 0 {
            return;
        }
        self.spawn_tanks(count);
        self.refresh_overlay();
        self.push_sfx(Sfx::Spawn);
    }

    fn push_sfx(&mut self, s: Sfx) {
        let same = self.sfx.iter().filter(|x| **x == s).count();
        if same < 2 && self.sfx.len() < 10 {
            self.sfx.push(s);
        }
    }

    pub fn drain_sfx(&mut self) -> Vec<Sfx> {
        std::mem::take(&mut self.sfx)
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    fn spawn_trees(&mut self, cat: &Catalog) {
        let mut placed = 0;
        let mut guard = 0;
        while placed < TREE_COUNT && guard < 12000 {
            guard += 1;
            let x = (self.rand() * 2.0 - 1.0) * (MAP_HALF * 0.48);
            let z = (self.rand() * 2.0 - 1.0) * (MAP_HALF - 16.0);
            if x.abs() < 10.0 && z.abs() < 28.0 {
                continue;
            }
            let kind = (self.rand() * 4.0).floor() as usize % 4;
            let radius = cat.tree_radius[kind];
            if self.trees.iter().any(|t| {
                let dx = t.x - x;
                let dz = t.z - z;
                dx * dx + dz * dz < (t.radius + radius + 3.5).powi(2)
            }) {
                continue;
            }
            let yaw = self.rand() * std::f32::consts::TAU;
            let scale = 0.82 + self.rand() * 0.45;
            self.trees.push(Tree {
                kind,
                x,
                z,
                yaw,
                scale,
                radius,
            });
            placed += 1;
        }
    }

    fn spawn_tanks(&mut self, per_team: usize) {
        for team in [Team::Green, Team::Steel] {
            let side = if team == Team::Green { -1.0 } else { 1.0 };
            let yaw0 = if team == Team::Green {
                std::f32::consts::FRAC_PI_2
            } else {
                -std::f32::consts::FRAC_PI_2
            };
            for _ in 0..per_team {
                let (x, z) = self.spawn_slot(side);
                let yaw = yaw0 + (self.rand() - 0.5) * 0.35;
                let fire_cd = self.rand() * 0.8;
                self.tanks.push(Tank {
                    team,
                    x,
                    z,
                    yaw,
                    hp: MAX_HP,
                    fire_cd,
                    dust_t: 0.0,
                    speed: 0.0,
                });
            }
        }
    }

    fn spawn_slot(&mut self, side: f32) -> (f32, f32) {
        let min_d = self.tank_radius * 2.2;
        let rear0 = MAP_HALF * 0.62;
        let rear1 = MAP_HALF * 0.90;
        for _ in 0..80 {
            let x = side * (rear0 + self.rand() * (rear1 - rear0));
            let z = (self.rand() * 2.0 - 1.0) * (MAP_HALF - 16.0);
            let ok = self.tanks.iter().all(|o| {
                if o.hp <= 0 {
                    return true;
                }
                let dx = o.x - x;
                let dz = o.z - z;
                dx * dx + dz * dz > min_d * min_d
            });
            if ok {
                return (x, z);
            }
        }
        (
            side * (rear0 + self.rand() * (rear1 - rear0)),
            (self.rand() * 2.0 - 1.0) * (MAP_HALF - 16.0),
        )
    }

    pub fn overlay_msg(&self) -> String {
        format!("{}   ·   {:.0} fps", self.overlay, self.fps)
    }

    pub fn camera_eye_target(&self) -> (Vec3, Vec3) {
        let target = Vec3::new(self.cam_at.x, 1.2, self.cam_at.z);
        let cp = self.cam_pitch.cos();
        let offset = Vec3::new(
            self.cam_dist * self.cam_yaw.sin() * cp,
            self.cam_dist * self.cam_pitch.sin(),
            self.cam_dist * self.cam_yaw.cos() * cp,
        );
        (target.add(offset), target)
    }

    pub fn update(&mut self, dt: f32, input: &Input) {
        self.time += dt;
        self.update_camera(dt, input);
        if !self.ready {
            return;
        }
        self.update_ai(dt);
        self.update_shells(dt);
        self.particles.update(dt);
        self.refresh_overlay();
    }

    fn update_camera(&mut self, dt: f32, input: &Input) {
        if input.pointer_down {
            self.cam_yaw += input.orbit_dx * 0.005;
            self.cam_pitch = clamp(self.cam_pitch + input.orbit_dy * 0.005, 0.18, 1.35);
        }
        self.cam_dist = clamp(self.cam_dist + input.zoom, 22.0, 360.0);
        let look = Vec3::new(-self.cam_yaw.sin(), 0.0, -self.cam_yaw.cos()).normalized();
        let right = Vec3::new(look.z, 0.0, -look.x);
        let pan = 72.0 * dt;
        self.cam_at = self.cam_at.add(look.scale(input.pan_z * pan)).add(right.scale(input.pan_x * pan));
        self.cam_at.x = clamp(self.cam_at.x, -MAP_HALF, MAP_HALF);
        self.cam_at.z = clamp(self.cam_at.z, -MAP_HALF, MAP_HALF);
    }

    fn update_ai(&mut self, dt: f32) {
        let n = self.tanks.len();
        for i in 0..n {
            if self.tanks[i].hp <= 0 {
                continue;
            }
            let team = self.tanks[i].team;
            let px = self.tanks[i].x;
            let pz = self.tanks[i].z;
            let mut best = None;
            let mut best_d = f32::MAX;
            for j in 0..n {
                if i == j || self.tanks[j].hp <= 0 || self.tanks[j].team == team {
                    continue;
                }
                let dx = self.tanks[j].x - px;
                let dz = self.tanks[j].z - pz;
                let d = (dx * dx + dz * dz).sqrt();
                if d < best_d {
                    best_d = d;
                    best = Some(j);
                }
            }
            let hunting = best.is_none() || best_d > SIGHT_RANGE;
            let (mut aim_x, mut aim_z, dist) = if let Some(j) = best {
                (self.tanks[j].x, self.tanks[j].z, best_d)
            } else {
                (0.0, 0.0, (px * px + pz * pz).sqrt())
            };
            if hunting {
                let lane = ((i % 21) as f32 - 10.0) * 7.0;
                aim_z += lane;
            }
            let blocked = self.los_blocked(px, pz, aim_x, aim_z);
            if blocked {
                let side = if (i % 2) == 0 { 1.0 } else { -1.0 };
                let dx = aim_x - px;
                let dz = aim_z - pz;
                aim_x = px - dz * side * 0.35 + dx * 0.4;
                aim_z = pz + dx * side * 0.35 + dz * 0.4;
            }

            let want = (aim_x - px).atan2(aim_z - pz);
            let mut yaw = self.tanks[i].yaw;
            let mut diff = wrap_angle(want - yaw);
            let avoid = self.steer_avoid(i, yaw);
            diff = wrap_angle(diff + avoid);
            let max_turn = TURN_RATE * dt;
            yaw += clamp(diff, -max_turn, max_turn);
            self.tanks[i].yaw = yaw;

            let aligned = diff.abs() < 0.55;
            let want_speed = if hunting || dist > FIRE_RANGE * 0.72 || blocked {
                TANK_SPEED
            } else if dist < 14.0 {
                -TANK_SPEED * 0.35
            } else {
                TANK_SPEED * 0.22
            };
            let speed = if aligned { want_speed } else { want_speed * 0.35 };
            self.tanks[i].speed = speed;
            let fwd = Vec3::new(yaw.sin(), 0.0, yaw.cos());
            let mut nx = px + fwd.x * speed * dt;
            let mut nz = pz + fwd.z * speed * dt;
            nx = clamp(nx, -MAP_HALF + 4.0, MAP_HALF - 4.0);
            nz = clamp(nz, -MAP_HALF + 4.0, MAP_HALF - 4.0);
            self.separate_tank(i, &mut nx, &mut nz);
            self.push_trees(&mut nx, &mut nz);
            self.tanks[i].x = nx;
            self.tanks[i].z = nz;

            if speed.abs() > 1.2 {
                self.tanks[i].dust_t += dt;
                if self.tanks[i].dust_t > 0.045 {
                    self.tanks[i].dust_t = 0.0;
                    let right = Vec3::new(fwd.z, 0.0, -fwd.x);
                    let vel = fwd.scale(speed);
                    let l = Vec3::new(nx, 0.0, nz).add(right.scale(-1.25)).add(fwd.scale(-1.1));
                    let r = Vec3::new(nx, 0.0, nz).add(right.scale(1.25)).add(fwd.scale(-1.1));
                    self.particles.dust(l, vel);
                    self.particles.dust(r, vel);
                }
            }

            self.tanks[i].fire_cd = (self.tanks[i].fire_cd - dt).max(0.0);
            if let Some(j) = best {
                if !hunting
                    && !blocked
                    && dist < FIRE_RANGE
                    && diff.abs() < 0.12
                    && self.tanks[i].fire_cd <= 0.0
                {
                    self.fire(i, j);
                }
            }
        }
    }

    fn fire(&mut self, i: usize, target: usize) {
        let yaw = self.tanks[i].yaw;
        let fwd = Vec3::new(yaw.sin(), 0.0, yaw.cos());
        let muzzle = Vec3::new(self.tanks[i].x, 1.15, self.tanks[i].z).add(fwd.scale(2.6));
        let tx = self.tanks[target].x - self.tanks[i].x;
        let tz = self.tanks[target].z - self.tanks[i].z;
        let dist = (tx * tx + tz * tz).sqrt().max(1.0);
        let lead = 0.08 * dist;
        let aim = Vec3::new(tx + (self.rand() - 0.5) * lead, 0.35, tz + (self.rand() - 0.5) * lead)
            .normalized();
        self.particles.muzzle(muzzle, aim);
        self.push_sfx(Sfx::Cannon);
        self.shells.push(Shell {
            pos: muzzle,
            vel: aim.scale(SHELL_SPEED),
            team: self.tanks[i].team,
            life: 2.4,
            from: i,
        });
        self.tanks[i].fire_cd = 1.45 + self.rand() * 0.9;
    }

    fn update_shells(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.shells.len() {
            {
                let s = &mut self.shells[i];
                s.life -= dt;
                s.vel.y -= 6.5 * dt;
                s.pos = s.pos.add(s.vel.scale(dt));
            }
            let pos = self.shells[i].pos;
            let team = self.shells[i].team;
            let from = self.shells[i].from;
            self.particles.trail(pos, self.shells[i].vel);

            let mut hit = pos.y < 0.2 || self.shells[i].life <= 0.0;
            if !hit {
                hit = self.tree_hit(pos.x, pos.z, 0.35);
            }
            let mut killed = None;
            if !hit {
                for (ti, tank) in self.tanks.iter_mut().enumerate() {
                    if tank.hp <= 0 || tank.team == team || ti == from {
                        continue;
                    }
                    let dx = tank.x - pos.x;
                    let dz = tank.z - pos.z;
                    if dx * dx + dz * dz < (self.tank_radius + 0.45).powi(2) && pos.y < 2.4 {
                        tank.hp -= 1;
                        hit = true;
                        if tank.hp <= 0 {
                            killed = Some((tank.x, tank.z, tank.team == Team::Green));
                        }
                        break;
                    }
                }
            }
            if hit {
                let green = team == Team::Green;
                self.particles.explode(Vec3::new(pos.x, pos.y.max(0.4), pos.z), green);
                self.push_sfx(Sfx::Explosion);
                if let Some((x, z, g)) = killed {
                    self.particles.explode(Vec3::new(x, 1.0, z), g);
                }
                self.shells.swap_remove(i);
                continue;
            }
            if pos.x.abs() > MAP_HALF + 8.0 || pos.z.abs() > MAP_HALF + 8.0 {
                self.shells.swap_remove(i);
                continue;
            }
            i += 1;
        }
    }

    fn los_blocked(&self, x0: f32, z0: f32, x1: f32, z1: f32) -> bool {
        let dx = x1 - x0;
        let dz = z1 - z0;
        let len = (dx * dx + dz * dz).sqrt().max(1e-4);
        for t in &self.trees {
            let px = t.x - x0;
            let pz = t.z - z0;
            let u = clamp((px * dx + pz * dz) / (len * len), 0.08, 0.92);
            let cx = x0 + dx * u - t.x;
            let cz = z0 + dz * u - t.z;
            if cx * cx + cz * cz < (t.radius * t.scale + 0.4).powi(2) {
                return true;
            }
        }
        false
    }

    fn tree_hit(&self, x: f32, z: f32, r: f32) -> bool {
        self.trees.iter().any(|t| {
            let dx = t.x - x;
            let dz = t.z - z;
            dx * dx + dz * dz < (t.radius * t.scale + r).powi(2)
        })
    }

    fn steer_avoid(&self, i: usize, yaw: f32) -> f32 {
        let t = &self.tanks[i];
        let feel = 7.5;
        let fx = t.x + yaw.sin() * feel;
        let fz = t.z + yaw.cos() * feel;
        let mut turn = 0.0;
        for tree in &self.trees {
            let dx = tree.x - t.x;
            let dz = tree.z - t.z;
            let d2 = dx * dx + dz * dz;
            let lim = (tree.radius * tree.scale + self.tank_radius + 3.0).powi(2);
            if d2 > lim {
                continue;
            }
            let ahead_x = tree.x - fx;
            let ahead_z = tree.z - fz;
            if ahead_x * ahead_x + ahead_z * ahead_z < (tree.radius * tree.scale + 2.5).powi(2) {
                let cross = yaw.cos() * dx - yaw.sin() * dz;
                turn += if cross > 0.0 { -0.9 } else { 0.9 };
            }
        }
        turn
    }

    fn separate_tank(&self, i: usize, nx: &mut f32, nz: &mut f32) {
        let min_d = self.tank_radius * 2.05;
        for (j, o) in self.tanks.iter().enumerate() {
            if i == j || o.hp <= 0 {
                continue;
            }
            let dx = *nx - o.x;
            let dz = *nz - o.z;
            let d = (dx * dx + dz * dz).sqrt();
            if d < min_d && d > 1e-3 {
                let push = (min_d - d) / d;
                *nx += dx * push * 0.55;
                *nz += dz * push * 0.55;
            }
        }
    }

    fn push_trees(&self, nx: &mut f32, nz: &mut f32) {
        for t in &self.trees {
            let dx = *nx - t.x;
            let dz = *nz - t.z;
            let min_d = t.radius * t.scale + self.tank_radius * 0.85;
            let d = (dx * dx + dz * dz).sqrt();
            if d < min_d && d > 1e-3 {
                let push = (min_d - d) / d;
                *nx += dx * push;
                *nz += dz * push;
            }
        }
    }

    fn refresh_overlay(&mut self) {
        let g = self.tanks.iter().filter(|t| t.team == Team::Green && t.hp > 0).count();
        let s = self.tanks.iter().filter(|t| t.team == Team::Steel && t.hp > 0).count();
        self.overlay = if g == 0 && s == 0 {
            "Draw  —  R restart".into()
        } else if g == 0 {
            "STEEL wins  —  R restart".into()
        } else if s == 0 {
            "GREEN wins  —  R restart".into()
        } else {
            format!("GREEN {g}   —   STEEL {s}")
        };
    }

    pub fn draw_list(&self) -> DrawList {
        let mut green = Vec::new();
        let mut steel = Vec::new();
        for t in &self.tanks {
            if t.hp <= 0 {
                continue;
            }
            let y = 0.0;
            let m = Mat4::translation(Vec3::new(t.x, y, t.z)).mul(Mat4::rotation_y(t.yaw));
            let inst = Instance {
                transform: m.m,
                color: [1.0, 1.0, 1.0, 1.0],
            };
            match t.team {
                Team::Green => green.push(inst),
                Team::Steel => steel.push(inst),
            }
        }
        let mut trees = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for t in &self.trees {
            let m = Mat4::translation(Vec3::new(t.x, 0.0, t.z))
                .mul(Mat4::rotation_y(t.yaw))
                .mul(Mat4::from_scale(Vec3::new(t.scale, t.scale, t.scale)));
            trees[t.kind].push(Instance {
                transform: m.m,
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
        let shells = self
            .shells
            .iter()
            .map(|s| {
                let m = Mat4::translation(s.pos).mul(Mat4::from_scale(Vec3::new(0.22, 0.22, 0.22)));
                Instance {
                    transform: m.m,
                    color: [0.12, 0.11, 0.10, 1.0],
                }
            })
            .collect();
        DrawList {
            green,
            steel,
            trees,
            shells,
            particles: self.particles.instances(),
        }
    }
}

fn wrap_angle(a: f32) -> f32 {
    let mut x = a;
    while x > std::f32::consts::PI {
        x -= std::f32::consts::TAU;
    }
    while x < -std::f32::consts::PI {
        x += std::f32::consts::TAU;
    }
    x
}
