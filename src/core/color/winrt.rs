use std::path::Path;

use anyhow::Result;
use image::{ImageBuffer, Rgba};

use super::DecodedRgba;

pub(super) fn dimensions(path: &Path) -> Result<Option<(u32, u32)>> {
    #[cfg(target_os = "windows")]
    {
        dimensions_inner(path)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Ok(None)
    }
}

pub(super) fn decode_full(path: &Path) -> Result<Option<DecodedRgba>> {
    #[cfg(target_os = "windows")]
    {
        match decode_rgba8(path, None) {
            Ok(Some(res)) => Ok(Some(res)),
            Ok(None) => {
                log::warn!("WinRT: decode returned no data for {:?}", path);
                Ok(None)
            }
            Err(err) => {
                log::warn!("WinRT: decode failed for {:?}: {err:?}", path);
                Ok(None)
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Ok(None)
    }
}

pub(super) fn decode_scaled(path: &Path, width: u32, height: u32) -> Result<Option<DecodedRgba>> {
    if width == 0 || height == 0 {
        return Ok(None);
    }
    #[cfg(target_os = "windows")]
    {
        match decode_rgba8(path, Some((width, height))) {
            Ok(Some(res)) => Ok(Some(res)),
            Ok(None) => {
                log::warn!("WinRT: decode returned no data for {:?}", path);
                Ok(None)
            }
            Err(err) => {
                log::warn!("WinRT: decode failed for {:?}: {err:?}", path);
                Ok(None)
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (path, width, height);
        Ok(None)
    }
}

pub(super) fn decode_thumbnail(path: &Path, max_dim: u32) -> Result<Option<DecodedRgba>> {
    if max_dim == 0 {
        return Ok(None);
    }
    let Some((w, h)) = dimensions(path)? else {
        return Ok(None);
    };
    let scale = (max_dim as f32 / w as f32)
        .min(max_dim as f32 / h as f32)
        .min(1.0);
    let out_w = (w as f32 * scale).round().max(1.0) as u32;
    let out_h = (h as f32 * scale).round().max(1.0) as u32;
    decode_scaled(path, out_w, out_h)
}

fn unpremultiply_rgba8_in_place(buf: &mut [u8]) {
    for px in buf.chunks_mut(4) {
        let a = px[3] as u32;
        if a == 0 || a == 255 {
            continue;
        }
        let r = px[0] as u32;
        let g = px[1] as u32;
        let b = px[2] as u32;
        let r = ((r * 255 + a / 2) / a).min(255) as u8;
        let g = ((g * 255 + a / 2) / a).min(255) as u8;
        let b = ((b * 255 + a / 2) / a).min(255) as u8;
        px[0] = r;
        px[1] = g;
        px[2] = b;
    }
}

#[cfg(target_os = "windows")]
fn abs_path(path: &Path) -> std::path::PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    }
}

#[cfg(target_os = "windows")]
fn init() -> Result<()> {
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::WinRT::{
        RoInitialize, RO_INIT_MULTITHREADED, RO_INIT_SINGLETHREADED,
    };

    let res = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    match res {
        Ok(_) => Ok(()),
        Err(err) => {
            if err.code() == RPC_E_CHANGED_MODE {
                unsafe { RoInitialize(RO_INIT_SINGLETHREADED) }.map_err(|e| e.into())
            } else {
                Err(err.into())
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn create_decoder(
    stream: &windows::Storage::Streams::IRandomAccessStream,
    path: &Path,
) -> Result<windows::Graphics::Imaging::BitmapDecoder> {
    use windows::Graphics::Imaging::BitmapDecoder;

    let decode_default = BitmapDecoder::CreateAsync(stream).and_then(|op| op.get());
    if let Ok(dec) = decode_default {
        return Ok(dec);
    }
    let mut last_err = decode_default
        .err()
        .unwrap_or_else(windows::core::Error::from_win32);

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if crate::core::formats::is_heif_or_avif_ext(ext.as_str()) {
        if let Ok(id) = BitmapDecoder::HeifDecoderId() {
            match BitmapDecoder::CreateWithIdAsync(id, stream) {
                Ok(op) => match op.get() {
                    Ok(dec) => return Ok(dec),
                    Err(err) => last_err = err,
                },
                Err(err) => last_err = err,
            }
        }
    }

    Err(last_err.into())
}

#[cfg(target_os = "windows")]
fn dimensions_inner(path: &Path) -> Result<Option<(u32, u32)>> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{HSTRING, PCWSTR};
    use windows::Storage::FileAccessMode;
    use windows::Storage::StorageFile;
    use windows::Storage::Streams::IRandomAccessStream;
    use windows::Win32::System::WinRT::CreateRandomAccessStreamOnFile;

    init()?;

    let full_path = abs_path(path);
    let wide: Vec<u16> = OsStr::new(&full_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let stream: IRandomAccessStream =
        match unsafe { CreateRandomAccessStreamOnFile(PCWSTR(wide.as_ptr()), 0) } {
            Ok(s) => s,
            Err(_) => {
                let hpath = HSTRING::from(full_path.to_string_lossy().as_ref());
                let file = StorageFile::GetFileFromPathAsync(&hpath)?.get()?;
                file.OpenAsync(FileAccessMode::Read)?.get()?
            }
        };

    let decoder = create_decoder(&stream, path)?;
    let frame = decoder.GetFrameAsync(0)?.get()?;
    let w = frame.OrientedPixelWidth().or_else(|_| frame.PixelWidth())?;
    let h = frame
        .OrientedPixelHeight()
        .or_else(|_| frame.PixelHeight())?;
    if w == 0 || h == 0 {
        return Ok(None);
    }
    Ok(Some((w, h)))
}

#[cfg(target_os = "windows")]
fn decode_rgba8(path: &Path, scaled: Option<(u32, u32)>) -> Result<Option<DecodedRgba>> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{HSTRING, PCWSTR};
    use windows::Graphics::Imaging::{
        BitmapAlphaMode, BitmapPixelFormat, BitmapTransform, ColorManagementMode,
        ExifOrientationMode,
    };
    use windows::Storage::FileAccessMode;
    use windows::Storage::StorageFile;
    use windows::Storage::Streams::IRandomAccessStream;
    use windows::Win32::System::WinRT::CreateRandomAccessStreamOnFile;

    init()?;

    let full_path = abs_path(path);
    let wide: Vec<u16> = OsStr::new(&full_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let stream: IRandomAccessStream =
        match unsafe { CreateRandomAccessStreamOnFile(PCWSTR(wide.as_ptr()), 0) } {
            Ok(s) => s,
            Err(_) => {
                let hpath = HSTRING::from(full_path.to_string_lossy().as_ref());
                let file = StorageFile::GetFileFromPathAsync(&hpath)?.get()?;
                file.OpenAsync(FileAccessMode::Read)?.get()?
            }
        };

    let decoder = create_decoder(&stream, path)?;
    let frame = decoder.GetFrameAsync(0)?.get()?;
    let transform = BitmapTransform::new()?;
    if let Some((w, h)) = scaled {
        transform.SetScaledWidth(w)?;
        transform.SetScaledHeight(h)?;
    }

    let mut premultiplied = false;
    let pixel = match frame.GetPixelDataTransformedAsync(
        BitmapPixelFormat::Bgra8,
        BitmapAlphaMode::Straight,
        &transform,
        ExifOrientationMode::RespectExifOrientation,
        ColorManagementMode::ColorManageToSRgb,
    ) {
        Ok(op) => match op.get() {
            Ok(px) => px,
            Err(_) => {
                premultiplied = true;
                frame
                    .GetPixelDataTransformedAsync(
                        BitmapPixelFormat::Bgra8,
                        BitmapAlphaMode::Premultiplied,
                        &transform,
                        ExifOrientationMode::RespectExifOrientation,
                        ColorManagementMode::ColorManageToSRgb,
                    )?
                    .get()?
            }
        },
        Err(_) => {
            premultiplied = true;
            frame
                .GetPixelDataTransformedAsync(
                    BitmapPixelFormat::Bgra8,
                    BitmapAlphaMode::Premultiplied,
                    &transform,
                    ExifOrientationMode::RespectExifOrientation,
                    ColorManagementMode::ColorManageToSRgb,
                )?
                .get()?
        }
    };

    let data = pixel.DetachPixelData()?;
    let mut bytes = data.as_slice().to_vec();
    for px in bytes.chunks_mut(4) {
        px.swap(0, 2);
    }
    if premultiplied {
        unpremultiply_rgba8_in_place(&mut bytes);
    }

    let (out_w, out_h) = if let Some((w, h)) = scaled {
        (w, h)
    } else {
        let w = frame.OrientedPixelWidth().or_else(|_| frame.PixelWidth())?;
        let h = frame
            .OrientedPixelHeight()
            .or_else(|_| frame.PixelHeight())?;
        (w, h)
    };
    if out_w == 0 || out_h == 0 {
        log::warn!("WinRT: invalid image size for {:?}", path);
        return Ok(None);
    }

    let Some(img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(out_w, out_h, bytes) else {
        log::warn!("WinRT: pixel buffer size mismatch for {:?}", path);
        return Ok(None);
    };
    super::clear_missing_codec_for_path(path);
    Ok(Some(DecodedRgba {
        rgba: img,
        width: out_w,
        height: out_h,
    }))
}
