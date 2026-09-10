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

struct VsIn {
    @location(0) mesh_pos: vec3<f32>,
    @location(1) mesh_n: vec3<f32>,
    @location(2) pos_glow: vec4<f32>,
    @location(3) extent: vec4<f32>,
    @location(4) rot: vec4<f32>,
    @location(5) color: vec4<f32>,
};
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
fn vs_main(inp: VsIn) -> VsOut {
    let R = euler(inp.rot.xyz);
    let local = inp.mesh_pos * inp.extent.xyz;
    let world = inp.pos_glow.xyz + R * local;
    var o: VsOut;
    o.clip = globals.view_proj * vec4<f32>(world, 1.0);
    o.world = world;
    o.normal = normalize(R * inp.mesh_n);
    o.color = inp.color;
    o.extra = vec3<f32>(inp.pos_glow.w, inp.extent.w, 0.0);
    return o;
}

@fragment
fn fs_main(inp: VsOut) -> @location(0) vec4<f32> {
    let N = normalize(inp.normal);
    let V = normalize(globals.cam_pos.xyz - inp.world);
    let L = normalize(globals.light_dir.xyz);
    let glow = inp.extra.x;
    let glass = inp.extra.y;
    let fresnel = pow(1.0 - max(dot(N, V), 0.0), 3.0);
    let diff = max(dot(N, L), 0.0);
    let spec = pow(max(dot(reflect(-L, N), V), 0.0), 48.0);
    let to_ball = globals.ball_light.xyz - inp.world;
    let bd = length(to_ball);
    let bl = max(dot(N, to_ball / max(bd, 0.001)), 0.0) * globals.ball_light.w / (1.0 + bd * bd * 0.00003);
    let inner = inp.color.rgb * (0.22 + 0.55 * diff + 0.35 * glow + 0.25 * bl);
    let edge = mix(inp.color.rgb, vec3<f32>(1.0, 0.98, 1.0), 0.65) * fresnel * (0.7 + 0.8 * glass);
    let rgb = inner + edge + vec3<f32>(spec) * (0.45 + 0.7 * glass) + inp.color.rgb * glow * 0.55;
    let alpha = mix(1.0, clamp(0.48 + fresnel * 0.40 + glow * 0.12, 0.0, 0.94), glass);
    return vec4<f32>(rgb * alpha, alpha);
}
