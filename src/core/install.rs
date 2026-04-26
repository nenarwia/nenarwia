use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use image::{ImageBuffer, Rgba};
use lz4_flex::compress_prepend_size;
use rayon::prelude::*;

use crate::core::color;
use crate::core::index::{asset_key_for, modified_to_ms, rel_path, MediaIndex};
use crate::core::loader::disk_cache::{self, TILE_HALO, TILE_PHYSICAL_SIZE, TILE_SIZE};
use crate::core::pack::{AssetRecord, MediaPack, PageKey};

const THUMB_SIZES: [u16; 4] = [32, 64, 128, 256];
const DEFAULT_BASE_PREVIEW: u16 = 512;
const PAGE_CODEC_THUMB: &str = "rgba_lz4";
const PAGE_CODEC_TILE: &str = disk_cache::TILE_CODEC;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstallMode {
    /// Only build base preview + thumb tiers. Tiles/mips will be built lazily at runtime (if enabled).
    Thumbs,
    /// Build only the lowest N mips (coarsest) per asset. Greatly reduces install time/disk.
    Coarse,
    /// Build all mips (full pyramid). Very slow for large libraries.
    Full,
}

pub struct InstallStats {
    pub total: usize,
    pub installed: usize,
    pub skipped: usize,
    pub failed: usize,
}

pub fn run_install(root: &Path) -> Result<InstallStats> {
    let mut index = MediaIndex::load_or_create(root);
    let files = index.refresh(root);

    let mut pack = MediaPack::open(
        &disk_cache::library_root(),
        crate::core::pack::PackKind::Library,
    )
    .context("open media pack")?;

    let mode = install_mode();
    let force = install_force();
    let lowest_mips = install_lowest_mips();
    let base_preview_px = base_preview_size();
    let mut decode_count = 0usize;

    let mut stats = InstallStats {
        total: files.len(),
        installed: 0,
        skipped: 0,
        failed: 0,
    };

    log::info!(
        "Install: {} files (mode={:?}, force={}, base_preview_px={}, lowest_mips={}, tile_size={}, tile_codec={})",
        files.len(),
        mode,
        force,
        base_preview_px,
        lowest_mips,
        TILE_SIZE,
        PAGE_CODEC_TILE,
    );

    for (file_idx, file) in files.into_iter().enumerate() {
        let t0 = std::time::Instant::now();
        let rel = rel_path(root, &file.path);
        let meta = match fs::metadata(&file.path) {
            Ok(m) => m,
            Err(err) => {
                log::warn!("Install: metadata failed for {}: {err:?}", rel);
                stats.failed += 1;
                continue;
            }
        };

        let size = meta.len();
        let modified_ms = modified_to_ms(meta.modified());
        let asset_key = if file.asset_key != 0 {
            file.asset_key
        } else {
            asset_key_for(&rel, size, modified_ms)
        };

        let (mut width, mut height) = (file.width, file.height);
        if width == 0 || height == 0 {
            let (w, h) = color::image_dimensions_any(&file.path).unwrap_or((0, 0));
            width = w;
            height = h;
        }

        let max_mip = max_mip_for_dims(width, height, TILE_SIZE);

        let needs_install = if force {
            true
        } else {
            match pack.asset_record(asset_key)? {
                Some(existing) => {
                    asset_needs_rebuild(&existing, size, modified_ms, width, height, max_mip)
                }
                None => true,
            }
        };

        if !needs_install {
            stats.skipped += 1;
            continue;
        }

        pack.begin_write_batch()?;
        let asset_result: Result<()> = (|| {
            let decoded = color::decode_rgba8_srgb(&file.path)
                .with_context(|| format!("decode image: {}", file.path.display()))?;
            decode_count += 1;
            log::debug!("Install decode [{}] {}", decode_count, file.path.display());
            let rgba = decoded.rgba;

            pack.delete_asset_pages(asset_key)?;

            let base_preview = build_thumbnail(&rgba, base_preview_px);
            let packed = compress_prepend_size(base_preview.as_raw());
            pack.write_page(
                PageKey::thumb(asset_key, base_preview_px),
                &packed,
                PAGE_CODEC_THUMB,
            )?;

            for &size in THUMB_SIZES.iter() {
                if size >= base_preview_px {
                    continue;
                }
                let thumb = downscale_thumbnail(&base_preview, size);
                let packed = compress_prepend_size(thumb.as_raw());
                pack.write_page(PageKey::thumb(asset_key, size), &packed, PAGE_CODEC_THUMB)?;
            }

            // Tile/mip building strategy:
            // - `thumbs`: only base preview + small thumbs (fastest; no tiles).
            // - `coarse`: only the lowest N mips (coarsest) per asset (fast).
            // - `full`: all mips (slow; produces huge disk installs).
            if mode != InstallMode::Thumbs {
                let (lod_start, lod_end) = match mode {
                    InstallMode::Thumbs => (1, 0), // unreachable
                    InstallMode::Full => (0, max_mip),
                    InstallMode::Coarse => {
                        let n = lowest_mips.max(1);
                        let start = max_mip.saturating_add(1).saturating_sub(n).min(max_mip);
                        (start, max_mip)
                    }
                };

                // Build lod images iteratively (start -> end). This avoids repeatedly resizing from
                // the full-res image for every mip.
                let mut lod_img: Option<color::RgbaImage> = None;
                for lod in lod_start..=lod_end {
                    let (lod_w, lod_h) = lod_dims(width, height, lod);

                    // Prepare mip image.
                    if lod == 0 {
                        lod_img = None;
                    } else if lod_img.is_none() || lod == lod_start {
                        // Jump directly from full-res for the first requested mip.
                        lod_img = Some(color::resize_linear_rgba8_exact(&rgba, lod_w, lod_h));
                    } else {
                        // Downscale from previous mip.
                        let prev = lod_img.as_ref().expect("lod_img");
                        lod_img = Some(color::resize_linear_rgba8_exact(prev, lod_w, lod_h));
                    }

                    let lod_ref = lod_img.as_ref().unwrap_or(&rgba);

                    let tiles_x = div_ceil(lod_w, TILE_SIZE);
                    let tiles_y = div_ceil(lod_h, TILE_SIZE);

                    // Encode tiles in parallel; DB/file writes stay sequential.
                    let coords: Vec<(u32, u32)> = (0..tiles_y)
                        .flat_map(|ty| (0..tiles_x).map(move |tx| (tx, ty)))
                        .collect();
                    let encoded: Result<Vec<(PageKey, Vec<u8>)>> = coords
                        .par_iter()
                        .map(|&(tx, ty)| -> Result<(PageKey, Vec<u8>)> {
                            let tile = extract_tile(lod_ref, lod_w, lod_h, tx, ty);
                            let packed = compress_prepend_size(&tile);
                            Ok((PageKey::tile(asset_key, lod, tx, ty), packed))
                        })
                        .collect();

                    for (key, packed) in encoded? {
                        pack.write_page(key, &packed, PAGE_CODEC_TILE)?;
                    }
                }
            }

            let record = AssetRecord {
                asset_id: asset_key,
                rel_path: rel,
                size,
                modified_ms,
                width,
                height,
                kind: "image".to_string(),
                codec: PAGE_CODEC_TILE.to_string(),
                tile_size: TILE_SIZE,
                max_mip,
            };
            pack.upsert_asset(&record)?;

            Ok(())
        })();

        match asset_result {
            Ok(()) => {
                pack.commit_write_batch()?;
                stats.installed += 1;
                log::info!(
                    "Install [{}/{}] ok in {:.2}s: {}",
                    file_idx + 1,
                    stats.total,
                    t0.elapsed().as_secs_f32(),
                    file.path.display()
                );
            }
            Err(err) => {
                pack.rollback_write_batch()?;
                stats.failed += 1;
                log::warn!("Install [{}/{}] failed: {err:?}", file_idx + 1, stats.total);
                continue;
            }
        }
    }

    disk_cache::bump_library_generation();
    log::info!("Install decode count: {}", decode_count);
    Ok(stats)
}

fn asset_needs_rebuild(
    existing: &AssetRecord,
    size: u64,
    modified_ms: u64,
    width: u32,
    height: u32,
    max_mip: u8,
) -> bool {
    existing.size != size
        || existing.modified_ms != modified_ms
        || existing.width != width
        || existing.height != height
        || existing.tile_size != TILE_SIZE
        || existing.max_mip != max_mip
        || existing.codec != PAGE_CODEC_TILE
}

fn install_mode() -> InstallMode {
    if install_full() {
        return InstallMode::Full;
    }
    let val = std::env::var("CANVAS_INSTALL_MODE")
        .unwrap_or_else(|_| "thumbs".to_string())
        .to_lowercase();
    match val.as_str() {
        "thumbs" | "thumb" | "t" | "preview" | "base" => InstallMode::Thumbs,
        "coarse" | "low" => InstallMode::Coarse,
        "full" | "all" => InstallMode::Full,
        // default
        _ => InstallMode::Thumbs,
    }
}

fn install_lowest_mips() -> u8 {
    // Default: build the 2 coarsest mips per image (e.g. for 4k: 500px + 250px),
    // which keeps install time practical while still giving decent zoomed-out quality.
    let default_mips = 2u8;
    let val = std::env::var("CANVAS_INSTALL_LOWEST_MIPS")
        .ok()
        .and_then(|v| v.parse::<u16>().ok());
    match val {
        Some(v) if (1..=255).contains(&v) => v as u8,
        _ => default_mips,
    }
}

fn install_force() -> bool {
    let val = std::env::var("CANVAS_INSTALL_FORCE")
        .unwrap_or_default()
        .to_lowercase();
    matches!(val.as_str(), "1" | "true" | "yes" | "on")
}

fn base_preview_size() -> u16 {
    let val = std::env::var("CANVAS_BASE_PREVIEW")
        .ok()
        .and_then(|v| v.parse::<u16>().ok());
    match val {
        Some(256) => 256,
        Some(512) => 512,
        _ => DEFAULT_BASE_PREVIEW,
    }
}

fn install_full() -> bool {
    let val = std::env::var("CANVAS_INSTALL_FULL")
        .unwrap_or_default()
        .to_lowercase();
    matches!(val.as_str(), "1" | "true" | "yes" | "on")
}

fn build_thumbnail(img: &color::RgbaImage, size: u16) -> color::RgbaImage {
    let size_u32 = size as u32;
    let scaled = color::resize_linear_rgba8_fit(img, size_u32, size_u32);
    let mut buffer = ImageBuffer::from_pixel(size_u32, size_u32, Rgba([0, 0, 0, 0]));

    let dx = (((size as i32) - scaled.width() as i32) / 2).max(0) as i64;
    let dy = (((size as i32) - scaled.height() as i32) / 2).max(0) as i64;
    image::imageops::overlay(&mut buffer, &scaled, dx, dy);

    buffer
}

fn downscale_thumbnail(img: &color::RgbaImage, size: u16) -> color::RgbaImage {
    let size_u32 = size as u32;
    color::resize_linear_rgba8_exact(img, size_u32, size_u32)
}

fn extract_tile(
    lod_img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    lod_w: u32,
    lod_h: u32,
    tx: u32,
    ty: u32,
) -> Vec<u8> {
    let physical = TILE_PHYSICAL_SIZE as usize;
    let logical = TILE_SIZE as i64;
    let halo = TILE_HALO as i64;
    let mut out = vec![0u8; physical.saturating_mul(physical).saturating_mul(4)];

    if lod_w == 0 || lod_h == 0 {
        return out;
    }

    let src = lod_img.as_raw();
    let src_stride = (lod_w as usize).saturating_mul(4);
    let dst_stride = physical.saturating_mul(4);
    let base_x = (tx as i64).saturating_mul(logical);
    let base_y = (ty as i64).saturating_mul(logical);
    let max_x = lod_w.saturating_sub(1) as i64;
    let max_y = lod_h.saturating_sub(1) as i64;

    for oy in 0..physical {
        let sy = (base_y + oy as i64 - halo).clamp(0, max_y) as usize;
        let src_row = sy.saturating_mul(src_stride);
        let dst_row = oy.saturating_mul(dst_stride);
        for ox in 0..physical {
            let sx = (base_x + ox as i64 - halo).clamp(0, max_x) as usize;
            let src_idx = src_row + sx.saturating_mul(4);
            let dst_idx = dst_row + ox.saturating_mul(4);
            out[dst_idx..dst_idx + 4].copy_from_slice(&src[src_idx..src_idx + 4]);
        }
    }

    out
}

fn lod_dims(orig_w: u32, orig_h: u32, lod: u8) -> (u32, u32) {
    let shift = (lod as u32).min(31);
    let scale = 1u32 << shift;
    (
        div_ceil(orig_w, scale).max(1),
        div_ceil(orig_h, scale).max(1),
    )
}

fn max_mip_for_dims(orig_w: u32, orig_h: u32, tile: u32) -> u8 {
    let mut size = orig_w.max(orig_h).max(1);
    let mut lod = 0u8;
    while size > tile {
        size = div_ceil(size, 2);
        lod = lod.saturating_add(1);
    }
    lod
}

fn div_ceil(a: u32, b: u32) -> u32 {
    if b == 0 {
        return 0;
    }
    a.div_ceil(b)
}
