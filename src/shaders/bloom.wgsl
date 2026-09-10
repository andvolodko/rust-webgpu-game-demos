struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    var p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    var o: VsOut;
    o.pos = vec4<f32>(p[i], 0.0, 1.0);
    o.uv = vec2<f32>(p[i].x * 0.5 + 0.5, 0.5 - p[i].y * 0.5);
    return o;
}

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@fragment
fn fs_extract(inp: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(src, samp, inp.uv);
    let b = max(max(c.r, c.g), c.b);
    let k = max(b - 0.62, 0.0) * 1.6;
    return vec4<f32>(c.rgb * k, 1.0);
}

@group(0) @binding(2) var<uniform> blur: vec4<f32>;

@fragment
fn fs_blur(inp: VsOut) -> @location(0) vec4<f32> {
    let dir = blur.xy * blur.zw;
    var acc = vec3<f32>(0.0);
    acc += textureSample(src, samp, inp.uv - dir * 3.0).rgb * 0.08;
    acc += textureSample(src, samp, inp.uv - dir * 2.0).rgb * 0.12;
    acc += textureSample(src, samp, inp.uv - dir).rgb * 0.18;
    acc += textureSample(src, samp, inp.uv).rgb * 0.24;
    acc += textureSample(src, samp, inp.uv + dir).rgb * 0.18;
    acc += textureSample(src, samp, inp.uv + dir * 2.0).rgb * 0.12;
    acc += textureSample(src, samp, inp.uv + dir * 3.0).rgb * 0.08;
    return vec4<f32>(acc, 1.0);
}
