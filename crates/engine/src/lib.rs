//! Спільне ядро демо: математика, GPU, нахил, ассети, glTF/GLB.

pub mod assets;
pub mod audio;
pub mod gltf_mesh;
pub mod gpu;
pub mod math;
pub mod mesh;
pub mod tilt;

pub use assets::{load_bytes, load_bytes_from};
pub use gltf_mesh::{load_glb, CpuMesh, RgbaImage};
pub use gpu::Gpu;
pub use math::{clamp, Mat4, Vec2, Vec3};
pub use mesh::{box_mesh, plane_xz, sphere_mesh};
