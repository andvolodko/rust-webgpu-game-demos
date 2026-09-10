//! Черга GLB, скинутого з диска / drag-and-drop, доки рендерер ще не готовий.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

static PENDING: Mutex<Option<Vec<u8>>> = Mutex::new(None);
static ADD_TANKS: AtomicU32 = AtomicU32::new(0);

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn queue(bytes: Vec<u8>) {
    if let Ok(mut slot) = PENDING.lock() {
        *slot = Some(bytes);
    }
}

#[allow(dead_code)]
pub fn take() -> Option<Vec<u8>> {
    PENDING.lock().ok().and_then(|mut slot| slot.take())
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn request_add(n: u32) {
    ADD_TANKS.fetch_add(n, Ordering::Relaxed);
}

pub fn take_add() -> u32 {
    ADD_TANKS.swap(0, Ordering::Relaxed)
}
