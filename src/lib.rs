//! Crystal Arkanoid: native `run()` + wasm `start()`.

mod app;
mod game;
mod gfx;
mod math;
mod particles;
mod tilt;

#[cfg(not(target_arch = "wasm32"))]
pub use app::run;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn start() {
    app::run_web();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn set_tilt(gamma: f32, beta: f32) {
    tilt::set(gamma, beta);
}
