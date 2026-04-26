pub const TILE_SIZE: u32 = 256;
pub const TILE_HALO: u32 = 2;
pub const TILE_PHYSICAL_SIZE: u32 = TILE_SIZE + TILE_HALO * 2;

// NOTE:
// The physical tile cache size is now configurable at runtime (VRAM budgeting).
// `TILE_SIZE` stays constant because it is baked into import/streaming logic.
