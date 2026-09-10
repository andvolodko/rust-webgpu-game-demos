//! CPU лише емітить партікли в GPU storage-буфери.
//! Гравітація, drag, bounce і fade живуть у compute-шейдері.

use crate::math::{game_to_world, Vec3};

pub const MAX_SHARDS: usize = 65_536;
pub const MAX_SPARKLES: usize = 65_536;
pub const PARTICLE_STRIDE: u64 = 80;

const WORLD_H: f32 = 600.0;

/// Пакування 1:1 з WGSL `Particle` (80 байт).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuParticle {
    pub pos: [f32; 3],
    pub life: f32,
    pub vel: [f32; 3],
    pub max_life: f32,
    pub angles: [f32; 3],
    pub scale: f32,
    pub color: [f32; 4],
    pub ang_vel: [f32; 3],
    pub bounce: f32,
}

const _: () = assert!(std::mem::size_of::<GpuParticle>() == PARTICLE_STRIDE as usize);

pub struct ParticleUpload {
    pub index: u32,
    pub particle: GpuParticle,
}

pub struct ParticleSystem {
    pending_shards: Vec<ParticleUpload>,
    pending_sparks: Vec<ParticleUpload>,
    wants_clear: bool,
    rng: u32,
    win_acc: f32,
    shard_cursor: u32,
    spark_cursor: u32,
    shard_filled: u32,
    spark_filled: u32,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            pending_shards: Vec::with_capacity(512),
            pending_sparks: Vec::with_capacity(512),
            wants_clear: true,
            rng: 0xC001_D00D,
            win_acc: 0.0,
            shard_cursor: 0,
            spark_cursor: 0,
            shard_filled: 0,
            spark_filled: 0,
        }
    }

    pub fn clear(&mut self) {
        self.pending_shards.clear();
        self.pending_sparks.clear();
        self.wants_clear = true;
        self.win_acc = 0.0;
        self.shard_cursor = 0;
        self.spark_cursor = 0;
        self.shard_filled = 0;
        self.spark_filled = 0;
    }

    /// Забирає емісію за кадр: повний clear GPU-пулу + нові слоти + скільки слотів малювати.
    pub fn drain_uploads(&mut self) -> (bool, Vec<ParticleUpload>, Vec<ParticleUpload>, u32, u32) {
        (
            std::mem::replace(&mut self.wants_clear, false),
            std::mem::take(&mut self.pending_shards),
            std::mem::take(&mut self.pending_sparks),
            self.shard_filled,
            self.spark_filled,
        )
    }

    fn rand(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / 16_777_216.0
    }

    fn rand_range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.rand() * (hi - lo)
    }

    fn push_shard(&mut self, p: GpuParticle) {
        let i = self.shard_cursor % MAX_SHARDS as u32;
        self.shard_cursor = self.shard_cursor.wrapping_add(1);
        self.shard_filled = self.shard_filled.saturating_add(1).min(MAX_SHARDS as u32);
        self.pending_shards.push(ParticleUpload {
            index: i,
            particle: p,
        });
    }

    fn push_sparkle(&mut self, p: GpuParticle) {
        let i = self.spark_cursor % MAX_SPARKLES as u32;
        self.spark_cursor = self.spark_cursor.wrapping_add(1);
        self.spark_filled = self.spark_filled.saturating_add(1).min(MAX_SPARKLES as u32);
        self.pending_sparks.push(ParticleUpload {
            index: i,
            particle: p,
        });
    }

    fn world(x: f32, y: f32, z: f32) -> Vec3 {
        game_to_world(x, y, z, WORLD_H)
    }

    fn shard(
        pos: Vec3,
        vel: Vec3,
        angles: Vec3,
        ang_vel: Vec3,
        color: [f32; 4],
        scale: f32,
        life: f32,
        max_life: f32,
    ) -> GpuParticle {
        GpuParticle {
            pos: [pos.x, pos.y, pos.z],
            life,
            vel: [vel.x, vel.y, vel.z],
            max_life,
            angles: [angles.x, angles.y, angles.z],
            scale,
            color,
            ang_vel: [ang_vel.x, ang_vel.y, ang_vel.z],
            bounce: 1.0,
        }
    }

    fn spark(pos: Vec3, vel: Vec3, color: [f32; 4], size: f32, life: f32, max_life: f32) -> GpuParticle {
        GpuParticle {
            pos: [pos.x, pos.y, pos.z],
            life,
            vel: [vel.x, vel.y, vel.z],
            max_life,
            angles: [0.0, 0.0, 0.0],
            scale: size,
            color,
            ang_vel: [0.0, 0.0, 0.0],
            bounce: 0.0,
        }
    }

    /// Вибух кристала: уламки + іскри в точці цеглини.
    pub fn shatter(&mut self, gx: f32, gy: f32, color: [f32; 4], intense: bool) {
        let origin = Self::world(gx, gy, 14.0);
        let n_shards = if intense { 140 } else { 80 };
        let n_sparks = if intense { 220 } else { 100 };
        for _ in 0..n_shards {
            let dir = Vec3::new(
                self.rand_range(-1.0, 1.0),
                self.rand_range(-1.0, 1.0),
                self.rand_range(0.35, 1.0),
            )
            .normalized();
            let speed = self.rand_range(90.0, 240.0);
            let pos = origin.add(Vec3::new(
                self.rand_range(-6.0, 6.0),
                self.rand_range(-4.0, 4.0),
                self.rand_range(0.0, 5.0),
            ));
            let angles = Vec3::new(
                self.rand_range(0.0, 6.28),
                self.rand_range(0.0, 6.28),
                self.rand_range(0.0, 6.28),
            );
            let ang_vel = Vec3::new(
                self.rand_range(-8.0, 8.0),
                self.rand_range(-8.0, 8.0),
                self.rand_range(-8.0, 8.0),
            );
            let scale = self.rand_range(2.2, 5.0);
            let life = self.rand_range(0.55, 0.95);
            self.push_shard(Self::shard(
                pos,
                dir.scale(speed),
                angles,
                ang_vel,
                color,
                scale,
                life,
                0.9,
            ));
        }
        for _ in 0..n_sparks {
            let dir = Vec3::new(
                self.rand_range(-1.0, 1.0),
                self.rand_range(-1.0, 1.0),
                self.rand_range(0.1, 1.0),
            )
            .normalized();
            let vel = dir.scale(self.rand_range(60.0, 280.0));
            let size = self.rand_range(3.0, 7.0);
            let life = self.rand_range(0.25, 0.55);
            self.push_sparkle(Self::spark(
                origin,
                vel,
                [
                    (color[0] + 0.35).min(1.0),
                    (color[1] + 0.35).min(1.0),
                    (color[2] + 0.45).min(1.0),
                    1.0,
                ],
                size,
                life,
                0.5,
            ));
        }
    }

    /// Короткий спалах іскор (удар по HP / paddle / стіна).
    pub fn sparks(&mut self, gx: f32, gy: f32, color: [f32; 4], count: u32, speed: f32) {
        let origin = Self::world(gx, gy, 12.0);
        for _ in 0..count {
            let dir = Vec3::new(
                self.rand_range(-1.0, 1.0),
                self.rand_range(-1.0, 1.0),
                self.rand_range(0.2, 1.0),
            )
            .normalized();
            let vel = dir.scale(self.rand_range(speed * 0.4, speed));
            let size = self.rand_range(2.2, 5.5);
            let life = self.rand_range(0.12, 0.32);
            self.push_sparkle(Self::spark(origin, vel, color, size, life, 0.3));
        }
    }

    /// Слід м'яча.
    pub fn trail(&mut self, gx: f32, gy: f32, color: [f32; 4]) {
        let origin = Self::world(gx, gy, 10.0);
        for _ in 0..8 {
            let pos = origin.add(Vec3::new(
                self.rand_range(-2.0, 2.0),
                self.rand_range(-2.0, 2.0),
                self.rand_range(-1.0, 2.0),
            ));
            let vel = Vec3::new(
                self.rand_range(-12.0, 12.0),
                self.rand_range(-12.0, 12.0),
                self.rand_range(8.0, 28.0),
            );
            let size = self.rand_range(2.8, 5.5);
            self.push_sparkle(Self::spark(pos, vel, color, size, 0.22, 0.22));
        }
    }

    pub fn pickup_burst(&mut self, gx: f32, gy: f32, color: [f32; 4]) {
        self.sparks(gx, gy, color, 180, 220.0);
        let origin = Self::world(gx, gy, 16.0);
        for _ in 0..60 {
            let dir = Vec3::new(
                self.rand_range(-1.0, 1.0),
                self.rand_range(-1.0, 1.0),
                self.rand_range(0.5, 1.0),
            )
            .normalized();
            let vel = dir.scale(self.rand_range(70.0, 140.0));
            let angles = Vec3::new(self.rand(), self.rand(), self.rand()).scale(6.28);
            let ang_vel = Vec3::new(
                self.rand_range(-6.0, 6.0),
                self.rand_range(-6.0, 6.0),
                self.rand_range(-6.0, 6.0),
            );
            let scale = self.rand_range(3.0, 5.5);
            self.push_shard(Self::shard(origin, vel, angles, ang_vel, color, scale, 0.5, 0.5));
        }
    }

    pub fn fireworks(&mut self, dt: f32) {
        self.win_acc += dt;
        if self.win_acc < 0.18 {
            return;
        }
        self.win_acc = 0.0;
        let gx = self.rand_range(80.0, 720.0);
        let gy = self.rand_range(80.0, 380.0);
        let color = match (self.rand() * 5.0) as u32 {
            0 => [0.95, 0.45, 0.85, 1.0],
            1 => [0.40, 0.85, 1.00, 1.0],
            2 => [0.95, 0.80, 0.35, 1.0],
            3 => [0.45, 0.95, 0.70, 1.0],
            _ => [0.85, 0.55, 1.00, 1.0],
        };
        self.shatter(gx, gy, color, true);
    }
}
