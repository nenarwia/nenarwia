use std::path::Path;

use crate::core::color::basic_decode::apply_icc_profile;
use crate::core::color::RgbaImage;

pub(super) fn extract_icc(
    frame: &windows::Win32::Graphics::Imaging::IWICBitmapFrameDecode,
) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Imaging::{IWICColorContext, WICColorContextProfile};

    let mut count: u32 = 0;
    let _ = unsafe { frame.GetColorContexts(&mut [], &mut count) };
    if count == 0 {
        return None;
    }
    let mut contexts: Vec<Option<IWICColorContext>> = vec![None; count as usize];
    unsafe { frame.GetColorContexts(&mut contexts, &mut count) }.ok()?;

    for ctx in contexts.into_iter().flatten() {
        let Ok(kind) = (unsafe { ctx.GetType() }) else {
            continue;
        };
        if kind != WICColorContextProfile {
            continue;
        }
        let mut needed: u32 = 0;
        let _ = unsafe { ctx.GetProfileBytes(&mut [], &mut needed) };
        if needed == 0 {
            continue;
        }
        let mut buf = vec![0u8; needed as usize];
        if unsafe { ctx.GetProfileBytes(&mut buf, &mut needed) }.is_ok() {
            buf.truncate(needed as usize);
            if !buf.is_empty() {
                return Some(buf);
            }
        }
    }
    None
}

pub(super) fn apply_icc_if_any(path: &Path, rgba: &mut RgbaImage, icc: Option<Vec<u8>>) {
    if let Some(profile) = icc {
        if let Err(err) = apply_icc_profile(rgba, &profile) {
            log::warn!("ICC: failed to apply WIC profile for {:?}: {err:?}", path);
        }
    }
}
