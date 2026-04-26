use std::path::{Path, PathBuf};

use crate::core::wallpaper::ensure_saved_wallpaper_preview_blur;

pub(super) fn decode_default_wallpaper_source() -> Result<crate::core::color::DecodedRgba, String> {
    let rgba = image::load_from_memory_with_format(
        super::DEFAULT_WALLPAPER_BYTES,
        image::ImageFormat::Jpeg,
    )
    .map_err(|err| format!("Failed to decode bundled default wallpaper: {err:#}"))?
    .to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return Err("Bundled default wallpaper has invalid dimensions.".to_string());
    }
    Ok(crate::core::color::DecodedRgba {
        rgba,
        width,
        height,
    })
}

pub(super) fn decode_wallpaper_preview_source(
    path: PathBuf,
    max_dim: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    decode_wallpaper_thumbnail(&path, max_dim)
}

pub(super) fn decode_wallpaper_thumbnail(
    path: &Path,
    max_dim: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    let decoded = crate::core::color::decode_rgba8_srgb_thumbnail(path, max_dim)
        .map_err(|err| format!("Failed to open image '{}': {err:#}", path.display()))?;
    if decoded.width == 0 || decoded.height == 0 {
        return Err(format!(
            "Image '{}' has invalid dimensions.",
            path.display()
        ));
    }
    let mut rgba = decoded.rgba;
    if decoded.width.max(decoded.height) > max_dim.max(1) {
        rgba = crate::core::color::resize_linear_rgba8_fit(&rgba, max_dim.max(1), max_dim.max(1));
    }
    let (width, height) = rgba.dimensions();
    Ok((rgba.into_raw(), width, height))
}

pub(super) fn decode_saved_wallpaper_preview_blur(
    source_path: &Path,
    blur_path: &Path,
    max_dim: u32,
    preview_blur_max_dim: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    ensure_saved_wallpaper_preview_blur(source_path, blur_path, preview_blur_max_dim).map_err(
        |err| {
            format!(
                "Failed to build saved wallpaper preview blur '{}': {err:#}",
                blur_path.display()
            )
        },
    )?;
    decode_wallpaper_thumbnail(blur_path, max_dim)
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_default_wallpaper_decodes() {
        let decoded = super::decode_default_wallpaper_source()
            .expect("bundled default wallpaper should decode");

        assert!(decoded.width > 0);
        assert!(decoded.height > 0);
        assert_eq!(
            decoded.rgba.as_raw().len(),
            decoded.width as usize * decoded.height as usize * 4
        );
    }
}
