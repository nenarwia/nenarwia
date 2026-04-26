use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WicCodecFamily {
    Heif,
    Avif,
    Raw,
    Jpeg2000,
    JpegXr,
}

// Built-in formats we support without WIC-specific codec mapping.
const COMMON_EXTS: &[&str] = &[
    "jpg", "jpeg", "jpe", "jfif", "png", "bmp", "dib", "gif", "tif", "tiff", "webp", "ico",
];

pub const FILE_DIALOG_IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "jpe", "jfif", "png", "bmp", "dib", "gif", "tif", "tiff", "webp", "ico", "heic",
    "heif", "heifs", "heix", "hevc", "avif", "jp2", "j2k", "j2c", "jpf", "jpx", "jpm", "jxr",
    "wdp", "hdp", "dng", "cr2", "cr3", "nef", "arw", "raf", "rw2", "orf", "pef", "srw", "sr2",
    "kdc", "mrw", "mos", "rwl", "iiq", "3fr", "erf", "mef", "x3f",
];

// WIC/system-codec extensions with their semantic family.
const WIC_CODEC_EXTS: &[(&str, WicCodecFamily)] = &[
    // HEIF/HEIC + AVIF
    ("heic", WicCodecFamily::Heif),
    ("heif", WicCodecFamily::Heif),
    ("heifs", WicCodecFamily::Heif),
    ("heix", WicCodecFamily::Heif),
    ("hevc", WicCodecFamily::Heif),
    ("avif", WicCodecFamily::Avif),
    // JPEG 2000
    ("jp2", WicCodecFamily::Jpeg2000),
    ("j2k", WicCodecFamily::Jpeg2000),
    ("j2c", WicCodecFamily::Jpeg2000),
    ("jpf", WicCodecFamily::Jpeg2000),
    ("jpx", WicCodecFamily::Jpeg2000),
    ("jpm", WicCodecFamily::Jpeg2000),
    // JPEG XR
    ("jxr", WicCodecFamily::JpegXr),
    ("wdp", WicCodecFamily::JpegXr),
    ("hdp", WicCodecFamily::JpegXr),
    // RAW (Microsoft Raw Image Extension, vendor codecs)
    ("dng", WicCodecFamily::Raw),
    ("cr2", WicCodecFamily::Raw),
    ("cr3", WicCodecFamily::Raw),
    ("nef", WicCodecFamily::Raw),
    ("arw", WicCodecFamily::Raw),
    ("raf", WicCodecFamily::Raw),
    ("rw2", WicCodecFamily::Raw),
    ("orf", WicCodecFamily::Raw),
    ("pef", WicCodecFamily::Raw),
    ("srw", WicCodecFamily::Raw),
    ("sr2", WicCodecFamily::Raw),
    ("kdc", WicCodecFamily::Raw),
    ("mrw", WicCodecFamily::Raw),
    ("mos", WicCodecFamily::Raw),
    ("rwl", WicCodecFamily::Raw),
    ("iiq", WicCodecFamily::Raw),
    ("3fr", WicCodecFamily::Raw),
    ("erf", WicCodecFamily::Raw),
    ("mef", WicCodecFamily::Raw),
    ("x3f", WicCodecFamily::Raw),
];

pub fn is_supported_ext(ext: &str) -> bool {
    COMMON_EXTS.iter().any(|e| e == &ext) || is_wic_only_ext(ext)
}

pub fn is_wic_only_ext(ext: &str) -> bool {
    wic_codec_family_for_ext(ext).is_some()
}

pub fn wic_codec_family_for_ext(ext: &str) -> Option<WicCodecFamily> {
    WIC_CODEC_EXTS.iter().find_map(
        |(known, family)| {
            if *known == ext {
                Some(*family)
            } else {
                None
            }
        },
    )
}

pub fn wic_codec_hint_for_ext(ext: &str) -> &'static str {
    match wic_codec_family_for_ext(ext) {
        Some(WicCodecFamily::Heif) => {
            "Install HEIF Image Extensions (and HEVC Video Extensions if needed), then restart."
        }
        Some(WicCodecFamily::Avif) => "Install AV1 Video Extension, then restart.",
        Some(WicCodecFamily::Raw) => "Install Raw Image Extension, then restart.",
        Some(WicCodecFamily::Jpeg2000) => "Install a JPEG 2000 codec, then restart.",
        Some(WicCodecFamily::JpegXr) => "Install a JPEG XR codec, then restart.",
        None => "Install a Windows image codec for this format, then restart.",
    }
}

pub fn is_heif_or_avif_ext(ext: &str) -> bool {
    matches!(
        wic_codec_family_for_ext(ext),
        Some(WicCodecFamily::Heif | WicCodecFamily::Avif)
    )
}

pub fn is_supported_path(path: &Path) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    let ext_str = ext.to_string_lossy().to_lowercase();
    is_supported_ext(ext_str.as_str())
}

pub fn is_wic_only_path(path: &Path) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    let ext_str = ext.to_string_lossy().to_lowercase();
    is_wic_only_ext(ext_str.as_str())
}

pub fn is_wic_scaled_candidate_ext(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "bmp" | "dib" | "gif" | "tif" | "tiff" | "webp" | "ico"
    ) || is_wic_only_ext(ext)
}

pub fn is_wic_scaled_candidate_path(path: &Path) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    let ext_str = ext.to_string_lossy().to_lowercase();
    is_wic_scaled_candidate_ext(ext_str.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_dialog_filter_includes_common_canvas_formats() {
        for ext in ["jpg", "png", "gif", "webp", "ico", "heic", "avif"] {
            assert!(FILE_DIALOG_IMAGE_EXTS.contains(&ext));
            assert!(is_supported_ext(ext));
        }
    }

    #[test]
    fn wic_scaled_candidates_cover_non_jpeg_formats() {
        for ext in ["png", "webp", "tiff", "ico", "heic", "avif"] {
            assert!(is_wic_scaled_candidate_ext(ext));
        }
        for ext in ["jpg", "jpeg", "jfif"] {
            assert!(!is_wic_scaled_candidate_ext(ext));
        }
    }
}
