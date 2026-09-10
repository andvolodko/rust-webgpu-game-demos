//! Ігрові константи світу, платформи, м'яча і сітки цеглин.

pub const FIXED_DT: f32 = 1.0 / 120.0;

pub const WORLD_W: f32 = 800.0;
pub const WORLD_H: f32 = 600.0;

pub const PADDLE_W: f32 = 128.0;
pub const PADDLE_W_WIDE: f32 = 200.0;
pub const PADDLE_H: f32 = 16.0;
pub const PADDLE_Y: f32 = 560.0;
pub const PADDLE_SPEED: f32 = 720.0;

pub const BALL_R: f32 = 8.0;
pub const BALL_SPEED_START: f32 = 270.0;
pub const BALL_SPEED_INC: f32 = 8.0;
pub const BALL_SPEED_MAX: f32 = 430.0;
pub const MAX_BALLS: usize = 3;

pub const WALL: f32 = 12.0;

pub const BRICK_COLS: usize = 32;
pub const BRICK_ROWS: usize = 22;
pub const BRICK_TOP: f32 = 48.0;
pub const BRICK_W: f32 = 21.0;
pub const BRICK_H: f32 = 13.0;
pub const BRICK_GAP: f32 = 3.0;
pub const BRICK_HPAD: f32 =
    (WORLD_W - BRICK_COLS as f32 * BRICK_W - (BRICK_COLS as f32 - 1.0) * BRICK_GAP) * 0.5;
pub const BRICK_VGAP: f32 = 2.5;

pub const MAX_LIVES: u32 = 6;
pub const MAX_STORED_LIVES: u32 = 9;
pub const MAX_LEVEL: u32 = 5;
pub const DROP_CHANCE: f32 = 0.28;
pub const DROP_SPEED: f32 = 140.0;

pub const EXPAND_TIME: f32 = 12.0;
pub const SLOW_TIME: f32 = 9.0;
pub const PIERCE_TIME: f32 = 8.0;
