//! HUD-цифри на GPU: рахунок, рівень, combo, життя, FPS.

use super::types::{SpriteInstance, HUD_CAP};
use crate::game::Game;

const DIGITS: [[u8; 7]; 10] = [
    [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
    [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
    [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
    [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110],
    [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
    [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
    [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
    [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
    [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
    [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
];

fn push_digit(
    out: &mut Vec<SpriteInstance>,
    cx: f32,
    cy: f32,
    d: u8,
    color: [f32; 4],
    s: f32,
    aspect: f32,
) {
    let hx = (s / aspect) * 0.38;
    let hy = s * 0.38;
    let g = DIGITS[d.min(9) as usize];
    for row in 0..7 {
        for col in 0..5 {
            if g[row] & (1 << (4 - col)) != 0 {
                let x = cx + (col as f32 - 2.0) * hx * 2.2;
                let y = cy - (row as f32 - 3.0) * hy * 2.2;
                out.push(SpriteInstance {
                    pos_size: [x, y, hx, hy],
                    color,
                });
            }
        }
    }
}

fn push_number(
    out: &mut Vec<SpriteInstance>,
    mut cx: f32,
    cy: f32,
    mut value: u32,
    digits: u32,
    color: [f32; 4],
    s: f32,
    aspect: f32,
) {
    let step = s * 5.4 / aspect;
    cx += step * (digits as f32 - 1.0);
    for _ in 0..digits {
        push_digit(out, cx, cy, (value % 10) as u8, color, s, aspect);
        value /= 10;
        cx -= step;
    }
}

pub fn collect_hud(game: &Game, aspect: f32) -> Vec<SpriteInstance> {
    let mut out = Vec::with_capacity(256);
    let panel = [0.02, 0.04, 0.07, 0.72];
    let ice = [0.82, 0.94, 1.0, 1.0];
    out.push(SpriteInstance {
        pos_size: [-0.72, 0.88, 0.26, 0.075],
        color: panel,
    });
    out.push(SpriteInstance {
        pos_size: [0.78, 0.88, 0.20, 0.075],
        color: panel,
    });
    push_number(&mut out, -0.88, 0.88, game.score, 6, ice, 0.028, aspect);
    push_digit(&mut out, 0.58, 0.88, game.level.min(9) as u8, ice, 0.028, aspect);
    if game.combo > 1 {
        let gold = [1.0, 0.84, 0.40, 1.0];
        out.push(SpriteInstance {
            pos_size: [0.0, 0.88, 0.07, 0.075],
            color: panel,
        });
        push_digit(&mut out, 0.0, 0.88, game.combo.min(9) as u8, gold, 0.036, aspect);
    }
    let heart = [1.0, 0.38, 0.52, 1.0];
    if game.immortal {
        out.push(SpriteInstance {
            pos_size: [0.82, 0.88, 0.14, 0.075],
            color: [0.22, 0.16, 0.04, 0.72],
        });
        out.push(SpriteInstance {
            pos_size: [0.82, 0.88, 0.028 / aspect, 0.018],
            color: [1.0, 0.82, 0.32, 1.0],
        });
    } else {
        for i in 0..game.lives.min(8) {
            let x = 0.92 - i as f32 * 0.042;
            out.push(SpriteInstance {
                pos_size: [x, 0.88, 0.014 / aspect, 0.014],
                color: heart,
            });
        }
    }
    out.push(SpriteInstance {
        pos_size: [-0.84, -0.90, 0.14, 0.055],
        color: panel,
    });
    push_number(
        &mut out,
        -0.94,
        -0.90,
        game.fps.round().clamp(0.0, 999.0) as u32,
        3,
        ice,
        0.022,
        aspect,
    );
    if out.len() > HUD_CAP {
        out.truncate(HUD_CAP);
    }
    out
}
