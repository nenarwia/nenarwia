use std::path::Path;

use crate::core::loader::types::LoadedImage;

use super::common::{extract_tile_with_halo, CanvasMediaSlotJob};
use super::encode::finalize_raw_slot;
use super::{decode_lod_cached, div_ceil, resolve_dims, WorkerState};

pub(super) fn try_build_from_lod_cache(
    path: &Path,
    job: CanvasMediaSlotJob,
    orig_w: &mut u32,
    orig_h: &mut u32,
    st: &mut WorkerState,
) -> Option<LoadedImage> {
    if job.lod < 1 {
        return None;
    }

    resolve_dims(path, orig_w, orig_h);
    if *orig_w == 0 || *orig_h == 0 {
        return None;
    }

    let scale_lod = (1u32 << (job.lod.min(31) as u32)).max(1);
    let lod_w = div_ceil(*orig_w, scale_lod).max(1);
    let lod_h = div_ceil(*orig_h, scale_lod).max(1);
    let (lod_img, dec_w, dec_h) = decode_lod_cached(job.asset_key, job.lod, path, lod_w, lod_h)?;
    st.scratch_tile =
        extract_tile_with_halo(lod_img.as_raw(), dec_w, dec_h, job.tile_x, job.tile_y);
    Some(finalize_raw_slot(
        st.scratch_tile.clone(),
        job,
        *orig_w,
        *orig_h,
    ))
}
