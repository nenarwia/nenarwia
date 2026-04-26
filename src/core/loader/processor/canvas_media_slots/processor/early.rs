use std::path::Path;

use crate::core::loader::disk_cache as dc;
use crate::core::loader::mem_cache as mc;
use crate::core::loader::types::LoadedImage;

use super::common::{make_failed, make_rgba, tile_payload_len_matches_rgba, CanvasMediaSlotJob};
use super::{allow_runtime_decode, resolve_dims};

pub(super) fn try_early_paths(
    path: &Path,
    job: CanvasMediaSlotJob,
    orig_w: &mut u32,
    orig_h: &mut u32,
) -> Option<LoadedImage> {
    if let Some(bytes) = mc::get_canvas_media_slot(job.asset_key, job.lod, job.tile_x, job.tile_y) {
        if tile_payload_len_matches_rgba(&bytes) {
            resolve_dims(path, orig_w, orig_h);
            return Some(make_rgba(job, bytes, *orig_w, *orig_h));
        }
    }

    if let Ok(rgba) =
        dc::read_canvas_media_slot_rgba_lz4(job.asset_key, job.lod, job.tile_x, job.tile_y)
    {
        if tile_payload_len_matches_rgba(&rgba) {
            resolve_dims(path, orig_w, orig_h);
            mc::put_canvas_media_slot(job.asset_key, job.lod, job.tile_x, job.tile_y, rgba.clone());
            return Some(make_rgba(job, rgba, *orig_w, *orig_h));
        }
    }

    if !allow_runtime_decode() {
        resolve_dims(path, orig_w, orig_h);
        return Some(make_failed(job, *orig_w, *orig_h));
    }

    None
}
