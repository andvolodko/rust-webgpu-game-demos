//! Збір opaque / glass інстансів і м'ячів зі стану гри.

use super::types::{CrystalInstance, SpriteInstance, BALL_CAP};
use crate::game::{Game, GameState, BALL_R, BRICK_H, BRICK_W, PADDLE_H, PADDLE_Y, WALL, WORLD_H, WORLD_W};
use crate::math::{game_to_world, Vec3};

fn gw(x: f32, y: f32, z: f32) -> Vec3 {
    game_to_world(x, y, z, WORLD_H)
}

pub fn collect_opaque(game: &Game) -> Vec<CrystalInstance> {
    let mut opaque = Vec::with_capacity(32);
    let z0 = Vec3::ZERO;
    opaque.push(CrystalInstance::new(
        gw(WORLD_W * 0.5, WORLD_H * 0.5, -6.0),
        0.0,
        Vec3::new(WORLD_W * 0.5 + 24.0, WORLD_H * 0.5 + 24.0, 4.0),
        0.0,
        z0,
        [0.07, 0.09, 0.14, 1.0],
    ));
    let wall_c = [0.18, 0.22, 0.30, 1.0];
    opaque.push(CrystalInstance::new(
        gw(WALL * 0.5, WORLD_H * 0.5, 16.0),
        0.05,
        Vec3::new(WALL * 0.5, WORLD_H * 0.5, 16.0),
        0.0,
        z0,
        wall_c,
    ));
    opaque.push(CrystalInstance::new(
        gw(WORLD_W - WALL * 0.5, WORLD_H * 0.5, 16.0),
        0.05,
        Vec3::new(WALL * 0.5, WORLD_H * 0.5, 16.0),
        0.0,
        z0,
        wall_c,
    ));
    opaque.push(CrystalInstance::new(
        gw(WORLD_W * 0.5, WALL * 0.5, 16.0),
        0.05,
        Vec3::new(WORLD_W * 0.5, WALL * 0.5, 16.0),
        0.0,
        z0,
        wall_c,
    ));
    if game.immortal {
        opaque.push(CrystalInstance::new(
            gw(WORLD_W * 0.5, WORLD_H - WALL * 0.5, 16.0),
            0.55,
            Vec3::new(WORLD_W * 0.5, WALL * 0.5, 16.0),
            0.0,
            z0,
            [0.85, 0.62, 0.22, 1.0],
        ));
    }

    let pw = game.paddle_w();
    let paddle_c = if game.expand_t > 0.0 {
        [0.45, 0.95, 1.0, 1.0]
    } else if game.state == GameState::Ready {
        [0.55, 0.90, 1.0, 1.0]
    } else {
        [0.82, 0.90, 0.98, 1.0]
    };
    opaque.push(CrystalInstance::new(
        gw(game.paddle_x, PADDLE_Y, 11.0),
        if game.expand_t > 0.0 { 0.45 } else { 0.18 },
        Vec3::new(pw * 0.5, PADDLE_H * 0.5, 9.0),
        0.0,
        z0,
        paddle_c,
    ));

    for b in &game.bricks {
        if !b.alive || !b.unbreakable {
            continue;
        }
        let hz = b.height() * 0.5;
        opaque.push(CrystalInstance::new(
            gw(b.x, b.y, hz + 1.6),
            0.08 + b.flash * 0.5,
            Vec3::new(BRICK_W * 0.46, BRICK_H * 0.46, hz),
            0.0,
            z0,
            b.color(),
        ));
    }

    for d in &game.drops {
        let rot = Vec3::new(game.time * 4.0, game.time * 2.6, 0.4);
        opaque.push(CrystalInstance::new(
            gw(d.pos.x, d.pos.y, 12.0),
            0.8,
            Vec3::new(8.0, 8.0, 11.0),
            0.0,
            rot,
            d.kind.color(),
        ));
    }

    opaque
}

pub fn collect_glass(game: &Game) -> Vec<CrystalInstance> {
    let mut bricks: Vec<_> = game
        .bricks
        .iter()
        .filter(|b| b.alive && !b.unbreakable)
        .collect();
    bricks.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = Vec::with_capacity(bricks.len());
    let z0 = Vec3::ZERO;
    for b in bricks {
        let hz = b.height() * 0.5;
        out.push(CrystalInstance::new(
            gw(b.x, b.y, hz + 1.6),
            0.22 + b.flash * 0.85,
            Vec3::new(BRICK_W * 0.46, BRICK_H * 0.46, hz),
            0.62,
            z0,
            b.color(),
        ));
    }
    out
}

pub fn collect_balls(game: &Game) -> Vec<SpriteInstance> {
    let pierce = game.pierce_t > 0.0;
    game.balls
        .iter()
        .take(BALL_CAP)
        .map(|b| {
            let p = gw(b.pos.x, b.pos.y, 10.0);
            let c = if pierce {
                [0.95, 0.62, 1.0, 1.0]
            } else {
                [0.85, 0.97, 1.0, 1.0]
            };
            SpriteInstance {
                pos_size: [p.x, p.y, p.z, BALL_R],
                color: c,
            }
        })
        .collect()
}
