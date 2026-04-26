use crate::core::loader::disk_cache as dc;
use crate::core::loader::mem_cache as mc;
use crate::core::loader::types::LoadedImage;

use super::common::{make_rgba, CanvasMediaSlotJob};

pub(super) fn finalize_raw_slot(
    raw: Vec<u8>,
    job: CanvasMediaSlotJob,
    orig_w: u32,
    orig_h: u32,
) -> LoadedImage {
    // Match canvas preview cache ordering: write disk cache first, then warm RAM.
    let _ =
        dc::write_canvas_media_slot_rgba_lz4(job.asset_key, job.lod, job.tile_x, job.tile_y, &raw);
    mc::put_canvas_media_slot(job.asset_key, job.lod, job.tile_x, job.tile_y, raw.clone());
    make_rgba(job, raw, orig_w, orig_h)
}
