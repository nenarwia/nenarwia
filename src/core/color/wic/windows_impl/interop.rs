use std::path::Path;

use anyhow::Result;

use super::errors::log_decode_error;

pub(super) fn create_factory(
    path: &Path,
) -> Option<windows::Win32::Graphics::Imaging::IWICImagingFactory> {
    use windows::Win32::Graphics::Imaging::{
        CLSID_WICImagingFactory, CLSID_WICImagingFactory2, IWICImagingFactory,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};

    let factory2: Result<IWICImagingFactory, _> =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory2, None, CLSCTX_INPROC_SERVER) };
    match factory2 {
        Ok(f) => Some(f),
        Err(_) => {
            let factory1: Result<IWICImagingFactory, _> =
                unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) };
            match factory1 {
                Ok(f) => Some(f),
                Err(err) => {
                    log_decode_error(path, &err);
                    None
                }
            }
        }
    }
}

pub(super) fn create_decoder(
    factory: &windows::Win32::Graphics::Imaging::IWICImagingFactory,
    path: &Path,
) -> Result<windows::Win32::Graphics::Imaging::IWICBitmapDecoder> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::GENERIC_ACCESS_RIGHTS;
    use windows::Win32::Graphics::Imaging::{
        WICDecodeMetadataCacheOnDemand, WICDecodeMetadataCacheOnLoad,
    };

    let wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let on_demand = unsafe {
        factory.CreateDecoderFromFilename(
            PCWSTR(wide.as_ptr()),
            None,
            GENERIC_ACCESS_RIGHTS(0),
            WICDecodeMetadataCacheOnDemand,
        )
    };
    if let Ok(decoder) = on_demand {
        return Ok(decoder);
    }

    let on_load = unsafe {
        factory.CreateDecoderFromFilename(
            PCWSTR(wide.as_ptr()),
            None,
            GENERIC_ACCESS_RIGHTS(0),
            WICDecodeMetadataCacheOnLoad,
        )
    };
    match on_load {
        Ok(decoder) => Ok(decoder),
        Err(err) => {
            log_decode_error(path, &err);
            Err(err.into())
        }
    }
}
