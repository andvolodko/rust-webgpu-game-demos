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

@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var bloom: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@fragment
fn fs_composite(inp: VsOut) -> @location(0) vec4<f32> {
    let s = textureSample(scene, samp, inp.uv);
    let b = textureSample(bloom, samp, inp.uv);
    return vec4<f32>(s.rgb + b.rgb * 0.9, 1.0);
}
