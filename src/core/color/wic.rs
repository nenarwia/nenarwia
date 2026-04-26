use std::path::Path;

use anyhow::Result;

use crate::core::formats;

use super::DecodedRgba;

#[cfg(target_os = "windows")]
mod windows_impl;

pub(super) fn dimensions(path: &Path) -> Result<Option<(u32, u32)>> {
    if !tiles_enabled() {
        return Ok(None);
    }
    if !formats::is_wic_only_path(path) {
        return Ok(None);
    }
    if let Ok(Some((w, h))) = winrt_dimensions(path) {
        return Ok(Some((w, h)));
    }
    dimensions_inner(path)
}

pub(super) fn decode_full(path: &Path) -> Result<Option<DecodedRgba>> {
    if !tiles_enabled() {
        return Ok(None);
    }
    if !formats::is_wic_only_path(path) {
        return Ok(None);
    }
    if let Ok(Some(decoded)) = super::winrt::decode_full(path) {
        return Ok(Some(decoded));
    }
    let wic = decode_full_inner(path);
    match wic {
        Ok(Some(img)) => Ok(Some(img)),
        _ => {
            super::register_missing_codec_for_path(path);
            Ok(None)
        }
    }
}

pub(super) fn decode_scaled(path: &Path, width: u32, height: u32) -> Result<Option<DecodedRgba>> {
    if !tiles_enabled() {
        return Ok(None);
    }
    if !formats::is_wic_scaled_candidate_path(path) {
        return Ok(None);
    }
    if let Ok(Some(decoded)) = super::winrt::decode_scaled(path, width, height) {
        return Ok(Some(decoded));
    }
    let wic = decode_scaled_inner(path, width, height);
    match wic {
        Ok(Some(img)) => Ok(Some(img)),
        _ => {
            if formats::is_wic_only_path(path) {
                super::register_missing_codec_for_path(path);
            }
            Ok(None)
        }
    }
}

pub(super) fn decode_thumbnail(path: &Path, max_dim: u32) -> Result<Option<DecodedRgba>> {
    if !thumbs_enabled() {
        return Ok(None);
    }
    if max_dim == 0 {
        return Ok(None);
    }
    if !formats::is_wic_scaled_candidate_path(path) {
        return Ok(None);
    }
    if let Ok(Some(decoded)) = super::winrt::decode_thumbnail(path, max_dim) {
        return Ok(Some(decoded));
    }
    let wic = decode_thumbnail_inner(path, max_dim);
    match wic {
        Ok(Some(img)) => Ok(Some(img)),
        _ => {
            if formats::is_wic_only_path(path) {
                super::register_missing_codec_for_path(path);
            }
            Ok(None)
        }
    }
}

pub(super) fn probe_codec_once(path: &Path) {
    let _ = path;
    #[cfg(target_os = "windows")]
    {
        if !tiles_enabled() {
            return;
        }
        let Some(ext) = path.extension().map(|e| e.to_string_lossy().to_lowercase()) else {
            return;
        };
        if !formats::is_wic_only_ext(ext.as_str()) {
            return;
        }
        if !windows_impl::probe_ext_once(ext.as_str()) {
            return;
        }
        if let Ok(Some(_)) = winrt_dimensions(path) {
            super::clear_missing_codec_for_path(path);
            return;
        }
        if let Ok(Some(_)) = dimensions(path) {
            super::clear_missing_codec_for_path(path);
            return;
        }
        super::register_missing_codec_for_path(path);
    }
}

fn thumbs_enabled() -> bool {
    let val = std::env::var("CANVAS_WIC_THUMBS")
        .unwrap_or_else(|_| "1".to_string())
        .to_lowercase();
    !matches!(val.as_str(), "0" | "false" | "no" | "off")
}

fn tiles_enabled() -> bool {
    let val = std::env::var("CANVAS_WIC_TILES")
        .unwrap_or_else(|_| "1".to_string())
        .to_lowercase();
    !matches!(val.as_str(), "0" | "false" | "no" | "off")
}

fn winrt_dimensions(path: &Path) -> Result<Option<(u32, u32)>> {
    #[cfg(target_os = "windows")]
    {
        match super::winrt::dimensions(path) {
            Ok(v) => Ok(v),
            Err(err) => {
                log::warn!("WinRT: dimensions failed for {:?}: {err:?}", path);
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

#[cfg(target_os = "windows")]
fn dimensions_inner(path: &Path) -> Result<Option<(u32, u32)>> {
    windows_impl::dimensions_inner(path)
}

#[cfg(not(target_os = "windows"))]
fn dimensions_inner(_path: &Path) -> Result<Option<(u32, u32)>> {
    Ok(None)
}

#[cfg(target_os = "windows")]
fn decode_full_inner(path: &Path) -> Result<Option<DecodedRgba>> {
    windows_impl::decode_full_inner(path)
}

#[cfg(not(target_os = "windows"))]
fn decode_full_inner(_path: &Path) -> Result<Option<DecodedRgba>> {
    Ok(None)
}

#[cfg(target_os = "windows")]
fn decode_thumbnail_inner(path: &Path, max_dim: u32) -> Result<Option<DecodedRgba>> {
    windows_impl::decode_thumbnail_inner(path, max_dim)
}

#[cfg(not(target_os = "windows"))]
fn decode_thumbnail_inner(_path: &Path, _max_dim: u32) -> Result<Option<DecodedRgba>> {
    Ok(None)
}

#[cfg(target_os = "windows")]
fn decode_scaled_inner(path: &Path, width: u32, height: u32) -> Result<Option<DecodedRgba>> {
    windows_impl::decode_scaled_inner(path, width, height)
}

#[cfg(not(target_os = "windows"))]
fn decode_scaled_inner(_path: &Path, _width: u32, _height: u32) -> Result<Option<DecodedRgba>> {
    Ok(None)
}
