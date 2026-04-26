mod decode;
mod errors;
mod icc;
mod interop;
mod once;

use std::path::Path;

use anyhow::Result;

use crate::core::color::DecodedRgba;

pub(super) fn probe_ext_once(ext: &str) -> bool {
    once::probe_ext_once(ext)
}

pub(super) fn dimensions_inner(path: &Path) -> Result<Option<(u32, u32)>> {
    decode::dimensions_inner(path)
}

pub(super) fn decode_full_inner(path: &Path) -> Result<Option<DecodedRgba>> {
    decode::decode_full_inner(path)
}

pub(super) fn decode_thumbnail_inner(path: &Path, max_dim: u32) -> Result<Option<DecodedRgba>> {
    decode::decode_thumbnail_inner(path, max_dim)
}

pub(super) fn decode_scaled_inner(
    path: &Path,
    width: u32,
    height: u32,
) -> Result<Option<DecodedRgba>> {
    decode::decode_scaled_inner(path, width, height)
}
