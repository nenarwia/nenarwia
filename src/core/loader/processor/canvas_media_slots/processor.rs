use std::path::Path;

use crate::core::loader::mem_cache as mc;
use crate::core::loader::types::LoadedImage;

use super::super::{
    allow_runtime_decode, decode_cached, decode_lod_cached, div_ceil, is_jpeg, make_tile,
    resolve_dims, WorkerState,
};

mod common;
mod early;
mod encode;
mod full_decode;
mod lod_cached;

use common::{make_failed, CanvasMediaSlotJob};
use early::try_early_paths;
use full_decode::try_build_from_full_decode;
use lod_cached::try_build_from_lod_cache;

#[allow(clippy::too_many_arguments)]
pub(in crate::core::loader::processor) fn process_canvas_media_slot_request(
    path: &Path,
    id: u64,
    asset_key: u64,
    lod: u8,
    tile_x: u32,
    tile_y: u32,
    orig_w_hint: u32,
    orig_h_hint: u32,
    epoch: u64,
    st: &mut WorkerState,
) -> LoadedImage {
    let mut orig_w = orig_w_hint;
    let mut orig_h = orig_h_hint;
    let job = CanvasMediaSlotJob {
        id,
        asset_key,
        epoch,
        lod,
        tile_x,
        tile_y,
    };

    if !mc::is_ram_media_slot_asset(asset_key) {
        resolve_dims(path, &mut orig_w, &mut orig_h);
        return make_failed(job, orig_w, orig_h);
    }

    if let Some(done) = try_early_paths(path, job, &mut orig_w, &mut orig_h) {
        return done;
    }

    if let Some(done) = try_build_from_lod_cache(path, job, &mut orig_w, &mut orig_h, st) {
        return done;
    }

    if let Some(done) = try_build_from_full_decode(path, job, st) {
        return done;
    }

    resolve_dims(path, &mut orig_w, &mut orig_h);
    make_failed(job, orig_w, orig_h)
}

pub(super) fn make_canvas_media_slot(
    id: u64,
    asset_key: u64,
    epoch: u64,
    lod: u8,
    tile_x: u32,
    tile_y: u32,
    payload: crate::core::loader::types::ImagePayload,
    missing: bool,
    orig_w: u32,
    orig_h: u32,
) -> LoadedImage {
    make_tile(
        id, asset_key, epoch, lod, tile_x, tile_y, payload, missing, orig_w, orig_h,
    )
}
