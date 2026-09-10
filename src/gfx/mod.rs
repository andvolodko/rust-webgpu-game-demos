//! Графічний шар: wgpu-рендерер, меші, сцена, HUD, GPU-партікли.

mod camera;
mod gpu_particles;
mod hud;
mod mesh;
mod renderer;
mod scene;
mod targets;
mod types;

pub use renderer::Renderer;
