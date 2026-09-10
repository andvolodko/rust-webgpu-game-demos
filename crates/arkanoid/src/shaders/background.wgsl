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

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    var p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    var o: VsOut;
    o.pos = vec4<f32>(p[i], 0.0, 1.0);
    o.uv = p[i] * 0.5 + 0.5;
    return o;
}

fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var x = p;
    for (var i = 0; i < 4; i++) {
        v += a * noise(x);
        x = x * 2.07 + vec2<f32>(17.0, 9.0);
        a *= 0.5;
    }
    return v;
}

@fragment
fn fs_main(inp: VsOut) -> @location(0) vec4<f32> {
    let uv = inp.uv;
    let t = globals.info.x;
    let immortal = globals.info.w;
    let p = (uv - vec2<f32>(0.5, 0.42)) * vec2<f32>(1.7, 1.0);

    let n1 = fbm(p * 2.2 + vec2<f32>(t * 0.03, -t * 0.02));
    let n2 = fbm(p * 3.4 + vec2<f32>(-t * 0.04, t * 0.025) + vec2<f32>(n1, n1));
    let neb = smoothstep(0.28, 0.82, n2);

    let deep = vec3<f32>(0.012, 0.018, 0.05);
    let teal = vec3<f32>(0.07, 0.28, 0.48);
    let mag = vec3<f32>(0.42, 0.12, 0.55);
    let gold = vec3<f32>(0.45, 0.28, 0.12);
    var col = mix(deep, teal, neb);
    col = mix(col, mag, smoothstep(0.45, 0.9, n1) * 0.55);
    col = mix(col, gold, immortal * neb * 0.45);

    let aurora = sin(uv.x * 6.0 + t * 0.35 + n2 * 3.0) * 0.5 + 0.5;
    let band = smoothstep(0.35, 0.0, abs(uv.y - 0.38 - n1 * 0.12)) * aurora;
    col += vec3<f32>(0.12, 0.42, 0.55) * band * 0.22;
    col += vec3<f32>(0.55, 0.22, 0.45) * band * immortal * 0.18;

    let gv = floor(uv * vec2<f32>(90.0, 160.0));
    let star = hash21(gv);
    let spark = step(0.985, star) * pow(hash21(gv + 19.0), 8.0);
    let twinkle = 0.55 + 0.45 * sin(t * (2.0 + hash21(gv + 3.0) * 4.0) + star * 20.0);
    col += vec3<f32>(0.85, 0.92, 1.0) * spark * twinkle;

    let vignette = smoothstep(1.35, 0.18, length(uv - vec2<f32>(0.5, 0.46)));
    col *= vignette;
    return vec4<f32>(col, 1.0);
}
