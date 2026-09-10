//! Tanks demo: native `run()` + wasm `start()`.

mod app;
mod catalog;
mod game;
mod particles;
mod pending;
mod renderer;

#[cfg(not(target_arch = "wasm32"))]
pub use app::run;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn start() {
    app::run_web();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn load_glb_bytes(data: &[u8]) {
    pending::queue(data.to_vec());
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn add_tanks() {
    pending::request_add(10);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn unlock_audio() {
    engine::audio::resume();
}
