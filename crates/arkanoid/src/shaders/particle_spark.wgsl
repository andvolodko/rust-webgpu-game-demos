struct Globals {
    view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    light_dir: vec4<f32>,
    ball_light: vec4<f32>,
    info: vec4<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;

struct Particle {
    pos: vec3<f32>,
    life: f32,
    vel: vec3<f32>,
    max_life: f32,
    angles: vec3<f32>,
    scale: f32,
    color: vec4<f32>,
    ang_vel: vec3<f32>,
    bounce: f32,
};
@group(1) @binding(0) var<storage, read> particles: array<Particle>;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) corner: vec2<f32>,
    @builtin(instance_index) inst: u32,
) -> VsOut {
    let p = particles[inst];
    var o: VsOut;
    if (p.life <= 0.0) {
        o.clip = vec4<f32>(0.0, 0.0, 2.0, 1.0);
        o.uv = vec2<f32>(0.0);
        o.color = vec4<f32>(0.0);
        return o;
    }
    let k = clamp(p.life / max(p.max_life, 0.001), 0.0, 1.0);
    let size = p.scale * (0.4 + 0.6 * k);
    let c = corner - vec2<f32>(0.5, 0.5);
    let world = p.pos
        + globals.cam_right.xyz * c.x * size
        + globals.cam_up.xyz * c.y * size;
    o.clip = globals.view_proj * vec4<f32>(world, 1.0);
    o.uv = corner;
    o.color = vec4<f32>(p.color.rgb, k);
    return o;
}

@fragment
fn fs_main(inp: VsOut) -> @location(0) vec4<f32> {
    let p = inp.uv * 2.0 - 1.0;
    let d = length(p);
    let a = exp(-d * d * 3.2) * inp.color.a;
    return vec4<f32>(inp.color.rgb * a, a);
}
