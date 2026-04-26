use crate::core::loader::disk_cache as dc;
use crate::core::loader::types::{ImagePayload, LoadedImage};

#[allow(clippy::too_many_arguments)]
pub(super) fn make_thumb(
    id: u64,
    asset_key: u64,
    epoch: u64,
    size: u16,
    payload: ImagePayload,
    missing: bool,
    orig_w: u32,
    orig_h: u32,
) -> LoadedImage {
    LoadedImage {
        id,
        asset_key,
        epoch,
        payload,
        width: size as u32,
        height: size as u32,
        is_detail: false,
        tile_x: 0,
        tile_y: 0,
        lod: 0,
        missing,
        orig_w,
        orig_h,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn make_tile(
    id: u64,
    asset_key: u64,
    epoch: u64,
    lod: u8,
    tile_x: u32,
    tile_y: u32,
    payload: ImagePayload,
    missing: bool,
    orig_w: u32,
    orig_h: u32,
) -> LoadedImage {
    LoadedImage {
        id,
        asset_key,
        epoch,
        payload,
        width: dc::TILE_PHYSICAL_SIZE,
        height: dc::TILE_PHYSICAL_SIZE,
        is_detail: true,
        tile_x,
        tile_y,
        lod,
        missing,
        orig_w,
        orig_h,
    }
}
