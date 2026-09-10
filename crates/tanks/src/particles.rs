//! CPU-партікли: пил з гусениць, трасер ядра, вибух.

use engine::math::Vec3;

#[derive(Clone, Copy)]
pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub color: [f32; 4],
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub gravity: f32,
    pub drag: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleInstance {
    pub pos_size: [f32; 4],
    pub color: [f32; 4],
}

pub struct Particles {
    items: Vec<Particle>,
    rng: u32,
}

impl Particles {
    pub fn new() -> Self {
        Self {
            items: Vec::with_capacity(4096),
            rng: 0xA11C_E5E5,
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    fn spawn(&mut self, p: Particle) {
        if self.items.len() >= 16384 {
            let i = (self.rng as usize) % self.items.len();
            self.items[i] = p;
        } else {
            self.items.push(p);
        }
    }

    fn burst(&mut self, pos: Vec3, n: u32, speed: f32, color: [f32; 4], size: f32, life: f32, gravity: f32) {
        for _ in 0..n {
            let a = self.rand() * std::f32::consts::TAU;
            let up = self.rand() * 0.9 + 0.15;
            let sp = speed * (0.35 + self.rand());
            let vel = Vec3::new(a.sin() * sp, up * sp, a.cos() * sp);
            let size = size * (0.6 + self.rand() * 0.8);
            self.spawn(Particle {
                pos,
                vel,
                color,
                life,
                max_life: life,
                size,
                gravity,
                drag: 1.8,
            });
        }
    }

    pub fn dust(&mut self, pos: Vec3, vel_xz: Vec3) {
        let a = self.rand() * std::f32::consts::TAU;
        let py = 0.12 + self.rand() * 0.1;
        let vx = vel_xz.x * 0.15 + (self.rand() - 0.5) * 0.8;
        let vy = 0.6 + self.rand() * 0.7;
        let vz = vel_xz.z * 0.15 + (self.rand() - 0.5) * 0.8;
        let life = 0.45 + self.rand() * 0.35;
        let size = 0.28 + self.rand() * 0.22;
        let p = Vec3::new(pos.x + a.sin() * 0.25, py, pos.z + a.cos() * 0.25);
        self.spawn(Particle {
            pos: p,
            vel: Vec3::new(vx, vy, vz),
            color: [0.42, 0.34, 0.22, 0.7],
            life,
            max_life: 0.7,
            size,
            gravity: 1.2,
            drag: 3.5,
        });
    }

    pub fn trail(&mut self, pos: Vec3, vel: Vec3) {
        self.spawn(Particle {
            pos,
            vel: vel.scale(0.08),
            color: [1.0, 0.78, 0.32, 1.0],
            life: 0.18,
            max_life: 0.18,
            size: 0.22,
            gravity: 0.0,
            drag: 4.0,
        });
        if self.rand() > 0.45 {
            let vx = (self.rand() - 0.5) * 1.2;
            let vy = (self.rand() - 0.3) * 1.0;
            let vz = (self.rand() - 0.5) * 1.2;
            self.spawn(Particle {
                pos,
                vel: Vec3::new(vx, vy, vz),
                color: [0.95, 0.55, 0.18, 0.9],
                life: 0.28,
                max_life: 0.28,
                size: 0.12,
                gravity: 2.0,
                drag: 2.0,
            });
        }
    }

    pub fn muzzle(&mut self, pos: Vec3, dir: Vec3) {
        self.burst(pos, 10, 6.0, [1.0, 0.72, 0.25, 1.0], 0.18, 0.16, 1.0);
        self.spawn(Particle {
            pos: pos.add(dir.scale(0.4)),
            vel: dir.scale(4.0),
            color: [1.0, 0.92, 0.55, 1.0],
            life: 0.08,
            max_life: 0.08,
            size: 0.55,
            gravity: 0.0,
            drag: 1.0,
        });
    }

    pub fn explode(&mut self, pos: Vec3, team_green: bool) {
        let fire = if team_green {
            [1.0, 0.55, 0.18, 1.0]
        } else {
            [1.0, 0.62, 0.28, 1.0]
        };
        self.burst(pos, 28, 14.0, fire, 0.32, 0.55, 9.0);
        self.burst(pos, 18, 7.0, [0.18, 0.16, 0.14, 0.85], 0.7, 1.1, -1.2);
        self.burst(pos, 12, 11.0, [0.55, 0.45, 0.35, 1.0], 0.2, 0.7, 12.0);
        self.spawn(Particle {
            pos,
            vel: Vec3::ZERO,
            color: [1.0, 0.85, 0.4, 1.0],
            life: 0.12,
            max_life: 0.12,
            size: 2.2,
            gravity: 0.0,
            drag: 0.5,
        });
    }

    pub fn update(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.items.len() {
            let p = &mut self.items[i];
            p.life -= dt;
            if p.life <= 0.0 {
                self.items.swap_remove(i);
                continue;
            }
            p.vel.y -= p.gravity * dt;
            p.vel = p.vel.scale((-p.drag * dt).exp());
            p.pos = p.pos.add(p.vel.scale(dt));
            if p.pos.y < 0.05 {
                p.pos.y = 0.05;
                p.vel.y *= -0.15;
                p.vel = p.vel.scale(0.6);
            }
            i += 1;
        }
    }

    pub fn instances(&self) -> Vec<ParticleInstance> {
        self.items
            .iter()
            .map(|p| {
                let t = (p.life / p.max_life.max(1e-4)).clamp(0.0, 1.0);
                let a = p.color[3] * t;
                ParticleInstance {
                    pos_size: [p.pos.x, p.pos.y, p.pos.z, p.size * (0.65 + 0.35 * t)],
                    color: [p.color[0], p.color[1], p.color[2], a],
                }
            })
            .collect()
    }
}
