pub const PT_TEXTURE_SIZE: u32 = 2048;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PtRegion {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}
