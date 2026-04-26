use std::path::Path;

use windows::core::Error;

use super::once::warn_once;

pub(super) fn log_decode_error(path: &Path, err: &Error) {
    use windows::Win32::Foundation::{
        WINCODEC_ERR_COMPONENTNOTFOUND, WINCODEC_ERR_UNKNOWNIMAGEFORMAT,
    };

    let code = err.code();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    if code == WINCODEC_ERR_COMPONENTNOTFOUND || code == WINCODEC_ERR_UNKNOWNIMAGEFORMAT {
        if warn_once(&ext) {
            log::warn!(
                "WIC: no system codec for {:?}. {}",
                path,
                crate::core::formats::wic_codec_hint_for_ext(ext.as_str())
            );
        }
    } else {
        let key = format!("{ext}:internal");
        if warn_once(&key) {
            log::warn!("WIC: decode failed for {:?}: {err:?}", path);
        }
    }
}
