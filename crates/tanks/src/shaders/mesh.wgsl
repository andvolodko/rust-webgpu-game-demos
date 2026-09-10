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
@group(0) @binding(1) var shadow_map: texture_depth_2d;
@group(0) @binding(2) var shadow_samp: sampler_comparison;
@group(1) @binding(0) var albedo_tex: texture_2d<f32>;
@group(1) @binding(1) var albedo_samp: sampler;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) i_m0: vec4<f32>,
    @location(4) i_m1: vec4<f32>,
    @location(5) i_m2: vec4<f32>,
    @location(6) i_m3: vec4<f32>,
    @location(7) i_color: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
}

@vertex
fn vs_main(vin: VsIn) -> VsOut {
    let model = mat4x4<f32>(vin.i_m0, vin.i_m1, vin.i_m2, vin.i_m3);
    let world = model * vec4(vin.pos, 1.0);
    let n = normalize((model * vec4(vin.normal, 0.0)).xyz);
    var out: VsOut;
    out.clip = globals.view_proj * world;
    out.world = world.xyz;
    out.normal = n;
    out.uv = vin.uv;
    out.color = vin.i_color;
    return out;
}

@vertex
fn vs_shadow(vin: VsIn) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(vin.i_m0, vin.i_m1, vin.i_m2, vin.i_m3);
    let world = model * vec4(vin.pos, 1.0);
    return globals.light_view_proj * world;
}

fn shadow_at(world: vec3<f32>, n: vec3<f32>) -> f32 {
    let lp = globals.light_view_proj * vec4(world + n * 0.12, 1.0);
    let ndc = lp.xyz / max(lp.w, 1e-5);
    let uv = vec2(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5);
    let uv_c = clamp(uv, vec2(0.001), vec2(0.999));
    let z = clamp(ndc.z - 0.003, 0.0, 1.0);
    let texel = 1.0 / 2048.0;
    var s = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            s += textureSampleCompare(
                shadow_map,
                shadow_samp,
                uv_c + vec2(f32(x), f32(y)) * texel,
                z,
            );
        }
    }
    let inside = select(
        0.0,
        1.0,
        uv.x > 0.001 && uv.x < 0.999 && uv.y > 0.001 && uv.y < 0.999 && ndc.z > 0.0 && ndc.z < 1.0,
    );
    return mix(1.0, s / 9.0, inside);
}

@fragment
fn fs_main(vin: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(vin.normal);
    let l = normalize(-globals.light_dir.xyz);
    let ndl = max(dot(n, l), 0.0);
    let sh = mix(0.38, 1.0, shadow_at(vin.world, n));
    let albedo = textureSample(albedo_tex, albedo_samp, vin.uv);
    let lit = albedo.rgb * vin.color.rgb * (0.18 + 0.82 * ndl * sh);
    let dist = length(vin.world - globals.cam_pos.xyz);
    let fog = clamp((dist - 160.0) / 380.0, 0.0, 0.62);
    let rgb = mix(lit, globals.fog_color.rgb, fog);
    return vec4(rgb, 1.0);
}
