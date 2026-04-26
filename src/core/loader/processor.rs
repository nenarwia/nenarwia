use super::disk_cache as dc;
use super::types::{LoadRequest, LoadedImage};
use crate::core::color;

mod builders;
mod canvas_media_slots;
mod decode_cache;
mod settings;
mod throttle;
mod thumbnail;

use builders::{make_thumb, make_tile};
use canvas_media_slots::process_canvas_media_slot_request;
use decode_cache::{decode_cached, decode_lod_cached};
use settings::{allow_runtime_decode, div_ceil, is_jpeg};
use throttle::acquire_decode_guard;
use thumbnail::process_thumbnail;

pub use settings::{
    decode_cache_byte_limit_value, decode_cache_item_limit_value, lod_cache_byte_limit_value,
    lod_cache_item_limit_value, max_decode_jobs_value, runtime_decode_enabled, total_ram_gib_value,
};

// --- STATE ---

pub struct WorkerState {
    scratch_tile: Vec<u8>,
    scratch_tmp: Vec<u16>,
}

impl Default for WorkerState {
    fn default() -> Self {
        let tile_len = (dc::TILE_PHYSICAL_SIZE as usize)
            .saturating_mul(dc::TILE_PHYSICAL_SIZE as usize)
            .saturating_mul(4);
        Self {
            scratch_tile: vec![0u8; tile_len],
            scratch_tmp: Vec::new(),
        }
    }
}

// --- PUBLIC ENTRY POINT ---

pub fn process_request(req: LoadRequest, st: &mut WorkerState) -> LoadedImage {
    match req {
        LoadRequest::Thumbnail {
            path,
            id,
            asset_key,
            size,
            decode_mode,
            orig_w,
            orig_h,
            epoch,
        } => process_thumbnail(
            &path,
            id,
            asset_key,
            size,
            decode_mode,
            orig_w,
            orig_h,
            epoch,
        ),
        LoadRequest::CanvasMediaSlot {
            path,
            id,
            asset_key,
            lod,
            tile_x,
            tile_y,
            orig_w,
            orig_h,
            epoch,
        } => process_canvas_media_slot_request(
            &path, id, asset_key, lod, tile_x, tile_y, orig_w, orig_h, epoch, st,
        ),
    }
}

// --- INTERNAL LOGIC ---

fn resolve_dims(path: &std::path::Path, orig_w: &mut u32, orig_h: &mut u32) {
    if *orig_w > 0 && *orig_h > 0 {
        return;
    }
    let (w, h) = color::image_dimensions_any(path).unwrap_or((0, 0));
    if *orig_w == 0 {
        *orig_w = w;
    }
    if *orig_h == 0 {
        *orig_h = h;
    }
}
