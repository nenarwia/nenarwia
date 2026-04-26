use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThumbDecodeMode {
    Draft,
    Medium,
    Full,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoadRequest {
    Thumbnail {
        path: PathBuf,
        id: u64,
        asset_key: u64,
        size: u16,
        decode_mode: ThumbDecodeMode,
        epoch: u64,
        orig_w: u32,
        orig_h: u32,
    },
    CanvasMediaSlot {
        path: PathBuf,
        id: u64,
        asset_key: u64,
        lod: u8,
        tile_x: u32,
        tile_y: u32,
        epoch: u64,
        orig_w: u32,
        orig_h: u32,
    },
}

impl LoadRequest {
    pub fn epoch(&self) -> u64 {
        match self {
            LoadRequest::Thumbnail { epoch, .. } => *epoch,
            LoadRequest::CanvasMediaSlot { epoch, .. } => *epoch,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImagePayload {
    Rgba8(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedImage {
    pub id: u64,
    pub asset_key: u64,
    pub epoch: u64,
    pub payload: ImagePayload,
    pub width: u32,
    pub height: u32,
    pub is_detail: bool,
    pub tile_x: u32,
    pub tile_y: u32,
    pub lod: u8,
    pub missing: bool,
    pub orig_w: u32,
    pub orig_h: u32,
}
