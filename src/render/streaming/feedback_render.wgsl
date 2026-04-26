struct CameraUniform { view_proj: mat4x4<f32>, };
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct FeedbackInstance {
    asset_key_lo: u32,
    asset_key_hi: u32,
    desired_lod: u32,
    _pad0: u32,
    desired_tiles: vec2<f32>,
    _pad1: vec2<f32>,
};

@group(1) @binding(0) var<storage, read> feedback_instances: array<FeedbackInstance>;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
};

struct InstanceInput {
    @location(5) data: vec4<f32>,
    @location(6) color: vec4<f32>,
    @location(7) uv_region: vec4<f32>,
    @location(8) params: vec4<f32>,
    @location(9) params2: vec4<f32>,
    @location(10) sample_flags: vec4<f32>,
    @location(11) fit_rect: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tile_mode: f32,
    @location(2) instance_index: u32,
    @location(3) fit_rect: vec4<f32>,
};

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
    out.tex_coords = base_uv;
    out.tile_mode = select(0.0, 1.0, instance.params.x >= 0.0);
    out.instance_index = in.instance_index;
    out.fit_rect = instance.fit_rect;

    return out;
}

fn fit_remap_uv(slot_uv: vec2<f32>, fit_rect: vec4<f32>) -> vec2<f32> {
    if (fit_rect.z <= 0.0 || fit_rect.w <= 0.0) {
        return vec2<f32>(-1.0, -1.0);
    }

    let local = slot_uv - fit_rect.xy;
    let remapped = vec2<f32>(local.x / fit_rect.z, local.y / fit_rect.w);
    if (remapped.x < 0.0 || remapped.x > 1.0 || remapped.y < 0.0 || remapped.y > 1.0) {
        return vec2<f32>(-1.0, -1.0);
    }

    return remapped;
}

struct FragOut {
    @location(0) tile: vec4<u32>,
    @location(1) lod_plus_one: u32,
};

@fragment
fn fs_main(in: VertexOutput) -> FragOut {
    var out: FragOut;
    let inst = feedback_instances[in.instance_index];

    if (in.tile_mode < 0.5) {
        out.tile = vec4<u32>(0u, 0u, 0u, 0u);
        out.lod_plus_one = 0u;
        return out;
    }

    if (inst.desired_lod == 0xFFFFFFFFu) {
        out.tile = vec4<u32>(0u, 0u, 0u, 0u);
        out.lod_plus_one = 0u;
        return out;
    }

    let tiles_x = max(1u, u32(ceil(inst.desired_tiles.x)));
    let tiles_y = max(1u, u32(ceil(inst.desired_tiles.y)));
    if (tiles_x == 0u || tiles_y == 0u) {
        out.tile = vec4<u32>(0u, 0u, 0u, 0u);
        out.lod_plus_one = 0u;
        return out;
    }

    let media_uv = fit_remap_uv(in.tex_coords, in.fit_rect);
    if (media_uv.x < 0.0) {
        out.tile = vec4<u32>(0u, 0u, 0u, 0u);
        out.lod_plus_one = 0u;
        return out;
    }

    let tx = min(u32(floor(media_uv.x * f32(tiles_x))), tiles_x - 1u);
    let ty = min(u32(floor(media_uv.y * f32(tiles_y))), tiles_y - 1u);

    out.tile = vec4<u32>(inst.asset_key_lo, inst.asset_key_hi, tx, ty);
    out.lod_plus_one = inst.desired_lod + 1u;
    return out;
}
