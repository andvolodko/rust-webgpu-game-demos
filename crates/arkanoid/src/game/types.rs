//! Типи ігрової логіки: стан, power-upи, цеглини, м'ячі, ввід.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Ready,
    Playing,
    GameOver,
    Win,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Powerup {
    Expand,
    Multiball,
    Slow,
    Pierce,
    ExtraLife,
    Laser,
}

impl Powerup {
    pub fn color(self) -> [f32; 4] {
        match self {
            Powerup::Expand => [0.35, 0.95, 0.90, 1.0],
            Powerup::Multiball => [0.98, 0.82, 0.28, 1.0],
            Powerup::Slow => [0.40, 0.65, 1.00, 1.0],
            Powerup::Pierce => [0.90, 0.40, 1.00, 1.0],
            Powerup::ExtraLife => [1.00, 0.35, 0.50, 1.0],
            Powerup::Laser => [1.00, 0.72, 0.22, 1.0],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Brick {
    pub x: f32,
    pub y: f32,
    pub hp: u8,
    pub max_hp: u8,
    pub alive: bool,
    pub unbreakable: bool,
    pub flash: f32,
}

impl Brick {
    pub fn color(&self) -> [f32; 4] {
        if self.unbreakable {
            return [0.42, 0.46, 0.55, 1.0];
        }
        match self.hp {
            6..=7 => [0.95, 0.38, 0.62, 1.0],
            4..=5 => [0.98, 0.55, 0.28, 1.0],
            2..=3 => [0.35, 0.88, 0.72, 1.0],
            _ => [0.40, 0.72, 1.00, 1.0],
        }
    }

    pub fn height(&self) -> f32 {
        if self.unbreakable {
            10.0
        } else {
            4.0 + self.max_hp as f32 * 0.7 + self.hp as f32 * 1.1
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ball {
    pub pos: engine::math::Vec2,
    pub vel: engine::math::Vec2,
}

#[derive(Debug, Clone, Copy)]
pub struct Drop {
    pub pos: engine::math::Vec2,
    pub kind: Powerup,
}

#[derive(Debug, Clone, Copy)]
pub struct Laser {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Input {
    pub axis: f32,
    pub pointer_x: Option<f32>,
    pub launch: bool,
    /// Нахил телефону −1..1 (ліворуч / вперед).
    pub tilt_x: f32,
    pub tilt_y: f32,
}
