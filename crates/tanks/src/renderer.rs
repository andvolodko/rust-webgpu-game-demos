//! Інстанси танків/дерев + більборди партіклів.

use crate::catalog::Catalog;
use crate::game::{Game, Instance, MAP_HALF};
use engine::gltf_mesh::{CpuMesh, RgbaImage};
use engine::math::{Mat4, Vec3};
use engine::{plane_xz, sphere_mesh, Gpu};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    n: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_proj: [f32; 16],
    light_view_proj: [f32; 16],
    cam_pos: [f32; 4],
    cam_right: [f32; 4],
    cam_up: [f32; 4],
    light_dir: [f32; 4],
    fog_color: [f32; 4],
}

struct MeshGpu {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    index_count: u32,
    _albedo: wgpu::Texture,
    albedo_bg: wgpu::BindGroup,
}

const VERT_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 32,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: 12,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: 24,
            shader_location: 2,
            format: wgpu::VertexFormat::Float32x2,
        },
    ],
};

const INST_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 80,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 3,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: 16,
            shader_location: 4,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: 32,
            shader_location: 5,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: 48,
            shader_location: 6,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: 64,
            shader_location: 7,
            format: wgpu::VertexFormat::Float32x4,
        },
    ],
};

const INST_CAP: u64 = 8192;
const PART_CAP: u64 = 16384;
const SHADOW_SIZE: u32 = 2048;

pub struct Renderer {
    gpu: Gpu,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    _shadow: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    mesh_pl: wgpu::RenderPipeline,
    shadow_pl: wgpu::RenderPipeline,
    part_pl: wgpu::RenderPipeline,
    globals_buf: wgpu::Buffer,
    uniform_bg: wgpu::BindGroup,
    lit_bg: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    albedo_layout: wgpu::BindGroupLayout,
    instance_buf: wgpu::Buffer,
    part_quad: wgpu::Buffer,
    part_inst: wgpu::Buffer,
    ground: MeshGpu,
    shell: MeshGpu,
    tank_green: Vec<MeshGpu>,
    tank_steel: Vec<MeshGpu>,
    trees: [Vec<MeshGpu>; 4],
}

impl Renderer {
    pub async fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let gpu = Gpu::new(instance, surface, width, height, "tanks-device").await?;
        let format = gpu.format();
        let (depth, depth_view) = make_depth(&gpu.device, gpu.config.width, gpu.config.height);
        let (shadow, shadow_view) = make_shadow(&gpu.device);

        let globals_buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Globals>() as u64),
                },
                count: None,
            }],
        });
        let lit_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lit-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Globals>() as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let albedo_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("albedo-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let uniform_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform-bg"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buf.as_entire_binding(),
            }],
        });
        let shadow_samp = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let lit_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lit-bg"),
            layout: &lit_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_samp),
                },
            ],
        });
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("albedo"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            ..Default::default()
        });

        let mesh_mod = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mesh"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/mesh.wgsl").into()),
        });
        let part_mod = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/particle.wgsl").into()),
        });
        let mesh_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh-pl"),
            bind_group_layouts: &[Some(&lit_layout), Some(&albedo_layout)],
            immediate_size: 0,
        });
        let part_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("part-pl"),
            bind_group_layouts: &[Some(&uniform_layout)],
            immediate_size: 0,
        });
        let shadow_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow-pl"),
            bind_group_layouts: &[Some(&uniform_layout)],
            immediate_size: 0,
        });
        let color_opaque = [Some(wgpu::ColorTargetState {
            format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let color_add = [Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let mesh_pl = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &mesh_mod,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(VERT_LAYOUT), Some(INST_LAYOUT)],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(depth_state(true)),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &mesh_mod,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &color_opaque,
            }),
            multiview_mask: None,
            cache: None,
        });
        let shadow_pl = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&shadow_layout),
            vertex: wgpu::VertexState {
                module: &mesh_mod,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &[Some(VERT_LAYOUT), Some(INST_LAYOUT)],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(shadow_depth_state()),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        let part_pl = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particles"),
            layout: Some(&part_layout),
            vertex: wgpu::VertexState {
                module: &part_mod,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 8,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        }],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 32,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    }),
                ],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(depth_state(false)),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &part_mod,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &color_add,
            }),
            multiview_mask: None,
            cache: None,
        });

        let instance_buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: INST_CAP * 80,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let part_inst = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("part-inst"),
            size: PART_CAP * 32,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let quad: [[f32; 2]; 6] = [
            [-1.0, -1.0],
            [1.0, -1.0],
            [1.0, 1.0],
            [-1.0, -1.0],
            [1.0, 1.0],
            [-1.0, 1.0],
        ];
        let part_quad = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("part-quad"),
            contents: bytemuck::cast_slice(&quad),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut ground_mesh = plane_xz(MAP_HALF + 18.0, 0.0, 62.0, [1.0, 1.0, 1.0, 1.0]);
        ground_mesh.albedo = Some(grass_image());
        let ground = upload_mesh(&gpu.device, &gpu.queue, &albedo_layout, &sampler, &ground_mesh);
        let shell = upload_mesh(
            &gpu.device,
            &gpu.queue,
            &albedo_layout,
            &sampler,
            &sphere_mesh(1.0, 12, 8, [0.14, 0.13, 0.12, 1.0]),
        );

        Ok(Self {
            gpu,
            depth,
            depth_view,
            _shadow: shadow,
            shadow_view,
            mesh_pl,
            shadow_pl,
            part_pl,
            globals_buf,
            uniform_bg,
            lit_bg,
            sampler,
            albedo_layout,
            instance_buf,
            part_quad,
            part_inst,
            ground,
            shell,
            tank_green: Vec::new(),
            tank_steel: Vec::new(),
            trees: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        })
    }

    pub fn upload_catalog(&mut self, cat: &Catalog) {
        let up = |meshes: &[CpuMesh]| -> Vec<MeshGpu> {
            meshes
                .iter()
                .map(|m| upload_mesh(&self.gpu.device, &self.gpu.queue, &self.albedo_layout, &self.sampler, m))
                .collect()
        };
        self.tank_green = up(&cat.tank_green);
        self.tank_steel = up(&cat.tank_steel);
        self.trees = [
            up(&cat.trees[0]),
            up(&cat.trees[1]),
            up(&cat.trees[2]),
            up(&cat.trees[3]),
        ];
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn size(&self) -> (u32, u32) {
        self.gpu.size()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        let (d, v) = make_depth(&self.gpu.device, self.gpu.config.width, self.gpu.config.height);
        self.depth = d;
        self.depth_view = v;
    }

    pub fn render(&mut self, game: &Game) -> bool {
        let draws = game.draw_list();
        let (eye, target) = game.camera_eye_target();
        let view = Mat4::look_at_rh(eye, target, Vec3::new(0.0, 1.0, 0.0));
        let proj = Mat4::perspective_rh(52.0_f32.to_radians(), self.gpu.aspect(), 0.4, 1100.0);
        let light = Vec3::new(0.42, -1.0, 0.22).normalized();
        let sun = Vec3::new(-light.x, -light.y, -light.z).scale(MAP_HALF * 1.85);
        let light_view = Mat4::look_at_rh(sun, Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
        let light_proj = Mat4::orthographic_rh(MAP_HALF + 28.0, MAP_HALF + 28.0, 1.0, MAP_HALF * 4.2);
        let globals = Globals {
            view_proj: proj.mul(view).m,
            light_view_proj: light_proj.mul(light_view).m,
            cam_pos: [eye.x, eye.y, eye.z, 1.0],
            cam_right: [view.m[0], view.m[4], view.m[8], 0.0],
            cam_up: [view.m[1], view.m[5], view.m[9], 0.0],
            light_dir: [light.x, light.y, light.z, 0.0],
            fog_color: [0.14, 0.16, 0.12, 1.0],
        };
        self.gpu
            .queue
            .write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&globals));

        let mut inst: Vec<Instance> = Vec::with_capacity(256);
        inst.push(Instance {
            transform: Mat4::IDENTITY.m,
            color: [1.0; 4],
        });
        let ground_r = 0u32..1;
        let g0 = inst.len() as u32;
        inst.extend_from_slice(&draws.green);
        let green_r = g0..inst.len() as u32;
        let s0 = inst.len() as u32;
        inst.extend_from_slice(&draws.steel);
        let steel_r = s0..inst.len() as u32;
        let mut tree_r = [0u32..0, 0u32..0, 0u32..0, 0u32..0];
        for k in 0..4 {
            let a = inst.len() as u32;
            inst.extend_from_slice(&draws.trees[k]);
            tree_r[k] = a..inst.len() as u32;
        }
        let sh0 = inst.len() as u32;
        inst.extend_from_slice(&draws.shells);
        let shell_r = sh0..inst.len() as u32;
        if inst.len() > INST_CAP as usize {
            inst.truncate(INST_CAP as usize);
        }
        self.gpu
            .queue
            .write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(&inst));

        let parts = &draws.particles;
        let part_n = parts.len().min(PART_CAP as usize) as u32;
        if part_n > 0 {
            self.gpu.queue.write_buffer(
                &self.part_inst,
                0,
                bytemuck::cast_slice(&parts[..part_n as usize]),
            );
        }

        let frame = match self.gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.gpu.surface.configure(&self.gpu.device, &self.gpu.config);
                match self.gpu.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(f)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                    _ => return false,
                }
            }
            _ => return false,
        };
        let surf_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tanks-frame"),
            });

        {
            let mut spass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            spass.set_pipeline(&self.shadow_pl);
            spass.set_bind_group(0, &self.uniform_bg, &[]);
            spass.set_vertex_buffer(1, self.instance_buf.slice(..));
            draw_shadow(&mut spass, &self.tank_green, green_r.clone());
            draw_shadow(&mut spass, &self.tank_steel, steel_r.clone());
            for k in 0..4 {
                draw_shadow(&mut spass, &self.trees[k], tree_r[k].clone());
            }
            draw_shadow(&mut spass, std::slice::from_ref(&self.shell), shell_r.clone());
        }

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("tanks"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surf_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.14,
                            g: 0.16,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.mesh_pl);
            rpass.set_bind_group(0, &self.lit_bg, &[]);
            rpass.set_vertex_buffer(1, self.instance_buf.slice(..));
            draw_parts(&mut rpass, std::slice::from_ref(&self.ground), ground_r);
            draw_parts(&mut rpass, &self.tank_green, green_r);
            draw_parts(&mut rpass, &self.tank_steel, steel_r);
            for k in 0..4 {
                draw_parts(&mut rpass, &self.trees[k], tree_r[k].clone());
            }
            draw_parts(&mut rpass, std::slice::from_ref(&self.shell), shell_r);

            if part_n > 0 {
                rpass.set_pipeline(&self.part_pl);
                rpass.set_bind_group(0, &self.uniform_bg, &[]);
                rpass.set_vertex_buffer(0, self.part_quad.slice(..));
                rpass.set_vertex_buffer(1, self.part_inst.slice(..));
                rpass.draw(0..6, 0..part_n);
            }
        }

        self.gpu.queue.submit(Some(encoder.finish()));
        self.gpu.queue.present(frame);
        true
    }
}

fn draw_shadow(
    rpass: &mut wgpu::RenderPass<'_>,
    parts: &[MeshGpu],
    instances: std::ops::Range<u32>,
) {
    if instances.is_empty() || parts.is_empty() {
        return;
    }
    for mesh in parts {
        if mesh.index_count == 0 {
            continue;
        }
        rpass.set_vertex_buffer(0, mesh.vertex.slice(..));
        rpass.set_index_buffer(mesh.index.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..mesh.index_count, 0, instances.clone());
    }
}

fn draw_parts(rpass: &mut wgpu::RenderPass<'_>, parts: &[MeshGpu], instances: std::ops::Range<u32>) {
    if instances.is_empty() || parts.is_empty() {
        return;
    }
    for mesh in parts {
        if mesh.index_count == 0 {
            continue;
        }
        rpass.set_bind_group(1, &mesh.albedo_bg, &[]);
        rpass.set_vertex_buffer(0, mesh.vertex.slice(..));
        rpass.set_index_buffer(mesh.index.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..mesh.index_count, 0, instances.clone());
    }
}

fn depth_state(write: bool) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth24Plus,
        depth_write_enabled: Some(write),
        depth_compare: Some(wgpu::CompareFunction::Less),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

fn shadow_depth_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::LessEqual),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState {
            constant: 2,
            slope_scale: 2.0,
            clamp: 0.0,
        },
    }
}

fn upload_mesh(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    albedo_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    mesh: &CpuMesh,
) -> MeshGpu {
    let n = mesh.positions.len();
    let mut verts = Vec::with_capacity(n);
    for i in 0..n {
        verts.push(Vertex {
            pos: mesh.positions[i],
            n: mesh.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
            uv: mesh.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
        });
    }
    if verts.is_empty() {
        verts.push(Vertex {
            pos: [0.0; 3],
            n: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
        });
    }
    let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("mesh-v"),
        contents: bytemuck::cast_slice(&verts),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let indices = if mesh.indices.is_empty() {
        vec![0u32]
    } else {
        mesh.indices.clone()
    };
    let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("mesh-i"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    let (albedo, view) = make_albedo(device, queue, mesh.albedo.as_ref(), mesh.color);
    let albedo_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("albedo-bg"),
        layout: albedo_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    MeshGpu {
        vertex,
        index,
        index_count: mesh.indices.len() as u32,
        _albedo: albedo,
        albedo_bg,
    }
}

fn make_albedo(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: Option<&RgbaImage>,
    color: [f32; 4],
) -> (wgpu::Texture, wgpu::TextureView) {
    let baked = color_to_srgb8(color);
    let (w, h, pixels): (u32, u32, &[u8]) = match img {
        Some(im) if !im.pixels.is_empty() => (im.width.max(1), im.height.max(1), im.pixels.as_slice()),
        _ => (1, 1, &baked),
    };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("albedo"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * w),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    (tex, view)
}

fn grass_image() -> RgbaImage {
    const W: u32 = 256;
    let mut pixels = vec![0u8; (W * W * 4) as usize];
    for y in 0..W {
        for x in 0..W {
            let n = hash(x, y);
            let n2 = hash(x.wrapping_mul(19), y.wrapping_mul(7));
            let dirt = n2 > 0.82;
            let (r, g, b) = if dirt {
                (0.38 + n * 0.08, 0.28 + n * 0.05, 0.14 + n * 0.03)
            } else {
                (
                    0.20 + n * 0.10,
                    0.34 + n * 0.14,
                    0.12 + n * 0.05,
                )
            };
            let i = ((y * W + x) * 4) as usize;
            pixels[i] = (r * 255.0) as u8;
            pixels[i + 1] = (g * 255.0) as u8;
            pixels[i + 2] = (b * 255.0) as u8;
            pixels[i + 3] = 255;
        }
    }
    RgbaImage {
        width: W,
        height: W,
        pixels,
    }
}

fn hash(x: u32, y: u32) -> f32 {
    let mut n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n >> 8) as f32 / 16_777_216.0
}

fn color_to_srgb8(c: [f32; 4]) -> [u8; 4] {
    let enc = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        let s = if x <= 0.0031308 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        (s.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
    };
    [enc(c[0]), enc(c[1]), enc(c[2]), (c[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8]
}

fn make_depth(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    (tex, view)
}

fn make_shadow(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow-map"),
        size: wgpu::Extent3d {
            width: SHADOW_SIZE,
            height: SHADOW_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    (tex, view)
}
