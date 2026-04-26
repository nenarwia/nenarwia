fn fit_remap_uv_clamped(slot_uv: vec2<f32>, fit_rect: vec4<f32>) -> vec2<f32> {
    if (fit_rect.z <= 0.0 || fit_rect.w <= 0.0) {
        return vec2<f32>(0.0, 0.0);
    }

    let fit_size = max(fit_rect.zw, vec2<f32>(1e-5, 1e-5));
    let local = clamp(slot_uv - fit_rect.xy, vec2<f32>(0.0, 0.0), fit_rect.zw);
    return vec2<f32>(local.x / fit_size.x, local.y / fit_size.y);
}

fn fit_axis_coverage(coord: f32, min_v: f32, max_v: f32, aa: f32) -> f32 {
    let enter = smoothstep(min_v - aa, min_v + aa, coord);
    let exit = 1.0 - smoothstep(max_v - aa, max_v + aa, coord);
    return enter * exit;
}

fn fit_rect_coverage(slot_uv: vec2<f32>, fit_rect: vec4<f32>) -> f32 {
    if (fit_rect.z <= 0.0 || fit_rect.w <= 0.0) {
        return 0.0;
    }

    let rect_min = fit_rect.xy;
    let rect_max = rect_min + fit_rect.zw;
    let aa_x = max(fwidth(slot_uv.x) * 0.5, 1e-5);
    let aa_y = max(fwidth(slot_uv.y) * 0.5, 1e-5);
    return fit_axis_coverage(slot_uv.x, rect_min.x, rect_max.x, aa_x)
        * fit_axis_coverage(slot_uv.y, rect_min.y, rect_max.y, aa_y);
}
