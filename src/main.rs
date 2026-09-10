fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    rust_webgpu_game::run();
}
