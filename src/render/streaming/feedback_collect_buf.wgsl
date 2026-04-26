const SAMPLE_GRID: u32 = 4u;
const WORKGROUP_SIZE: u32 = 64u;

struct Header {
    count: atomic<u32>,
    max_out: u32,
    overflow: atomic<u32>,
    instance_count: u32,
};

struct FeedbackInstance {
    asset_key_lo: u32,
    asset_key_hi: u32,
    desired_lod: u32,
    _pad0: u32,
    desired_tiles: vec2<f32>,
    _pad1: vec2<f32>,
};

struct TileOut {
    asset_key_lo: u32,
    asset_key_hi: u32,
    tile_x: u32,
    tile_y: u32,
    lod: u32,
};

@group(0) @binding(0) var<storage, read_write> header: Header;
@group(0) @binding(1) var<storage, read_write> out_tiles: array<TileOut>;
@group(0) @binding(2) var<storage, read> instances: array<FeedbackInstance>;

@compute @workgroup_size(WORKGROUP_SIZE)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= header.instance_count) {
        return;
    }

    let inst = instances[idx];
    if (inst.desired_lod == 0xFFFFFFFFu) {
        return;
    }

    let tiles_x = max(1u, u32(ceil(inst.desired_tiles.x)));
    let tiles_y = max(1u, u32(ceil(inst.desired_tiles.y)));

    let step_x = max(1u, tiles_x / SAMPLE_GRID);
    let step_y = max(1u, tiles_y / SAMPLE_GRID);

    var sy: u32 = 0u;
    loop {
        if (sy >= SAMPLE_GRID) { break; }
        var sx: u32 = 0u;
        loop {
            if (sx >= SAMPLE_GRID) { break; }
            let tx = min(tiles_x - 1u, sx * step_x);
            let ty = min(tiles_y - 1u, sy * step_y);
            let out_idx = atomicAdd(&header.count, 1u);
            if (out_idx < header.max_out) {
                out_tiles[out_idx] = TileOut(inst.asset_key_lo, inst.asset_key_hi, tx, ty, inst.desired_lod);
            } else {
                atomicStore(&header.overflow, 1u);
                return;
            }
            sx = sx + 1u;
        }
        sy = sy + 1u;
    }
}
