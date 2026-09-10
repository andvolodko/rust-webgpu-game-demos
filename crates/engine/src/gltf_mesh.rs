//! Завантаження self-contained GLB (і glTF з вбудованим BIN).
//! FBX у рантаймі не читаємо — конвертуй у Blender у `.glb`.

use crate::math::{Mat4, Vec3};
use image::GenericImageView;

#[derive(Clone, Debug)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct CpuMesh {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub color: [f32; 4],
    pub albedo: Option<RgbaImage>,
}

impl CpuMesh {
    pub fn transformed(mut self, m: Mat4) -> Self {
        for p in &mut self.positions {
            let t = m.transform_point(Vec3::new(p[0], p[1], p[2]));
            *p = [t.x, t.y, t.z];
        }
        for n in &mut self.normals {
            let t = m.transform_vector(Vec3::new(n[0], n[1], n[2])).normalized();
            *n = [t.x, t.y, t.z];
        }
        self
    }
}

/// Парсить `.glb` (або `.gltf` з BIN chunk). Зовнішні URI не підтягуються.
pub fn load_glb(bytes: &[u8]) -> Result<Vec<CpuMesh>, String> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| format!("gltf parse: {e}"))?;
    let blob: Vec<u8> = gltf.blob.clone().unwrap_or_default();
    let mut meshes = Vec::new();

    let scenes: Vec<gltf::Scene<'_>> = if let Some(scene) = gltf.default_scene() {
        vec![scene]
    } else {
        gltf.scenes().collect()
    };

    if scenes.is_empty() {
        for mesh in gltf.meshes() {
            push_mesh(&mut meshes, &mesh, Mat4::IDENTITY, &blob)?;
        }
        return finish(meshes);
    }

    for scene in scenes {
        for node in scene.nodes() {
            visit_node(node, Mat4::IDENTITY, &mut meshes, &blob)?;
        }
    }
    finish(meshes)
}

fn finish(meshes: Vec<CpuMesh>) -> Result<Vec<CpuMesh>, String> {
    if meshes.is_empty() {
        Err("glb has no triangle meshes".into())
    } else {
        Ok(meshes)
    }
}

fn visit_node(
    node: gltf::Node<'_>,
    parent: Mat4,
    meshes: &mut Vec<CpuMesh>,
    blob: &[u8],
) -> Result<(), String> {
    let local = Mat4::from_cols_array_2d(node.transform().matrix());
    let world = parent.mul(local);
    if let Some(mesh) = node.mesh() {
        push_mesh(meshes, &mesh, world, blob)?;
    }
    for child in node.children() {
        visit_node(child, world, meshes, blob)?;
    }
    Ok(())
}

fn push_mesh(
    out: &mut Vec<CpuMesh>,
    mesh: &gltf::Mesh<'_>,
    world: Mat4,
    blob: &[u8],
) -> Result<(), String> {
    let mesh_name = mesh.name().unwrap_or("mesh").to_string();
    for (pi, primitive) in mesh.primitives().enumerate() {
        if primitive.mode() != gltf::mesh::Mode::Triangles {
            continue;
        }

        let (positions, normals, uvs, indices) = {
            let reader = primitive.reader(|buffer| match buffer.source() {
                gltf::buffer::Source::Bin => Some(blob),
                gltf::buffer::Source::Uri(_) => None,
            });
            let Some(pos_iter) = reader.read_positions() else {
                continue;
            };
            let positions: Vec<[f32; 3]> = pos_iter.collect();
            if positions.is_empty() {
                continue;
            }
            let nvert = positions.len();
            let normals = reader
                .read_normals()
                .map(|n| n.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; nvert]);
            let uvs = reader
                .read_tex_coords(0)
                .map(|t| t.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; nvert]);
            let indices = reader
                .read_indices()
                .map(|i| i.into_u32().collect())
                .unwrap_or_else(|| (0..nvert as u32).collect());
            (positions, normals, uvs, indices)
        };

        let pbr = primitive.material().pbr_metallic_roughness();
        let color = pbr.base_color_factor();
        let albedo = pbr
            .base_color_texture()
            .and_then(|info| decode_texture(info.texture().source(), blob));

        let mut cpu = CpuMesh {
            name: format!("{mesh_name}#{pi}"),
            positions,
            normals,
            uvs,
            indices,
            color,
            albedo,
        };
        if world.m != Mat4::IDENTITY.m {
            cpu = cpu.transformed(world);
        }
        out.push(cpu);
    }
    Ok(())
}

fn decode_texture(image: gltf::Image<'_>, blob: &[u8]) -> Option<RgbaImage> {
    let bytes: Vec<u8> = match image.source() {
        gltf::image::Source::View { view, mime_type: _ } => {
            if !matches!(view.buffer().source(), gltf::buffer::Source::Bin) {
                return None;
            }
            let start = view.offset();
            let end = start.checked_add(view.length())?;
            blob.get(start..end)?.to_vec()
        }
        gltf::image::Source::Uri { uri, mime_type: _ } => decode_data_uri(uri)?,
    };
    let img = image::load_from_memory(&bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    Some(RgbaImage {
        width,
        height,
        pixels: rgba.into_raw(),
    })
}

fn decode_data_uri(uri: &str) -> Option<Vec<u8>> {
    const PREFIX: &str = "base64,";
    let i = uri.find(PREFIX)?;
    base64_decode(uri[i + PREFIX.len()..].trim())
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        })
    }
    let bytes: Vec<u8> = input
        .bytes()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    if bytes.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let pad = chunk.iter().filter(|&&c| c == b'=').count();
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let c = if chunk[2] == b'=' { 0 } else { val(chunk[2])? };
        let d = if chunk[3] == b'=' { 0 } else { val(chunk[3])? };
        let n = (u32::from(a) << 18) | (u32::from(b) << 12) | (u32::from(c) << 6) | u32::from(d);
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}
