//! Offscreen scene + bloom targets.

pub struct Targets {
    _scene: wgpu::Texture,
    pub scene_view: wgpu::TextureView,
    _depth: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    _bloom_a: wgpu::Texture,
    pub bloom_a_view: wgpu::TextureView,
    _bloom_b: wgpu::Texture,
    pub bloom_b_view: wgpu::TextureView,
    pub extract_bg: wgpu::BindGroup,
    pub blur_a_bg: wgpu::BindGroup,
    pub blur_b_bg: wgpu::BindGroup,
    pub composite_bg: wgpu::BindGroup,
    pub bloom_w: u32,
    pub bloom_h: u32,
}

fn make_color_tex(
    device: &wgpu::Device,
    label: &str,
    w: u32,
    h: u32,
    format: wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    (tex, view)
}

impl Targets {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        sampler: &wgpu::Sampler,
        extract_layout: &wgpu::BindGroupLayout,
        composite_layout: &wgpu::BindGroupLayout,
        blur_h: &wgpu::Buffer,
        blur_v: &wgpu::Buffer,
    ) -> Self {
        let w = config.width.max(1);
        let h = config.height.max(1);
        let bw = (w / 2).max(1);
        let bh = (h / 2).max(1);
        let (scene, scene_view) = make_color_tex(device, "scene", w, h, config.format);
        let (bloom_a, bloom_a_view) = make_color_tex(device, "bloom-a", bw, bh, config.format);
        let (bloom_b, bloom_b_view) = make_color_tex(device, "bloom-b", bw, bh, config.format);
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

        let extract_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("extract-bg"),
            layout: extract_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: blur_h.as_entire_binding(),
                },
            ],
        });
        let blur_a_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur-a"),
            layout: extract_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&bloom_a_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: blur_h.as_entire_binding(),
                },
            ],
        });
        let blur_b_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur-b"),
            layout: extract_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&bloom_b_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: blur_v.as_entire_binding(),
                },
            ],
        });
        let composite_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite-bg"),
            layout: composite_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&bloom_a_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        Self {
            _scene: scene,
            scene_view,
            _depth: depth,
            depth_view,
            _bloom_a: bloom_a,
            bloom_a_view,
            _bloom_b: bloom_b,
            bloom_b_view,
            extract_bg,
            blur_a_bg,
            blur_b_bg,
            composite_bg,
            bloom_w: bw,
            bloom_h: bh,
        }
    }
}
