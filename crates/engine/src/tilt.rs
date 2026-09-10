//! Нахил пристрою (web DeviceOrientation) для платформи і камери.

use std::cell::Cell;

thread_local! {
    static GAMMA: Cell<f32> = const { Cell::new(0.0) };
    static BETA: Cell<f32> = const { Cell::new(0.0) };
}

#[allow(dead_code)]
pub fn set(gamma: f32, beta: f32) {
    GAMMA.with(|c| c.set(if gamma.is_finite() { gamma } else { 0.0 }));
    BETA.with(|c| c.set(if beta.is_finite() { beta } else { 0.0 }));
}

pub fn get() -> (f32, f32) {
    (GAMMA.with(Cell::get), BETA.with(Cell::get))
}
