//! Патерни рівнів. 0 = порожньо, 9 = нерозбивна, 1..=6 = HP.
//! Нижні ряди завжди прострілюються — без суцільного сталевого «дна».

use super::constants::{BRICK_COLS, BRICK_ROWS};

pub fn level_cell(level: u32, row: usize, col: usize) -> u8 {
    let nx = col as f32 / (BRICK_COLS - 1) as f32;
    let ny = row as f32 / (BRICK_ROWS - 1) as f32;
    match level {
        1 => {
            let band = ((1.0 - ny) * 3.0).ceil() as u8;
            band.clamp(1, 3)
        }
        2 => {
            let dist = (nx - 0.5).abs() + (ny - 0.32).abs() * 0.7;
            if dist > 0.52 {
                0
            } else if dist < 0.10 {
                3
            } else if dist < 0.26 {
                2
            } else {
                1
            }
        }
        3 => {
            let pillar = (col == BRICK_COLS / 4 || col == (BRICK_COLS * 3) / 4)
                && row > 3
                && row + 4 < BRICK_ROWS
                && row % 4 != 0;
            if pillar {
                9
            } else if row + 1 == BRICK_ROWS {
                0
            } else if (row + col) % 2 == 0 {
                1
            } else {
                2
            }
        }
        4 => {
            // Колони зі щілинами, низ відкритий — м'яч завжди дістає поле.
            let bottom = row + 3 >= BRICK_ROWS;
            let pillar = col == 8 || col == 23;
            let gap = row % 4 == 1;
            if bottom {
                if row + 1 == BRICK_ROWS {
                    0
                } else {
                    1
                }
            } else if pillar && !gap {
                9
            } else if row == 0 && (col < 2 || col + 2 >= BRICK_COLS) {
                9
            } else {
                1 + (row % 2) as u8
            }
        }
        _ => {
            let bottom = row + 3 >= BRICK_ROWS;
            let side_post = (col == 0 || col + 1 == BRICK_COLS) && row < 7;
            let bar = row == 7 && (col < 9 || col + 9 >= BRICK_COLS);
            if bottom {
                1
            } else if side_post || bar {
                9
            } else if row == 0 {
                3
            } else {
                1 + ((row + col) % 3) as u8
            }
        }
    }
}
