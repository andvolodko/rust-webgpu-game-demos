//! Compute-симуляція 65k октаедрів + 65k іскор. CPU лише пише spawn.

use super::types::{MESH_LAYOUT, QUAD_LAYOUT};
use crate::particles::{ParticleSystem, ParticleUpload, MAX_SHARDS, MAX_SPARKLES, PARTICLE_STRIDE};

const WORKGROUP: u32 = 64;
const SHARD_GRAVITY: f32 = 520.0;
const SHARD_DRAG: f32 = 0.985;
const SPARK_GRAVITY: f32 = 520.0 * 0.35;
const SPARK_DRAG: f32 = 1.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SimParams {
    dt: f32,
    gravity: f32,
    drag: f32,
    _pad: f32,
}

pub struct GpuParticles {
    shard_buf: wgpu::Buffer,
    spark_buf: wgpu::Buffer,
    shard_sim_buf: wgpu::Buffer,
    spark_sim_buf: wgpu::Buffer,
    compute_pl: wgpu::ComputePipeline,
    shard_sim_bg: wgpu::BindGroup,
    spark_sim_bg: wgpu::BindGroup,
    shard_draw_bg: wgpu::BindGroup,
    spark_draw_bg: wgpu::BindGroup,
    shard_pl: wgpu::RenderPipeline,
    spark_pl: wgpu::RenderPipeline,
    shard_live: u32,
    spark_live: u32,
}

fn particle_buf(device: &wgpu::Device, label: &str, count: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: count as u64 * PARTICLE_STRIDE,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn sim_buf(device: &wgpu::Device, label: &str) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: std::mem::size_of::<SimParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn write_uploads(queue: &wgpu::Queue, buf: &wgpu::Buffer, uploads: &[ParticleUpload]) {
    if uploads.is_empty() {
        return;
    }
    let mut i = 0;
    while i < uploads.len() {
        let start = uploads[i].index;
        let mut run = vec![uploads[i].particle];
        let mut expected = start.wrapping_add(1);
        i += 1;
        while i < uploads.len() && uploads[i].index == expected {
            run.push(uploads[i].particle);
            expected = expected.wrapping_add(1);
            i += 1;
        }
        queue.write_buffer(
            buf,
            start as u64 * PARTICLE_STRIDE,
            bytemuck::cast_slice(&run),
        );
    }
}

impl GpuParticles {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        globals_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shard_buf = particle_buf(device, "gpu-shards", MAX_SHARDS);
        let spark_buf = particle_buf(device, "gpu-sparks", MAX_SPARKLES);
        let shard_sim_buf = sim_buf(device, "sim-shards");
        let spark_sim_buf = sim_buf(device, "sim-sparks");

        let sim_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle-sim-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(PARTICLE_STRIDE),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<SimParams>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });
        let draw_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle-draw-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(PARTICLE_STRIDE),
                },
                count: None,
            }],
        });

        let shard_sim_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shard-sim-bg"),
            layout: &sim_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shard_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: shard_sim_buf.as_entire_binding(),
                },
            ],
        });
        let spark_sim_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("spark-sim-bg"),
            layout: &sim_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: spark_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: spark_sim_buf.as_entire_binding(),
                },
            ],
        });
        let shard_draw_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shard-draw-bg"),
            layout: &draw_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shard_buf.as_entire_binding(),
            }],
        });
        let spark_draw_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("spark-draw-bg"),
            layout: &draw_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: spark_buf.as_entire_binding(),
            }],
        });

        let sim_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle-sim"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/particle_sim.wgsl").into()),
        });
        let shard_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle-shard"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/particle_shard.wgsl").into()),
        });
        let spark_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle-spark"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/particle_spark.wgsl").into()),
        });

        let sim_pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle-sim-pl"),
            bind_group_layouts: &[Some(&sim_layout)],
            immediate_size: 0,
        });
        let draw_pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle-draw-pl"),
            bind_group_layouts: &[Some(globals_layout), Some(&draw_layout)],
            immediate_size: 0,
        });

        let compute_pl = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("particle-sim"),
            layout: Some(&sim_pl_layout),
            module: &sim_mod,
            entry_point: Some("simulate"),
            compilation_options: Default::default(),
            cache: None,
        });

        let depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };
        let color_glass = wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        };
        let color_add = wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            write_mask: wgpu::ColorWrites::ALL,
        };

        let shard_targets = [Some(color_glass)];
        let spark_targets = [Some(color_add)];

        let shard_pl = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gpu-shards"),
            layout: Some(&draw_pl_layout),
            vertex: wgpu::VertexState {
                module: &shard_mod,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(MESH_LAYOUT)],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil.clone()),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shard_mod,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &shard_targets,
            }),
            multiview_mask: None,
            cache: None,
        });

        let spark_pl = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gpu-sparks"),
            layout: Some(&draw_pl_layout),
            vertex: wgpu::VertexState {
                module: &spark_mod,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(QUAD_LAYOUT)],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                depth_write_enabled: Some(false),
                ..depth_stencil
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &spark_mod,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &spark_targets,
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            shard_buf,
            spark_buf,
            shard_sim_buf,
            spark_sim_buf,
            compute_pl,
            shard_sim_bg,
            spark_sim_bg,
            shard_draw_bg,
            spark_draw_bg,
            shard_pl,
            spark_pl,
            shard_live: 0,
            spark_live: 0,
        }
    }

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, system: &mut ParticleSystem) {
        let (clear, shards, sparks, shard_live, spark_live) = system.drain_uploads();
        if clear {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("particle-clear"),
            });
            encoder.clear_buffer(&self.shard_buf, 0, None);
            encoder.clear_buffer(&self.spark_buf, 0, None);
            queue.submit(Some(encoder.finish()));
        }
        write_uploads(queue, &self.shard_buf, &shards);
        write_uploads(queue, &self.spark_buf, &sparks);
        self.shard_live = shard_live;
        self.spark_live = spark_live;
    }

    pub fn simulate(&self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, dt: f32) {
        if self.shard_live == 0 && self.spark_live == 0 {
            return;
        }
        let dt = dt.clamp(0.0, 0.05);
        queue.write_buffer(
            &self.shard_sim_buf,
            0,
            bytemuck::bytes_of(&SimParams {
                dt,
                gravity: SHARD_GRAVITY,
                drag: SHARD_DRAG,
                _pad: 0.0,
            }),
        );
        queue.write_buffer(
            &self.spark_sim_buf,
            0,
            bytemuck::bytes_of(&SimParams {
                dt,
                gravity: SPARK_GRAVITY,
                drag: SPARK_DRAG,
                _pad: 0.0,
            }),
        );

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("particle-sim"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.compute_pl);
        if self.shard_live > 0 {
            cpass.set_bind_group(0, &self.shard_sim_bg, &[]);
            cpass.dispatch_workgroups((self.shard_live + WORKGROUP - 1) / WORKGROUP, 1, 1);
        }
        if self.spark_live > 0 {
            cpass.set_bind_group(0, &self.spark_sim_bg, &[]);
            cpass.dispatch_workgroups((self.spark_live + WORKGROUP - 1) / WORKGROUP, 1, 1);
        }
    }

    pub fn draw_shards<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, octa: &'a wgpu::Buffer, count: u32) {
        if self.shard_live == 0 {
            return;
        }
        rpass.set_pipeline(&self.shard_pl);
        rpass.set_bind_group(1, &self.shard_draw_bg, &[]);
        rpass.set_vertex_buffer(0, octa.slice(..));
        rpass.draw(0..count, 0..self.shard_live);
    }

    pub fn draw_sparks<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, quad: &'a wgpu::Buffer) {
        if self.spark_live == 0 {
            return;
        }
        rpass.set_pipeline(&self.spark_pl);
        rpass.set_bind_group(1, &self.spark_draw_bg, &[]);
        rpass.set_vertex_buffer(0, quad.slice(..));
        rpass.draw(0..6, 0..self.spark_live);
    }
}
