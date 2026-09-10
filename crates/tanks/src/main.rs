fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    tanks::run();
}
