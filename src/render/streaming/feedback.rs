use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use crate::render::context::state::RenderContext;

mod apply;
mod encode;
mod readback;
mod setup;

pub const FEEDBACK_TEX_W: u32 = 256;
pub const FEEDBACK_TEX_H: u32 = 144;
const FEEDBACK_BLOCK: u32 = 16;
const FEEDBACK_BUF_WORKGROUP: u32 = 64;
const MAX_TILES: usize = 8192;
const HEADER_BYTES: u64 = 16;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct FeedbackInstance {
    pub asset_key_lo: u32,
    pub asset_key_hi: u32,
    pub desired_lod: u32,
    pub _pad0: u32,
    pub desired_tiles: [f32; 2],
    pub _pad1: [f32; 2],
}

impl FeedbackInstance {
    pub fn new(asset_key: u64, desired_lod: u32, tiles_x: f32, tiles_y: f32) -> Self {
        Self {
            asset_key_lo: asset_key as u32,
            asset_key_hi: (asset_key >> 32) as u32,
            desired_lod,
            _pad0: 0,
            desired_tiles: [tiles_x, tiles_y],
            _pad1: [0.0, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GpuFeedbackTile {
    asset_key_lo: u32,
    asset_key_hi: u32,
    tile_x: u32,
    tile_y: u32,
    lod: u32,
}

struct ReadbackSlot {
    buffer: wgpu::Buffer,
    ready: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    pending: bool,
    map_requested: bool,
    submitted_frame: u64,
}

pub struct FeedbackResult {
    tiles: Vec<GpuFeedbackTile>,
    overflow: bool,
    latency_frames: u32,
}

pub struct FeedbackSummary {
    pub unique_tiles: u32,
    pub overflow: bool,
    pub latency_frames: u32,
}

pub struct FeedbackEncodeInput<'a> {
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub camera_bg: &'a wgpu::BindGroup,
    pub instance_buffer: &'a wgpu::Buffer,
    pub instance_count: u32,
    pub feedback_instance_bg: &'a wgpu::BindGroup,
    pub feedback_collect_buf_bg: &'a wgpu::BindGroup,
    pub frame_index: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeedbackMode {
    Rt,
    Buf,
}

struct RtResources {
    _feedback_tex: wgpu::Texture,
    feedback_view: wgpu::TextureView,
    _feedback_valid_tex: wgpu::Texture,
    feedback_valid_view: wgpu::TextureView,
    feedback_pipeline: wgpu::RenderPipeline,
    collect_rt_pipeline: wgpu::ComputePipeline,
    collect_rt_bind_group: wgpu::BindGroup,
}

pub struct GpuFeedback {
    mode: FeedbackMode,
    header: wgpu::Buffer,
    output: wgpu::Buffer,
    readbacks: Vec<ReadbackSlot>,

    feedback_instance_layout: wgpu::BindGroupLayout,
    collect_buf_layout: wgpu::BindGroupLayout,
    collect_buf_pipeline: wgpu::ComputePipeline,

    rt: Option<RtResources>,
}

pub fn apply_feedback_results(
    ctx: &mut RenderContext,
    results: Vec<FeedbackResult>,
) -> FeedbackSummary {
    apply::apply_feedback_results_impl(ctx, results)
}
