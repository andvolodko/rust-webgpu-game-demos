struct VsIn {
    @location(0) corner: vec2<f32>,
    @location(1) pos_size: vec4<f32>,
    @location(2) color: vec4<f32>,
};
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(inp: VsIn) -> VsOut {
    let c = inp.corner - vec2<f32>(0.5, 0.5);
    let ndc = inp.pos_size.xy + c * 2.0 * inp.pos_size.zw;
    var o: VsOut;
    o.pos = vec4<f32>(ndc, 0.0, 1.0);
    o.color = inp.color;
    return o;
}

@fragment
fn fs_main(inp: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(inp.color.rgb * inp.color.a, inp.color.a);
}
