struct Globals {
    view_proj: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    light_dir: vec4<f32>,
    fog_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct VsIn {
    @location(0) corner: vec2<f32>,
    @location(1) pos_size: vec4<f32>,
    @location(2) color: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(vin: VsIn) -> VsOut {
    let world = vin.pos_size.xyz
        + globals.cam_right.xyz * vin.corner.x * vin.pos_size.w
        + globals.cam_up.xyz * vin.corner.y * vin.pos_size.w;
    var out: VsOut;
    out.clip = globals.view_proj * vec4(world, 1.0);
    out.uv = vin.corner * 0.5 + vec2(0.5, 0.5);
    out.color = vin.color;
    return out;
}

@fragment
fn fs_main(vin: VsOut) -> @location(0) vec4<f32> {
    let d = length(vin.uv * 2.0 - 1.0);
    let a = vin.color.a * smoothstep(1.0, 0.35, d);
    if a < 0.02 {
        discard;
    }
    return vec4(vin.color.rgb, a);
}
