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
    @location(0) corner: vec2<f32>,
    @location(1) pos_size: vec4<f32>,
    @location(2) color: vec4<f32>,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(inp: VsIn) -> VsOut {
    let c = inp.corner - vec2<f32>(0.5, 0.5);
    let world = inp.pos_size.xyz
        + globals.cam_right.xyz * c.x * inp.pos_size.w * 2.3
        + globals.cam_up.xyz * c.y * inp.pos_size.w * 2.3;
    var o: VsOut;
    o.clip = globals.view_proj * vec4<f32>(world, 1.0);
    o.uv = inp.corner;
    o.color = inp.color;
    return o;
}

@fragment
fn fs_main(inp: VsOut) -> @location(0) vec4<f32> {
    let p = inp.uv * 2.0 - 1.0;
    let d2 = dot(p, p);
    if (d2 > 1.0) { discard; }
    let z = sqrt(max(1.0 - d2, 0.0));
    let N = normalize(vec3<f32>(p.x, p.y, z));
    let L = normalize(vec3<f32>(0.35, 0.15, 0.9));
    let diff = max(dot(N, L), 0.0);
    let spec = pow(max(dot(reflect(-L, N), vec3<f32>(0.0, 0.0, 1.0)), 0.0), 40.0);
    let fresnel = pow(1.0 - N.z, 3.0);
    let core = inp.color.rgb * (0.35 + 0.65 * diff);
    let rgb = core + vec3<f32>(1.0) * spec * 0.85
        + mix(inp.color.rgb, vec3<f32>(1.0), 0.5) * fresnel * 0.7;
    return vec4<f32>(rgb * 0.92, 0.92);
}
