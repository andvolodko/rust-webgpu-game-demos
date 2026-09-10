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
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) extra: vec3<f32>,
};

fn euler(a: vec3<f32>) -> mat3x3<f32> {
    let cx = cos(a.x); let sx = sin(a.x);
    let cy = cos(a.y); let sy = sin(a.y);
    let cz = cos(a.z); let sz = sin(a.z);
    let rx = mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, cx, sx),
        vec3<f32>(0.0, -sx, cx),
    );
    let ry = mat3x3<f32>(
        vec3<f32>(cy, 0.0, -sy),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(sy, 0.0, cy),
    );
    let rz = mat3x3<f32>(
        vec3<f32>(cz, sz, 0.0),
        vec3<f32>(-sz, cz, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    );
    return rz * ry * rx;
}

@vertex
fn vs_main(
    @location(0) mesh_pos: vec3<f32>,
    @location(1) mesh_n: vec3<f32>,
    @builtin(instance_index) inst: u32,
) -> VsOut {
    let p = particles[inst];
    var o: VsOut;
    if (p.life <= 0.0) {
        o.clip = vec4<f32>(0.0, 0.0, 2.0, 1.0);
        o.world = vec3<f32>(0.0);
        o.normal = vec3<f32>(0.0, 0.0, 1.0);
        o.color = vec4<f32>(0.0);
        o.extra = vec3<f32>(0.0);
        return o;
    }
    let k = clamp(p.life / max(p.max_life, 0.001), 0.0, 1.0);
    let R = euler(p.angles);
    let s = p.scale * (0.55 + 0.45 * k);
    let world = p.pos + R * (mesh_pos * s);
    o.clip = globals.view_proj * vec4<f32>(world, 1.0);
    o.world = world;
    o.normal = normalize(R * mesh_n);
    o.color = vec4<f32>(p.color.rgb, k);
    o.extra = vec3<f32>(0.35 * k, 0.85, 0.0);
    return o;
}

@fragment
fn fs_main(inp: VsOut) -> @location(0) vec4<f32> {
    if (inp.color.a <= 0.001) { discard; }
    let N = normalize(inp.normal);
    let V = normalize(globals.cam_pos.xyz - inp.world);
    let L = normalize(globals.light_dir.xyz);
    let glow = inp.extra.x;
    let glass = inp.extra.y;
    let fresnel = pow(1.0 - max(dot(N, V), 0.0), 3.0);
    let diff = max(dot(N, L), 0.0);
    let spec = pow(max(dot(reflect(-L, N), V), 0.0), 48.0);
    let inner = inp.color.rgb * (0.22 + 0.55 * diff + 0.35 * glow);
    let edge = mix(inp.color.rgb, vec3<f32>(1.0), 0.65) * fresnel * 0.8;
    let rgb = inner + edge + vec3<f32>(spec) * 0.7;
    let alpha = clamp(0.48 + fresnel * 0.40 + glow * 0.12, 0.0, 0.94) * inp.color.a;
    return vec4<f32>(rgb * alpha, alpha);
}
