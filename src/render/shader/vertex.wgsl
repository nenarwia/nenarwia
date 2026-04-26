@vertex
fn vs_main(in: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, 0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, -0.5),
        vec2<f32>(0.5, -0.5),
        vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, 0.5)
    );
    var local_uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0)
    );

    let base_pos = pos[in.vertex_index];
    let base_uv = local_uvs[in.vertex_index];

    let world_pos = vec4<f32>(
        (base_pos.x * instance.data.z) + instance.data.x,
        (base_pos.y * instance.data.w) + instance.data.y,
        0.0,
        1.0
    );
    out.clip_position = camera.view_proj * world_pos;
    out.color = instance.color;
    out.atlas_region = instance.uv_region;
    out.sample_flags = instance.sample_flags;
    out.local_uv = base_uv;
    out.fit_rect = instance.fit_rect;

    // Tile streaming mode
    if (instance.params.x >= 0.0) {
        out.pt_pos = vec2<f32>(instance.params.x, instance.params.y);
        out.tile_count = vec2<f32>(instance.params.z, instance.params.w);

        out.coarse_pt_pos = vec2<f32>(instance.params2.x, instance.params2.y);
        out.coarse_tile_count = vec2<f32>(instance.params2.z, instance.params2.w);

        out.tex_coords = base_uv;
    } else {
        // Atlas mode
        out.pt_pos = vec2<f32>(-1.0, -1.0);
        out.tile_count = vec2<f32>(0.0, 0.0);
        out.coarse_pt_pos = vec2<f32>(-1.0, -1.0);
        out.coarse_tile_count = vec2<f32>(0.0, 0.0);

        if (instance.uv_region.z > 0.0) {
            out.tex_coords = vec2<f32>(
                instance.uv_region.x + (base_uv.x * instance.uv_region.z),
                instance.uv_region.y + (base_uv.y * instance.uv_region.w)
            );
        } else {
            out.pt_pos = vec2<f32>(-2.0, -2.0);
            out.tex_coords = vec2<f32>(0.0, 0.0);
        }
    }

    return out;
}
