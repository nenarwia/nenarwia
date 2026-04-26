const FEEDBACK_W: u32 = 256u;
const FEEDBACK_H: u32 = 144u;
const BLOCK_SIZE: u32 = 16u;

struct Header {
    count: atomic<u32>,
    max_out: u32,
    overflow: atomic<u32>,
    _pad: u32,
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
@group(0) @binding(2) var t_feedback: texture_2d<u32>;
@group(0) @binding(3) var t_valid: texture_2d<u32>;

@compute @workgroup_size(BLOCK_SIZE, BLOCK_SIZE, 1)
fn cs_main(
    @builtin(global_invocation_id) gid: vec3<u32>,
) {
    if (gid.x >= FEEDBACK_W || gid.y >= FEEDBACK_H) {
        return;
    }

    let lod_plus_one = textureLoad(t_valid, vec2<i32>(i32(gid.x), i32(gid.y)), 0).r;
    if (lod_plus_one == 0u) {
        return;
    }

    let v = textureLoad(t_feedback, vec2<i32>(i32(gid.x), i32(gid.y)), 0);
    let lod = lod_plus_one - 1u;
    let idx = atomicAdd(&header.count, 1u);
    if (idx < header.max_out) {
        out_tiles[idx] = TileOut(v.r, v.g, v.b, v.a, lod);
    } else {
        atomicStore(&header.overflow, 1u);
    }
}
