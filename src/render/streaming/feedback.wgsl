const BLOCK_SIZE: u32 = 16u;

struct Header {
    count: atomic<u32>,
    task_count: u32,
    max_out: u32,
    overflow: atomic<u32>,
};

struct Task {
    asset_key_lo: u32,
    asset_key_hi: u32,
    lod: u32,
    pt_x: u32,
    pt_y: u32,
    base_tx: u32,
    base_ty: u32,
    count_x: u32,
    count_y: u32,
};

struct TileOut {
    task_index: u32,
    tile_x: u32,
    tile_y: u32,
    _pad: u32,
};

@group(0) @binding(0) var<storage, read_write> header: Header;
@group(0) @binding(1) var<storage, read> tasks: array<Task>;
@group(0) @binding(2) var<storage, read_write> out_tiles: array<TileOut>;
@group(0) @binding(3) var t_page_table: texture_2d<u32>;

@compute @workgroup_size(BLOCK_SIZE, BLOCK_SIZE, 1)
fn cs_main(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) gid: vec3<u32>,
) {
    let task_index = gid.z;
    if (task_index >= header.task_count) {
        return;
    }

    let task = tasks[task_index];
    if (lid.x >= task.count_x || lid.y >= task.count_y) {
        return;
    }

    let tile_x = task.base_tx + lid.x;
    let tile_y = task.base_ty + lid.y;
    let pt_coord = vec2<i32>(i32(task.pt_x + tile_x), i32(task.pt_y + tile_y));
    let entry = textureLoad(t_page_table, pt_coord, 0);

    if (entry.a == 0u) {
        let idx = atomicAdd(&header.count, 1u);
        if (idx < header.max_out) {
            out_tiles[idx] = TileOut(task_index, tile_x, tile_y, 0u);
        } else {
            atomicStore(&header.overflow, 1u);
        }
    }
}
