struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct PreviewParams {
    values: vec4<f32>,
};

@group(0) @binding(0) var t_source: texture_2d<f32>;
@group(0) @binding(1) var t_blur: texture_2d<f32>;
@group(0) @binding(2) var s_preview: sampler;
@group(0) @binding(3) var<uniform> params: PreviewParams;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(t_source, s_preview, in.uv);
    let blurred = textureSample(t_blur, s_preview, in.uv);
    let blur_mix = clamp(params.values.y, 0.0, 1.0);
    let dim = clamp(params.values.x, 0.0, 1.0);
    let color = source * (1.0 - blur_mix) + blurred * blur_mix;
    return vec4<f32>(color.rgb * (1.0 - dim), color.a);
}
