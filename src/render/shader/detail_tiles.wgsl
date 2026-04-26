fn sample_detail_tile(pt_pos: vec2<f32>, tile_count: vec2<f32>, uv01: vec2<f32>, mode: f32) -> vec4<f32> {
    // 1) UV (0..1) -> "virtual tiles" space
    let max_virtual = max(
        tile_count - vec2<f32>(TILE_EDGE_EPS, TILE_EDGE_EPS),
        vec2<f32>(0.0, 0.0)
    );
    let virtual_pos = clamp(uv01 * tile_count, vec2<f32>(0.0, 0.0), max_virtual);
    let tile_idx = vec2<i32>(floor(virtual_pos));

    // 2) local uv inside logical tile [0..1] sampled through a physical tile that
    // includes a halo border for cross-tile reconstruction filters.
    let local_uv = virtual_pos - vec2<f32>(tile_idx);

    // 3) page table lookup
    let pt_coord = vec2<i32>(pt_pos) + tile_idx;
    let entry = textureLoad(t_page_table, pt_coord, 0);

    if (entry.a > 0u) {
        let slot = u32(entry.r) + (u32(entry.g) * 256u);
        let cache_cols = cache.cache_cols;
        let col = slot % cache_cols;
        let row = slot / cache_cols;
        let tile_size_uv = 1.0 / f32(cache_cols);
        let inner_offset = TILE_HALO_PX / PHYSICAL_TILE_PX;
        let inner_scale = LOGICAL_TILE_PX / PHYSICAL_TILE_PX;

        let final_uv = vec2<f32>(
            (f32(col) + inner_offset + local_uv.x * inner_scale) * tile_size_uv,
            (f32(row) + inner_offset + local_uv.y * inner_scale) * tile_size_uv
        );

        let size_u = textureDimensions(t_detail);
        let size_i = vec2<i32>(size_u);
        let cols = max(1, i32(cache.cache_cols));
        let tile_px = max(1, size_i.x / cols);
        let origin = vec2<i32>(i32(col) * tile_px, i32(row) * tile_px);
        let clamp_min = origin;
        let clamp_max = origin + vec2<i32>(tile_px - 1, tile_px - 1);

        return sample_tex_clamped(t_detail, final_uv, mode, clamp_min, clamp_max);
    }

    // Mark missing: alpha=0
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
