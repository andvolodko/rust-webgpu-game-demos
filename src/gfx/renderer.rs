//! 3D-рендерер: перспектива, скло, bloom, HUD + GPU-партікли.

use super::gpu_particles::GpuParticles;
use super::hud::collect_hud;
use super::mesh::{box_mesh, octa_mesh, QUAD};
use super::scene::{collect_balls, collect_glass, collect_opaque};
use super::targets::Targets;
use super::types::{
    CrystalInstance, Globals, SpriteInstance, MESH_LAYOUT, QUAD_LAYOUT, SCENE_CAP, BALL_CAP,
    HUD_CAP,
};
use super::camera;
use crate::game::{Game, WORLD_H};
use crate::math::{game_to_world, Vec3};
use wgpu::util::DeviceExt;

pub struct Renderer {
    /// Має жити стільки ж, скільки `surface` — інакше wgpu панікує (SurfaceId).
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    globals_buf: wgpu::Buffer,
    globals_bg: wgpu::BindGroup,
    blur_h_buf: wgpu::Buffer,
    blur_v_buf: wgpu::Buffer,
    sampler: wgpu::Sampler,
    extract_layout: wgpu::BindGroupLayout,
    composite_layout: wgpu::BindGroupLayout,
    targets: Targets,
    pl_bg: wgpu::RenderPipeline,
    pl_opaque: wgpu::RenderPipeline,
    pl_glass: wgpu::RenderPipeline,
    pl_ball: wgpu::RenderPipeline,
    pl_extract: wgpu::RenderPipeline,
    pl_blur: wgpu::RenderPipeline,
    pl_composite: wgpu::RenderPipeline,
    pl_hud: wgpu::RenderPipeline,
    box_buf: wgpu::Buffer,
    box_count: u32,
    octa_buf: wgpu::Buffer,
    octa_count: u32,
    quad_buf: wgpu::Buffer,
    scene_inst: wgpu::Buffer,
    ball_inst: wgpu::Buffer,
    hud_inst: wgpu::Buffer,
    particles: GpuParticles,
}

fn shader(device: &wgpu::Device, label: &str, src: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(src.into()),
    })
}

fn write_inst<T: bytemuck::Pod>(queue: &wgpu::Queue, buf: &wgpu::Buffer, data: &[T]) {
    if data.is_empty() {
        return;
    }
    queue.write_buffer(buf, 0, bytemuck::cast_slice(data));
}

fn depth_stencil(write: bool) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth24Plus,
        depth_write_enabled: Some(write),
        depth_compare: Some(wgpu::CompareFunction::Less),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

fn color_target(format: wgpu::TextureFormat, blend: Option<wgpu::BlendState>) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend,
        write_mask: wgpu::ColorWrites::ALL,
    }
}

impl Renderer {
    pub async fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| format!("No suitable GPU adapter: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Bgra8Unorm || *f == wgpu::TextureFormat::Rgba8Unorm)
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![],
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("crystal-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| format!("request_device failed: {e}"))?;

        surface.configure(&device, &config);

        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let blur_h_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur-h"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let blur_v_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur-v"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals-layout"),
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
        let globals_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals-bg"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buf.as_entire_binding(),
            }],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let extract_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("extract-layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
        });
        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("composite-layout"),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let crystal_mod = shader(&device, "crystal", include_str!("../shaders/crystal.wgsl"));
        let bg_mod = shader(&device, "bg", include_str!("../shaders/background.wgsl"));
        let ball_mod = shader(&device, "ball", include_str!("../shaders/ball.wgsl"));
        let hud_mod = shader(&device, "hud", include_str!("../shaders/hud.wgsl"));
        let bloom_mod = shader(&device, "bloom", include_str!("../shaders/bloom.wgsl"));
        let composite_mod = shader(&device, "composite", include_str!("../shaders/composite.wgsl"));

        let crystal_pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("crystal-pl"),
            bind_group_layouts: &[Some(&globals_layout)],
            immediate_size: 0,
        });
        let empty_pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("empty-pl"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let extract_pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("extract-pl"),
            bind_group_layouts: &[Some(&extract_layout)],
            immediate_size: 0,
        });
        let composite_pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composite-pl"),
            bind_group_layouts: &[Some(&composite_layout)],
            immediate_size: 0,
        });

        let pl_bg = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bg"),
            layout: Some(&crystal_pl_layout),
            vertex: wgpu::VertexState {
                module: &bg_mod,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(depth_stencil(false)),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &bg_mod,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_target(format, None))],
            }),
            multiview_mask: None,
            cache: None,
        });

        let opaque_targets = [Some(color_target(format, None))];
        let glass_targets = [Some(color_target(
            format,
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        ))];
        let crystal_vs = || wgpu::VertexState {
            module: &crystal_mod,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(MESH_LAYOUT), Some(CrystalInstance::LAYOUT)],
        };

        let pl_opaque = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("opaque"),
            layout: Some(&crystal_pl_layout),
            vertex: crystal_vs(),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil(true)),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &crystal_mod,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &opaque_targets,
            }),
            multiview_mask: None,
            cache: None,
        });
        let pl_glass = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glass"),
            layout: Some(&crystal_pl_layout),
            vertex: crystal_vs(),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil(true)),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &crystal_mod,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &glass_targets,
            }),
            multiview_mask: None,
            cache: None,
        });

        let sprite_vert = |module| wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(QUAD_LAYOUT), Some(SpriteInstance::LAYOUT)],
        };

        let pl_ball = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ball"),
            layout: Some(&crystal_pl_layout),
            vertex: sprite_vert(&ball_mod),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(depth_stencil(false)),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &ball_mod,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_target(
                    format,
                    Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                ))],
            }),
            multiview_mask: None,
            cache: None,
        });

        let fs_target = [Some(color_target(format, None))];
        let pl_extract = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("extract"),
            layout: Some(&extract_pl_layout),
            vertex: wgpu::VertexState {
                module: &bloom_mod,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &bloom_mod,
                entry_point: Some("fs_extract"),
                compilation_options: Default::default(),
                targets: &fs_target,
            }),
            multiview_mask: None,
            cache: None,
        });
        let pl_blur = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blur"),
            layout: Some(&extract_pl_layout),
            vertex: wgpu::VertexState {
                module: &bloom_mod,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &bloom_mod,
                entry_point: Some("fs_blur"),
                compilation_options: Default::default(),
                targets: &fs_target,
            }),
            multiview_mask: None,
            cache: None,
        });
        let pl_composite = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("composite"),
            layout: Some(&composite_pl_layout),
            vertex: wgpu::VertexState {
                module: &composite_mod,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &composite_mod,
                entry_point: Some("fs_composite"),
                compilation_options: Default::default(),
                targets: &fs_target,
            }),
            multiview_mask: None,
            cache: None,
        });
        let pl_hud = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud"),
            layout: Some(&empty_pl_layout),
            vertex: sprite_vert(&hud_mod),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &hud_mod,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_target(
                    format,
                    Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                ))],
            }),
            multiview_mask: None,
            cache: None,
        });

        let box_verts = box_mesh();
        let octa_verts = octa_mesh();
        let box_count = box_verts.len() as u32;
        let octa_count = octa_verts.len() as u32;
        let box_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("box"),
            contents: bytemuck::cast_slice(&box_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let octa_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("octa"),
            contents: bytemuck::cast_slice(&octa_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let quad_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad"),
            contents: bytemuck::cast_slice(&QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mk_inst = |label, n, stride| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (n * stride) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let scene_inst = mk_inst("scene-inst", SCENE_CAP, 64);
        let ball_inst = mk_inst("ball-inst", BALL_CAP, 32);
        let hud_inst = mk_inst("hud-inst", HUD_CAP, 32);

        let particles = GpuParticles::new(&device, format, &globals_layout);
        let targets = Targets::new(
            &device,
            &config,
            &sampler,
            &extract_layout,
            &composite_layout,
            &blur_h_buf,
            &blur_v_buf,
        );

        Ok(Self {
            _instance: instance,
            surface,
            device,
            queue,
            config,
            globals_buf,
            globals_bg,
            blur_h_buf,
            blur_v_buf,
            sampler,
            extract_layout,
            composite_layout,
            targets,
            pl_bg,
            pl_opaque,
            pl_glass,
            pl_ball,
            pl_extract,
            pl_blur,
            pl_composite,
            pl_hud,
            box_buf,
            box_count,
            octa_buf,
            octa_count,
            quad_buf,
            scene_inst,
            ball_inst,
            hud_inst,
            particles,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
        self.targets = Targets::new(
            &self.device,
            &self.config,
            &self.sampler,
            &self.extract_layout,
            &self.composite_layout,
            &self.blur_h_buf,
            &self.blur_v_buf,
        );
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn render(&mut self, game: &mut Game, dt: f32) -> bool {
        let w = self.config.width.max(1);
        let h = self.config.height.max(1);
        let aspect = w as f32 / h as f32;
        let shake = game.shake;
        let t = game.time;
        let ox = (t * 47.0).sin() * shake * 16.0;
        let oz = (t * 31.0).cos() * shake * 10.0;
        let cam = camera::fit(aspect, ox, oz, game.cam_lean_x, game.cam_lean_y);
        let cam_right = [cam.view.m[0], cam.view.m[4], cam.view.m[8], 0.0];
        let cam_up = [cam.view.m[1], cam.view.m[5], cam.view.m[9], 0.0];
        let ball_light = game
            .balls
            .first()
            .map(|b| {
                let p = game_to_world(b.pos.x, b.pos.y, 12.0, WORLD_H);
                [p.x, p.y, p.z, 1.35]
            })
            .unwrap_or([0.0, 0.0, 0.0, 0.0]);
        let light = Vec3::new(0.4, -0.35, 1.0).normalized();
        let globals = Globals {
            view_proj: cam.view_proj.m,
            cam_pos: [cam.eye.x, cam.eye.y, cam.eye.z, 1.0],
            cam_right,
            cam_up,
            light_dir: [light.x, light.y, light.z, 0.0],
            ball_light,
            info: [t, shake, aspect, if game.immortal { 1.0 } else { 0.0 }],
        };
        self.queue
            .write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&globals));

        let tw = self.targets.bloom_w as f32;
        let th = self.targets.bloom_h as f32;
        self.queue.write_buffer(
            &self.blur_h_buf,
            0,
            bytemuck::bytes_of(&[1.0f32, 0.0, 1.0 / tw, 1.0 / th]),
        );
        self.queue.write_buffer(
            &self.blur_v_buf,
            0,
            bytemuck::bytes_of(&[0.0f32, 1.0, 1.0 / tw, 1.0 / th]),
        );

        self.particles
            .upload(&self.device, &self.queue, &mut game.particles);

        let mut scene_all = collect_opaque(game);
        let opaque_n = scene_all.len().min(SCENE_CAP);
        scene_all.extend(collect_glass(game));
        scene_all.truncate(SCENE_CAP);
        let glass_n = scene_all.len().saturating_sub(opaque_n);
        let balls = collect_balls(game);
        let hud = collect_hud(game, aspect);
        write_inst(&self.queue, &self.scene_inst, &scene_all);
        write_inst(&self.queue, &self.ball_inst, &balls);
        write_inst(&self.queue, &self.hud_inst, &hud);

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                match self.surface.get_current_texture() {
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
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });

        self.particles.simulate(&self.queue, &mut encoder, dt);

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.scene_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.03,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.targets.depth_view,
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
            rpass.set_bind_group(0, &self.globals_bg, &[]);
            rpass.set_pipeline(&self.pl_bg);
            rpass.draw(0..3, 0..1);

            rpass.set_pipeline(&self.pl_opaque);
            rpass.set_vertex_buffer(0, self.box_buf.slice(..));
            rpass.set_vertex_buffer(1, self.scene_inst.slice(..));
            if opaque_n > 0 {
                rpass.draw(0..self.box_count, 0..opaque_n as u32);
            }

            rpass.set_pipeline(&self.pl_glass);
            if glass_n > 0 {
                rpass.draw(
                    0..self.box_count,
                    opaque_n as u32..(opaque_n + glass_n) as u32,
                );
            }

            self.particles
                .draw_shards(&mut rpass, &self.octa_buf, self.octa_count);

            rpass.set_pipeline(&self.pl_ball);
            rpass.set_vertex_buffer(0, self.quad_buf.slice(..));
            rpass.set_vertex_buffer(1, self.ball_inst.slice(..));
            if !balls.is_empty() {
                rpass.draw(0..6, 0..balls.len() as u32);
            }

            self.particles.draw_sparks(&mut rpass, &self.quad_buf);
        }

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("extract"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.bloom_a_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.pl_extract);
            rpass.set_bind_group(0, &self.targets.extract_bg, &[]);
            rpass.draw(0..3, 0..1);
        }

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blur-h"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.bloom_b_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.pl_blur);
            rpass.set_bind_group(0, &self.targets.blur_a_bg, &[]);
            rpass.draw(0..3, 0..1);
        }
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blur-v"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.bloom_a_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.pl_blur);
            rpass.set_bind_group(0, &self.targets.blur_b_bg, &[]);
            rpass.draw(0..3, 0..1);
        }

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surf_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.pl_composite);
            rpass.set_bind_group(0, &self.targets.composite_bg, &[]);
            rpass.draw(0..3, 0..1);
        }

        if !hud.is_empty() {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surf_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.pl_hud);
            rpass.set_vertex_buffer(0, self.quad_buf.slice(..));
            rpass.set_vertex_buffer(1, self.hud_inst.slice(..));
            rpass.draw(0..6, 0..hud.len() as u32);
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        true
    }
}
