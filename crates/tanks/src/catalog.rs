//! Завантаження GLB танків і дерев. Якщо файлу немає — процедурний fallback.

use engine::gltf_mesh::CpuMesh;
use engine::math::{Mat4, Vec3};
use engine::{box_mesh, load_glb, load_bytes};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Team {
    Green,
    Steel,
}

pub struct Catalog {
    pub tank_green: Vec<CpuMesh>,
    pub tank_steel: Vec<CpuMesh>,
    pub trees: [Vec<CpuMesh>; 4],
    pub tank_radius: f32,
    pub tree_radius: [f32; 4],
    pub source: String,
}

impl Catalog {
    pub async fn load() -> Self {
        let mut notes = Vec::new();
        let tank_green = load_or(
            "tanks/models/tank_1_green.glb",
            5.4,
            procedural_tank([0.28, 0.48, 0.24, 1.0]),
            &mut notes,
        )
        .await;
        let tank_steel = load_or(
            "tanks/models/tank_1_bw.glb",
            5.4,
            procedural_tank([0.42, 0.44, 0.46, 1.0]),
            &mut notes,
        )
        .await;
        let trees = [
            load_or("tanks/models/tree.glb", 9.0, procedural_tree(0), &mut notes).await,
            load_or("tanks/models/tree2.glb", 11.0, procedural_tree(1), &mut notes).await,
            load_or("tanks/models/tree3.glb", 8.0, procedural_tree(2), &mut notes).await,
            load_or("tanks/models/tree4.glb", 12.5, procedural_tree(3), &mut notes).await,
        ];
        let tank_radius = xz_radius(&tank_green).max(xz_radius(&tank_steel)).max(1.8);
        let tree_radius = [
            xz_radius(&trees[0]).clamp(1.1, 2.6),
            xz_radius(&trees[1]).clamp(1.1, 2.6),
            xz_radius(&trees[2]).clamp(1.1, 2.6),
            xz_radius(&trees[3]).clamp(1.1, 2.6),
        ];
        let source = if notes.is_empty() {
            "glb".into()
        } else {
            format!("glb · {}", notes.join(", "))
        };
        Self {
            tank_green,
            tank_steel,
            trees,
            tank_radius,
            tree_radius,
            source,
        }
    }
}

async fn load_or(path: &str, size: f32, fallback: Vec<CpuMesh>, notes: &mut Vec<String>) -> Vec<CpuMesh> {
    match load_bytes(path).await {
        Ok(bytes) => match load_glb(&bytes) {
            Ok(mut meshes) if !meshes.is_empty() => {
                normalize_model(&mut meshes, size);
                meshes
            }
            Ok(_) | Err(_) => {
                notes.push(format!("{path}: fallback"));
                fallback
            }
        },
        Err(_) => {
            notes.push(format!("{path}: missing"));
            fallback
        }
    }
}

pub fn normalize_model(meshes: &mut [CpuMesh], target: f32) {
    let (min, max) = bounds(meshes);
    if !min[0].is_finite() {
        return;
    }
    let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let longest = size[0].max(size[1]).max(size[2]).max(1e-4);
    let s = target / longest;
    let xform = Mat4::from_scale(Vec3::new(s, s, s)).mul(Mat4::translation(Vec3::new(
        -(min[0] + max[0]) * 0.5,
        -min[1],
        -(min[2] + max[2]) * 0.5,
    )));
    for mesh in meshes.iter_mut() {
        *mesh = take_mesh(mesh).transformed(xform);
    }
}

fn take_mesh(mesh: &mut CpuMesh) -> CpuMesh {
    std::mem::replace(
        mesh,
        CpuMesh {
            name: String::new(),
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
            color: [1.0; 4],
            albedo: None,
        },
    )
}

fn bounds(meshes: &[CpuMesh]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for mesh in meshes {
        for p in &mesh.positions {
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
    }
    (min, max)
}

fn xz_radius(meshes: &[CpuMesh]) -> f32 {
    let (min, max) = bounds(meshes);
    if !min[0].is_finite() {
        return 2.0;
    }
    let dx = (max[0] - min[0]) * 0.5;
    let dz = (max[2] - min[2]) * 0.5;
    dx.max(dz) * 0.55
}

fn procedural_tank(hull: [f32; 4]) -> Vec<CpuMesh> {
    let parts = [
        (1.45, 0.38, 2.05, 0.0, 0.48, 0.0, hull),
        (0.22, 0.30, 2.15, -1.38, 0.30, 0.0, [0.12, 0.13, 0.12, 1.0]),
        (0.22, 0.30, 2.15, 1.38, 0.30, 0.0, [0.12, 0.13, 0.12, 1.0]),
        (0.72, 0.30, 0.72, 0.0, 1.05, -0.12, hull),
        (0.11, 0.11, 1.15, 0.0, 1.08, 1.15, [0.16, 0.16, 0.16, 1.0]),
    ];
    parts
        .into_iter()
        .map(|(hx, hy, hz, x, y, z, color)| {
            box_mesh(hx, hy, hz, color).transformed(Mat4::translation(Vec3::new(x, y, z)))
        })
        .collect()
}

fn procedural_tree(kind: u32) -> Vec<CpuMesh> {
    let (trunk_h, canopy_y, canopy, trunk_c, leaf) = match kind {
        0 => (2.6, 4.4, (1.1, 2.4, 1.1), [0.32, 0.22, 0.12, 1.0], [0.18, 0.42, 0.16, 1.0]),
        1 => (1.8, 3.2, (2.2, 1.8, 2.2), [0.28, 0.18, 0.10, 1.0], [0.22, 0.48, 0.18, 1.0]),
        2 => (3.1, 4.8, (1.4, 2.0, 1.4), [0.72, 0.70, 0.62, 1.0], [0.35, 0.52, 0.22, 1.0]),
        _ => (1.2, 2.0, (2.4, 1.3, 2.4), [0.24, 0.16, 0.10, 1.0], [0.16, 0.38, 0.14, 1.0]),
    };
    vec![
        box_mesh(0.22, trunk_h * 0.5, 0.22, trunk_c)
            .transformed(Mat4::translation(Vec3::new(0.0, trunk_h * 0.5, 0.0))),
        box_mesh(canopy.0, canopy.1, canopy.2, leaf)
            .transformed(Mat4::translation(Vec3::new(0.0, canopy_y, 0.0))),
    ]
}
