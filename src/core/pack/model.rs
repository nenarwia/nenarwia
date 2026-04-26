use std::fs::File;

use rusqlite::Connection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageKind {
    Thumb = 0,
    Tile = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageKey {
    pub asset_id: u64,
    pub kind: PageKind,
    pub size: u16,
    pub mip_level: u8,
    pub tile_x: u32,
    pub tile_y: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackKind {
    Library = 0,
    Runtime = 1,
}

impl PackKind {
    pub(super) fn as_i32(self) -> i32 {
        self as i32
    }
}

impl PageKey {
    pub fn thumb(asset_id: u64, size: u16) -> Self {
        Self {
            asset_id,
            kind: PageKind::Thumb,
            size,
            mip_level: 0,
            tile_x: 0,
            tile_y: 0,
        }
    }

    pub fn tile(asset_id: u64, mip_level: u8, tile_x: u32, tile_y: u32) -> Self {
        Self {
            asset_id,
            kind: PageKind::Tile,
            size: 0,
            mip_level,
            tile_x,
            tile_y,
        }
    }
}

pub struct MediaPack {
    pub(super) conn: Connection,
    pub(super) file: File,
    pub(super) chunk_size: u64,
    pub(super) write_offset: u64,
    pub(super) kind: PackKind,
}

pub struct MediaPackReader {
    pub(super) conn: Connection,
    pub(super) file: File,
}

#[derive(Clone, Debug)]
pub struct AssetRecord {
    pub asset_id: u64,
    pub rel_path: String,
    pub size: u64,
    pub modified_ms: u64,
    pub width: u32,
    pub height: u32,
    pub kind: String,
    pub codec: String,
    pub tile_size: u32,
    pub max_mip: u8,
}
