pub const TILE_SIZE: u32 = 256;
pub const TILE_HALO: u32 = 2;
pub const TILE_PHYSICAL_SIZE: u32 = TILE_SIZE + TILE_HALO * 2;

pub const THUMB_CODEC: &str = "rgba_lz4";
pub const TILE_CODEC: &str = "rgba_lz4_tile";

mod io_ops;
mod maintenance;
mod paths;
mod shared;

pub use io_ops::{
    read_canvas_media_slot_rgba_lz4, read_thumb_rgba_lz4, read_tile_rgba_lz4,
    write_canvas_media_slot_rgba_lz4, write_thumb_rgba_lz4, write_tile_rgba_lz4,
};
pub use maintenance::{
    bump_library_generation, bump_runtime_generation, clear_runtime_cache, compact_library_pack,
    compact_runtime_pack, delete_runtime_asset,
};
pub use paths::{cache_root, library_root, state_root};
