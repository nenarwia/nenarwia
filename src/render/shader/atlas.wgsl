fn atlas_region_clamp_bounds(
    tex: texture_2d<f32>,
    region_min: vec2<f32>,
    region_max: vec2<f32>,
) -> vec4<i32> {
    let size_u = textureDimensions(tex);
    let size_f = vec2<f32>(size_u);
    let min_f = region_min * size_f - vec2<f32>(0.5, 0.5);
    let max_f = region_max * size_f - vec2<f32>(0.5, 0.5);
    return vec4<i32>(
        min(vec2<i32>(ceil(min_f)), vec2<i32>(floor(max_f))),
        max(vec2<i32>(ceil(min_f)), vec2<i32>(floor(max_f)))
    );
}

fn sample_thumb_from_atlas(
    tex: texture_2d<f32>,
    uv: vec2<f32>,
    region_min: vec2<f32>,
    region_max: vec2<f32>,
    mode: f32,
) -> vec4<f32> {
    let clamp_bounds = atlas_region_clamp_bounds(tex, region_min, region_max);
    return sample_tex_clamped(tex, uv, mode, clamp_bounds.xy, clamp_bounds.zw);
}

fn sample_thumb_region(encoded_uv: vec2<f32>, region: vec4<f32>, mode: f32) -> vec4<f32> {
    // UV.x is encoded as: tier + u
    let tier_f = floor(encoded_uv.x);
    let tier: u32 = u32(tier_f);
    let uv = vec2<f32>(encoded_uv.x - tier_f, encoded_uv.y);

    let region_tier_f = floor(region.x);
    let region_u0 = region.x - region_tier_f;
    let region_v0 = region.y;
    let region_u1 = region_u0 + region.z;
    let region_v1 = region_v0 + region.w;
    let region_min = vec2<f32>(region_u0, region_v0);
    let region_max = vec2<f32>(region_u1, region_v1);

    if (tier == 0u) {
        return sample_thumb_from_atlas(t_atlas32, uv, region_min, region_max, mode);
    } else if (tier == 1u) {
        return sample_thumb_from_atlas(t_atlas64, uv, region_min, region_max, mode);
    } else if (tier == 2u) {
        return sample_thumb_from_atlas(t_atlas128, uv, region_min, region_max, mode);
    } else if (tier == 3u) {
        return sample_thumb_from_atlas(t_atlas256, uv, region_min, region_max, mode);
    } else {
        return sample_thumb_from_atlas(t_atlas512, uv, region_min, region_max, mode);
    }
}
