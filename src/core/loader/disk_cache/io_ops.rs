use std::io;

use lz4_flex::{compress_prepend_size, decompress_size_prepended};

use crate::core::metrics;
use crate::core::pack::PageKey;

use super::maintenance::enforce_runtime_budget;
use super::shared;
use super::{THUMB_CODEC, TILE_CODEC, TILE_PHYSICAL_SIZE};

fn decode_rgba_lz4(bytes: &[u8], expected_len: usize) -> io::Result<Vec<u8>> {
    let raw = decompress_size_prepended(bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("lz4: {e:?}")))?;
    if raw.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("bad rgba len: {} != {}", raw.len(), expected_len),
        ));
    }
    Ok(raw)
}

pub fn read_thumb_rgba_lz4(asset_key: u64, size: u16) -> io::Result<Vec<u8>> {
    let expected = size as usize * size as usize * 4;
    let key = PageKey::thumb(asset_key, size);
    if let Ok(packed) = shared::with_runtime_reader(|pack| pack.read_page(key)) {
        metrics::record_io_read(packed.len() as u64);
        metrics::record_page_read(1);
        let _ =
            shared::with_runtime_writer(|pack| pack.touch_page(key, shared::next_usage_epoch()));
        return decode_rgba_lz4(&packed, expected);
    }

    let packed = shared::with_library_reader(|pack| pack.read_page(key))?;
    metrics::record_io_read(packed.len() as u64);
    metrics::record_page_read(1);
    decode_rgba_lz4(&packed, expected)
}

pub fn read_tile_rgba_lz4(
    asset_key: u64,
    lod: u8,
    tile_x: u32,
    tile_y: u32,
) -> io::Result<Vec<u8>> {
    let expected = TILE_PHYSICAL_SIZE as usize * TILE_PHYSICAL_SIZE as usize * 4;
    let key = PageKey::tile(asset_key, lod, tile_x, tile_y);
    if let Ok(packed) = shared::with_runtime_reader(|pack| pack.read_page(key)) {
        metrics::record_io_read(packed.len() as u64);
        metrics::record_page_read(1);
        let _ =
            shared::with_runtime_writer(|pack| pack.touch_page(key, shared::next_usage_epoch()));
        return decode_rgba_lz4(&packed, expected);
    }

    let packed = shared::with_library_reader(|pack| pack.read_page(key))?;
    metrics::record_io_read(packed.len() as u64);
    metrics::record_page_read(1);
    decode_rgba_lz4(&packed, expected)
}

pub fn read_canvas_media_slot_rgba_lz4(
    asset_key: u64,
    lod: u8,
    tile_x: u32,
    tile_y: u32,
) -> io::Result<Vec<u8>> {
    read_tile_rgba_lz4(asset_key, lod, tile_x, tile_y)
}

pub fn write_thumb_rgba_lz4(asset_key: u64, size: u16, rgba: &[u8]) -> io::Result<()> {
    let packed = compress_prepend_size(rgba);
    let key = PageKey::thumb(asset_key, size);
    shared::with_runtime_writer(|pack| pack.write_page(key, &packed, THUMB_CODEC))?;
    enforce_runtime_budget()
}

pub fn write_tile_rgba_lz4(
    asset_key: u64,
    lod: u8,
    tile_x: u32,
    tile_y: u32,
    rgba: &[u8],
) -> io::Result<()> {
    let packed = compress_prepend_size(rgba);
    let key = PageKey::tile(asset_key, lod, tile_x, tile_y);
    shared::with_runtime_writer(|pack| pack.write_page(key, &packed, TILE_CODEC))?;
    enforce_runtime_budget()
}

pub fn write_canvas_media_slot_rgba_lz4(
    asset_key: u64,
    lod: u8,
    tile_x: u32,
    tile_y: u32,
    rgba: &[u8],
) -> io::Result<()> {
    write_tile_rgba_lz4(asset_key, lod, tile_x, tile_y, rgba)
}
