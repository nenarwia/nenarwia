use std::path::Path;

use image::imageops::FilterType;

use crate::core::color;
use crate::core::loader::types::LoadedImage;

use super::common::{extract_tile_with_halo, CanvasMediaSlotJob};
use super::encode::finalize_raw_slot;
use super::{decode_cached, div_ceil, is_jpeg, WorkerState};

pub(super) fn try_build_from_full_decode(
    path: &Path,
    job: CanvasMediaSlotJob,
    st: &mut WorkerState,
) -> Option<LoadedImage> {
    // For JPEG at LOD>0 we already have a scaled decode path.
    // Falling back to full decode here creates expensive CPU spikes.
    if job.lod > 0 && is_jpeg(path) {
        return None;
    }

    let (img, orig_w, orig_h) = decode_cached(st, job.asset_key, path)?;
    let raw = if job.lod == 0 {
        extract_tile_with_halo(img.as_raw(), orig_w, orig_h, job.tile_x, job.tile_y)
    } else {
        let scale = (1u32 << (job.lod.min(31) as u32)).max(1);
        let lod_w = div_ceil(orig_w, scale).max(1);
        let lod_h = div_ceil(orig_h, scale).max(1);
        let lod_raw = if color::gamma_correct_resize_enabled_value() {
            let mut out = vec![
                0u8;
                (lod_w as usize)
                    .saturating_mul(lod_h as usize)
                    .saturating_mul(4)
            ];
            color::resize_rgba8_srgb_gamma_correct_into(color::ResizeGammaInto {
                src_rgba8: img.as_raw(),
                src_stride_bytes: (orig_w as usize).saturating_mul(4),
                src_x: 0,
                src_y: 0,
                src_w: orig_w,
                src_h: orig_h,
                dst_w: lod_w,
                dst_h: lod_h,
                filter: FilterType::Lanczos3,
                dst_rgba8: &mut out,
                dst_stride_bytes: (lod_w as usize).saturating_mul(4),
                tmp_linear: &mut st.scratch_tmp,
            });
            out
        } else {
            color::resize_linear_rgba8_exact(img.as_ref(), lod_w, lod_h).into_raw()
        };
        extract_tile_with_halo(&lod_raw, lod_w, lod_h, job.tile_x, job.tile_y)
    };

    st.scratch_tile = raw;
    Some(finalize_raw_slot(
        st.scratch_tile.clone(),
        job,
        orig_w,
        orig_h,
    ))
}
