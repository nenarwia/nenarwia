struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct FragmentInput {
    @location(0) uv: vec2<f32>,
    @builtin(position) pos: vec4<f32>,
};

struct BackdropParams {
    source_size: vec2<f32>,
    target_size: vec2<f32>,
    surface_size: vec2<f32>,
    blur_axis: vec2<f32>,
    chrome_height_px: f32,
    pass_kind: u32,
    extra_blur_rect_count: u32,
    saturate: f32,
    extra_blur_rects: array<vec4<f32>, 2>,
};

// Passes:
// 0 = Copy/downsample (t_source -> output)
// 1 = 1D blur (t_source -> output along blur_axis)
// 2 = Composite (blurred=t_source, base=t_scene_base -> output; blur only under titlebar)
@group(0) @binding(0) var t_source: texture_2d<f32>;
@group(0) @binding(1) var s_source: sampler;
@group(0) @binding(2) var t_scene_base: texture_2d<f32>;
@group(0) @binding(3) var<uniform> params: BackdropParams;

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(positions[vid], 0.0, 1.0);
    out.uv = uvs[vid];
    return out;
}

fn blur_1d(tex: texture_2d<f32>, uv: vec2<f32>, axis: vec2<f32>) -> vec4<f32> {
    let texel = axis / params.source_size;
    // 11-tap gaussian kernel at downsampled resolution.
    // With quarter-res buffers, radius=5 approximates CSS blur(20px).
    var color = textureSample(tex, s_source, uv) * 0.1423;
    color = color + textureSample(tex, s_source, uv + texel * 1.0) * 0.1346;
    color = color + textureSample(tex, s_source, uv - texel * 1.0) * 0.1346;
    color = color + textureSample(tex, s_source, uv + texel * 2.0) * 0.1140;
    color = color + textureSample(tex, s_source, uv - texel * 2.0) * 0.1140;
    color = color + textureSample(tex, s_source, uv + texel * 3.0) * 0.0863;
    color = color + textureSample(tex, s_source, uv - texel * 3.0) * 0.0863;
    color = color + textureSample(tex, s_source, uv + texel * 4.0) * 0.0585;
    color = color + textureSample(tex, s_source, uv - texel * 4.0) * 0.0585;
    color = color + textureSample(tex, s_source, uv + texel * 5.0) * 0.0355;
    color = color + textureSample(tex, s_source, uv - texel * 5.0) * 0.0355;
    return color;
}

fn downsample_prefilter(uv: vec2<f32>) -> vec4<f32> {
    let texel = 1.0 / params.source_size;
    let step = texel * 2.0;

    let center = textureSample(t_source, s_source, uv) * 0.25;
    let cross = (
        textureSample(t_source, s_source, uv + vec2<f32>(step.x, 0.0)) +
        textureSample(t_source, s_source, uv - vec2<f32>(step.x, 0.0)) +
        textureSample(t_source, s_source, uv + vec2<f32>(0.0, step.y)) +
        textureSample(t_source, s_source, uv - vec2<f32>(0.0, step.y))
    ) * 0.125;
    let diag = (
        textureSample(t_source, s_source, uv + vec2<f32>(step.x, step.y)) +
        textureSample(t_source, s_source, uv + vec2<f32>(step.x, -step.y)) +
        textureSample(t_source, s_source, uv + vec2<f32>(-step.x, step.y)) +
        textureSample(t_source, s_source, uv + vec2<f32>(-step.x, -step.y))
    ) * 0.0625;
    return center + cross + diag;
}

fn rect_alpha(rect: vec4<f32>, pos: vec2<f32>, feather_px: f32) -> f32 {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.z;
    let y1 = rect.y + rect.w;
    let ax = smoothstep(x0 - feather_px, x0 + feather_px, pos.x) *
        (1.0 - smoothstep(x1 - feather_px, x1 + feather_px, pos.x));
    let ay = smoothstep(y0 - feather_px, y0 + feather_px, pos.y) *
        (1.0 - smoothstep(y1 - feather_px, y1 + feather_px, pos.y));
    return ax * ay;
}

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let pixel_center_uv = (floor(in.pos.xy) + vec2<f32>(0.5, 0.5)) / params.target_size;

    if params.pass_kind == 0u {
        // Proper low-pass before quarter-res write to avoid shimmer while moving.
        return downsample_prefilter(pixel_center_uv);
    }
    if params.pass_kind == 1u {
        return blur_1d(t_source, pixel_center_uv, params.blur_axis);
    }

    // Composite pass.
    let base = textureSample(t_scene_base, s_source, pixel_center_uv);
    let chrome_alpha = select(0.0, 1.0, in.pos.y <= params.chrome_height_px);
    let overlay_feather_px = 3.0;
    var overlay_alpha = 0.0;
    if params.extra_blur_rect_count > 0u {
        overlay_alpha = max(
            overlay_alpha,
            rect_alpha(params.extra_blur_rects[0], in.pos.xy, overlay_feather_px),
        );
    }
    if params.extra_blur_rect_count > 1u {
        overlay_alpha = max(
            overlay_alpha,
            rect_alpha(params.extra_blur_rects[1], in.pos.xy, overlay_feather_px),
        );
    }
    let blur_alpha = max(chrome_alpha, overlay_alpha);
    if blur_alpha <= 0.0001 {
        return base;
    }

    let blurred = textureSample(t_source, s_source, pixel_center_uv);
    let luma = dot(blurred.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let saturated_rgb = clamp(
        vec3<f32>(luma) + (blurred.rgb - vec3<f32>(luma)) * params.saturate,
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    let blur_color = vec4<f32>(saturated_rgb, 1.0);
    return base + (blur_color - base) * blur_alpha;
}
