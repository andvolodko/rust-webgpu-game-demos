//! Процедурні меші (CPU), спільні для демо без GLB.

use crate::gltf_mesh::CpuMesh;

/// Вісь-вирівняний ящик: центр у початку координат, половинні розміри `hx, hy, hz`.
pub fn box_mesh(hx: f32, hy: f32, hz: f32, color: [f32; 4]) -> CpuMesh {
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        (
            [0.0, 0.0, 1.0],
            [
                [-hx, -hy, hz],
                [hx, -hy, hz],
                [hx, hy, hz],
                [-hx, hy, hz],
            ],
        ),
        (
            [0.0, 0.0, -1.0],
            [
                [hx, -hy, -hz],
                [-hx, -hy, -hz],
                [-hx, hy, -hz],
                [hx, hy, -hz],
            ],
        ),
        (
            [1.0, 0.0, 0.0],
            [
                [hx, -hy, hz],
                [hx, -hy, -hz],
                [hx, hy, -hz],
                [hx, hy, hz],
            ],
        ),
        (
            [-1.0, 0.0, 0.0],
            [
                [-hx, -hy, -hz],
                [-hx, -hy, hz],
                [-hx, hy, hz],
                [-hx, hy, -hz],
            ],
        ),
        (
            [0.0, 1.0, 0.0],
            [
                [-hx, hy, hz],
                [hx, hy, hz],
                [hx, hy, -hz],
                [-hx, hy, -hz],
            ],
        ),
        (
            [0.0, -1.0, 0.0],
            [
                [-hx, -hy, -hz],
                [hx, -hy, -hz],
                [hx, -hy, hz],
                [-hx, -hy, hz],
            ],
        ),
    ];

    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut uvs = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    let uv_corners = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

    for (n, q) in faces {
        let base = positions.len() as u32;
        for i in 0..4 {
            positions.push(q[i]);
            normals.push(n);
            uvs.push(uv_corners[i]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    CpuMesh {
        name: "box".into(),
        positions,
        normals,
        uvs,
        indices,
        color,
        albedo: None,
    }
}

/// Горизонтальна площина Y = `y`, половина розміру `half`, UV тайлиться `uv_repeat` разів.
pub fn plane_xz(half: f32, y: f32, uv_repeat: f32, color: [f32; 4]) -> CpuMesh {
    let h = half;
    let u = uv_repeat;
    CpuMesh {
        name: "plane".into(),
        positions: vec![[-h, y, -h], [h, y, -h], [h, y, h], [-h, y, h]],
        normals: vec![[0.0, 1.0, 0.0]; 4],
        uvs: vec![[0.0, 0.0], [u, 0.0], [u, u], [0.0, u]],
        indices: vec![0, 1, 2, 0, 2, 3],
        color,
        albedo: None,
    }
}

/// UV-сфера радіуса `r`.
pub fn sphere_mesh(r: f32, slices: u32, stacks: u32, color: [f32; 4]) -> CpuMesh {
    let slices = slices.max(3);
    let stacks = stacks.max(2);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for lat in 0..=stacks {
        let v = lat as f32 / stacks as f32;
        let phi = v * std::f32::consts::PI;
        let (sp, cp) = phi.sin_cos();
        for lon in 0..=slices {
            let u = lon as f32 / slices as f32;
            let theta = u * std::f32::consts::TAU;
            let (st, ct) = theta.sin_cos();
            let n = [st * sp, cp, ct * sp];
            positions.push([n[0] * r, n[1] * r, n[2] * r]);
            normals.push(n);
            uvs.push([u, v]);
        }
    }
    let row = slices + 1;
    for lat in 0..stacks {
        for lon in 0..slices {
            let a = lat * row + lon;
            let b = a + row;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    CpuMesh {
        name: "sphere".into(),
        positions,
        normals,
        uvs,
        indices,
        color,
        albedo: None,
    }
}
