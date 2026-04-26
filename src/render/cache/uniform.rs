use bytemuck::{Pod, Zeroable};

/// Small uniform block for shader constants related to the physical tile cache.
///
/// We keep it minimal and 16-byte aligned.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CacheUniform {
    pub cache_cols: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
}

impl CacheUniform {
    pub fn new(cache_cols: u32) -> Self {
        Self {
            cache_cols: cache_cols.max(1),
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }
}
