use crate::core::vram::VramInfo;
use crate::render::cache::constants::TILE_PHYSICAL_SIZE;

const THUMB_PAGE_PX: [u32; 5] = [32, 64, 128, 256, 512];
const THUMB_WEIGHT: [f32; 5] = [0.45, 0.27, 0.16, 0.08, 0.04];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CacheConfig {
    pub thumb_atlas_dim_32: u32,
    pub thumb_atlas_dim_64: u32,
    pub thumb_atlas_dim_128: u32,
    pub thumb_atlas_dim_256: u32,
    pub thumb_atlas_dim_512: u32,
    pub tile_cache_dim: u32,
    pub cache_cols: u32,
    pub prefetch_radius_tiles: u32,
    pub max_canvas_media_slot_requests_per_frame: usize,
    pub max_thumb_requests_per_frame: usize,
}

/// Pick a config based on *current* VRAM pressure.
///
/// We bias toward safety: avoid massive upfront allocations.
/// The app can still look sharp because quality is driven by screen-space LOD,
/// not by "load everything".
pub fn decide_cache_config(vram: Option<VramInfo>, max_dim: u32) -> CacheConfig {
    // Defaults (safe baseline).
    let mut thumb_dims = [4096u32, 4096u32, 2048u32, 1024u32, 1024u32];

    let mut tile_cache_dim = 4096u32;
    let mut prefetch = 1u32;
    let mut max_tiles = 128usize;
    let mut max_thumbs = 64usize;

    if let Some(v) = vram {
        let free = v.free_gib();
        let mut enabled = [true, true, true, true, true];

        if free >= 6.0 {
            tile_cache_dim = 8192;
            prefetch = 2;
            max_tiles = 512;
            max_thumbs = 160;
        } else if free >= 3.0 {
            tile_cache_dim = 8192;
            prefetch = 2;
            max_tiles = 384;
            max_thumbs = 128;
        } else if free >= 1.5 {
            enabled = [true, true, true, false, false];

            tile_cache_dim = 4096;
            prefetch = 1;
            max_tiles = 192;
            max_thumbs = 80;
        } else {
            // Very tight VRAM: keep only base coverage tiers.
            enabled = [true, true, false, false, false];

            tile_cache_dim = 4096;
            prefetch = 1;
            max_tiles = 96;
            max_thumbs = 48;
        }

        // Budget-driven preview sizing with low-tier priority.
        let auto_budget_bytes = ((free * 1024.0 * 1024.0 * 1024.0) * 0.10) as u64;
        let min_budget = 64u64 * 1024 * 1024;
        let max_budget = 268u64 * 1024 * 1024;
        let preview_budget_bytes = preview_budget_override_bytes()
            .unwrap_or(auto_budget_bytes.clamp(min_budget, max_budget));
        thumb_dims = thumb_dims_from_budget(preview_budget_bytes, enabled, max_dim);
    }

    apply_thumb_env_overrides(&mut thumb_dims, max_dim);

    // Clamp to device limits.
    let thumb32 = thumb_dims[0].min(max_dim);
    let thumb64 = thumb_dims[1].min(max_dim);
    let thumb128 = thumb_dims[2].min(max_dim);
    let thumb256 = thumb_dims[3].min(max_dim);
    let thumb512 = thumb_dims[4].min(max_dim);

    let tile_cache_dim = tile_cache_dim.min(max_dim);
    let cache_cols = (tile_cache_dim / TILE_PHYSICAL_SIZE).max(1);

    CacheConfig {
        thumb_atlas_dim_32: thumb32,
        thumb_atlas_dim_64: thumb64,
        thumb_atlas_dim_128: thumb128,
        thumb_atlas_dim_256: thumb256,
        thumb_atlas_dim_512: thumb512,
        tile_cache_dim,
        cache_cols,
        prefetch_radius_tiles: prefetch,
        max_canvas_media_slot_requests_per_frame: max_tiles,
        max_thumb_requests_per_frame: max_thumbs,
    }
}

fn preview_budget_override_bytes() -> Option<u64> {
    if let Ok(v) = std::env::var("CANVAS_PREVIEW_ATLAS_BUDGET_BYTES") {
        if let Ok(bytes) = v.parse::<u64>() {
            return Some(bytes);
        }
    }
    if let Ok(v) = std::env::var("CANVAS_PREVIEW_ATLAS_BUDGET_MB") {
        if let Ok(mb) = v.parse::<u64>() {
            return Some(mb.saturating_mul(1024 * 1024));
        }
    }
    None
}

fn thumb_dims_from_budget(total_bytes: u64, enabled: [bool; 5], max_dim: u32) -> [u32; 5] {
    let mut dims = [0u32; 5];
    let mut weight_sum = 0.0f32;
    for i in 0..enabled.len() {
        if enabled[i] {
            weight_sum += THUMB_WEIGHT[i];
        }
    }
    if weight_sum <= f32::EPSILON {
        return dims;
    }

    let total_bytes_f = total_bytes as f32;
    for i in 0..enabled.len() {
        if !enabled[i] {
            dims[i] = 0;
            continue;
        }
        let tier_budget = total_bytes_f * (THUMB_WEIGHT[i] / weight_sum);
        let dim = (tier_budget / 4.0).sqrt() as u32;
        dims[i] = snap_thumb_dim(dim, THUMB_PAGE_PX[i], max_dim);
    }
    dims
}

fn snap_thumb_dim(dim: u32, page: u32, max_dim: u32) -> u32 {
    if dim == 0 {
        return 0;
    }
    let mut d = dim.max(page).min(max_dim);
    d = (d / page) * page;
    d.max(page)
}

fn apply_thumb_env_overrides(dims: &mut [u32; 5], max_dim: u32) {
    let vars = [
        "CANVAS_PREVIEW_ATLAS_DIM_32",
        "CANVAS_PREVIEW_ATLAS_DIM_64",
        "CANVAS_PREVIEW_ATLAS_DIM_128",
        "CANVAS_PREVIEW_ATLAS_DIM_256",
        "CANVAS_PREVIEW_ATLAS_DIM_512",
    ];
    for i in 0..vars.len() {
        if let Ok(v) = std::env::var(vars[i]) {
            if let Ok(parsed) = v.parse::<u32>() {
                dims[i] = if parsed == 0 {
                    0
                } else {
                    snap_thumb_dim(parsed, THUMB_PAGE_PX[i], max_dim)
                };
            }
        }
    }
}
