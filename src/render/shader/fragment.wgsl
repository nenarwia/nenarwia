fn apply_instance_interaction(base: vec4<f32>, in: VertexOutput) -> vec4<f32> {
    return vec4<f32>(base.rgb * in.color.rgb, base.a * in.color.a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let desired_mode = in.sample_flags.x;
    let coarse_mode = in.sample_flags.y;
    let atlas_mode = in.sample_flags.z;
    let media_coverage = fit_rect_coverage(in.local_uv, in.fit_rect);
    let media_uv_clamped = fit_remap_uv_clamped(in.local_uv, in.fit_rect);

    if (in.pt_pos.x >= 0.0) {
        let inside_media = media_coverage > 0.0;

        // Desired first
        if (inside_media) {
            let desired = sample_detail_tile(
                in.pt_pos,
                in.tile_count,
                media_uv_clamped,
                desired_mode
            );
            if (desired.a > 0.0) {
                let shaded = apply_instance_interaction(desired, in);
                return vec4<f32>(shaded.rgb, shaded.a * media_coverage);
            }
        }

        // Coarse fallback (progressive refinement, maps-style)
        if (inside_media && in.coarse_pt_pos.x >= 0.0 && in.coarse_tile_count.x > 0.0 && in.coarse_tile_count.y > 0.0) {
            let coarse = sample_detail_tile(
                in.coarse_pt_pos,
                in.coarse_tile_count,
                media_uv_clamped,
                coarse_mode
            );
            if (coarse.a > 0.0) {
                let shaded = apply_instance_interaction(coarse, in);
                return vec4<f32>(shaded.rgb, shaded.a * media_coverage);
            }
        }

        // Atlas fallback
        if (in.atlas_region.z > 0.0) {
            let atlas_uv = vec2<f32>(
                in.atlas_region.x + (in.tex_coords.x * in.atlas_region.z),
                in.atlas_region.y + (in.tex_coords.y * in.atlas_region.w)
            );
            let shaded = apply_instance_interaction(
                sample_thumb_region(atlas_uv, in.atlas_region, atlas_mode),
                in
            );
            return vec4<f32>(shaded.rgb, shaded.a * media_coverage);
        }
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    if (in.pt_pos.x > -1.5) {
        let shaded = apply_instance_interaction(
            sample_thumb_region(in.tex_coords, in.atlas_region, atlas_mode),
            in
        );
        return vec4<f32>(shaded.rgb, shaded.a * media_coverage);
    }

    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
