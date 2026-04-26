struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct WallpaperParams {
    dim: vec4<f32>,
};

@group(0) @binding(0) var t_wallpaper: texture_2d<f32>;
@group(0) @binding(1) var s_wallpaper: sampler;
@group(0) @binding(2) var<uniform> params: WallpaperParams;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = textureSample(t_wallpaper, s_wallpaper, in.uv);
    let dim = clamp(params.dim.x, 0.0, 1.0);
    let rgb = c.rgb * (1.0 - dim);
    return vec4<f32>(rgb, c.a);
}
