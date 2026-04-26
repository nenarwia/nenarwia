use crate::core::loader::disk_cache as dc;
use crate::core::loader::types::{ImagePayload, LoadedImage};

use super::make_canvas_media_slot;

#[derive(Clone, Copy)]
pub(super) struct CanvasMediaSlotJob {
    pub id: u64,
    pub asset_key: u64,
    pub epoch: u64,
    pub lod: u8,
    pub tile_x: u32,
    pub tile_y: u32,
}

pub(super) fn make_rgba(
    job: CanvasMediaSlotJob,
    bytes: Vec<u8>,
    orig_w: u32,
    orig_h: u32,
) -> LoadedImage {
    make_canvas_media_slot(
        job.id,
        job.asset_key,
        job.epoch,
        job.lod,
        job.tile_x,
        job.tile_y,
        ImagePayload::Rgba8(bytes),
        false,
        orig_w,
        orig_h,
    )
}

pub(super) fn make_failed(job: CanvasMediaSlotJob, orig_w: u32, orig_h: u32) -> LoadedImage {
    make_canvas_media_slot(
        job.id,
        job.asset_key,
        job.epoch,
        job.lod,
        job.tile_x,
        job.tile_y,
        ImagePayload::Rgba8(Vec::new()),
        true,
        orig_w,
        orig_h,
    )
}

pub(super) fn extract_tile_with_halo(
    src_rgba: &[u8],
    src_w: u32,
    src_h: u32,
    tile_x: u32,
    tile_y: u32,
) -> Vec<u8> {
    let physical = dc::TILE_PHYSICAL_SIZE as usize;
    let logical = dc::TILE_SIZE as i64;
    let halo = dc::TILE_HALO as i64;
    let mut out = vec![0u8; physical.saturating_mul(physical).saturating_mul(4)];

    if src_w == 0 || src_h == 0 {
        return out;
    }
    let src_len_needed = (src_w as usize)
        .saturating_mul(src_h as usize)
        .saturating_mul(4);
    if src_rgba.len() < src_len_needed {
        return out;
    }

    let src_stride = (src_w as usize).saturating_mul(4);
    let dst_stride = physical.saturating_mul(4);
    let base_x = (tile_x as i64).saturating_mul(logical);
    let base_y = (tile_y as i64).saturating_mul(logical);
    let max_x = src_w.saturating_sub(1) as i64;
    let max_y = src_h.saturating_sub(1) as i64;

    for oy in 0..physical {
        let sy = (base_y + oy as i64 - halo).clamp(0, max_y) as usize;
        let src_row = sy.saturating_mul(src_stride);
        let dst_row = oy.saturating_mul(dst_stride);
        for ox in 0..physical {
            let sx = (base_x + ox as i64 - halo).clamp(0, max_x) as usize;
            let src_idx = src_row + sx.saturating_mul(4);
            let dst_idx = dst_row + ox.saturating_mul(4);
            out[dst_idx..dst_idx + 4].copy_from_slice(&src_rgba[src_idx..src_idx + 4]);
        }
    }

    out
}

pub(super) fn tile_payload_len_matches_rgba(bytes: &[u8]) -> bool {
    let physical = dc::TILE_PHYSICAL_SIZE as usize;
    let expected = physical.saturating_mul(physical).saturating_mul(4);
    bytes.len() >= expected
}
