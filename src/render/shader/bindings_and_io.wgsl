struct CameraUniform { view_proj: mat4x4<f32>, };
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct CacheUniform { cache_cols: u32, _pad0: u32, _pad1: u32, _pad2: u32, };
const LOGICAL_TILE_PX: f32 = 256.0;
const TILE_HALO_PX: f32 = 2.0;
const PHYSICAL_TILE_PX: f32 = LOGICAL_TILE_PX + TILE_HALO_PX * 2.0;
const TILE_EDGE_EPS: f32 = 1e-5;

@group(1) @binding(0) var t_atlas32: texture_2d<f32>;
@group(1) @binding(1) var t_atlas64: texture_2d<f32>;
@group(1) @binding(2) var t_atlas128: texture_2d<f32>;
@group(1) @binding(3) var t_atlas256: texture_2d<f32>;
@group(1) @binding(4) var t_atlas512: texture_2d<f32>;
@group(1) @binding(5) var s_linear: sampler;
@group(1) @binding(6) var s_nearest: sampler;
@group(1) @binding(7) var t_detail: texture_2d<f32>;
@group(1) @binding(8) var t_page_table: texture_2d<u32>;
@group(1) @binding(9) var<uniform> cache: CacheUniform;

struct VertexInput { @builtin(vertex_index) vertex_index: u32, };

struct InstanceInput {
    @location(5) data: vec4<f32>,
    @location(6) color: vec4<f32>,
    @location(7) uv_region: vec4<f32>,

    // Desired (screen-space driven) LOD params
    @location(8) params: vec4<f32>,
    // Coarse fallback LOD params (maps/AAA progressive refinement)
    @location(9) params2: vec4<f32>,
    @location(10) sample_flags: vec4<f32>,
    @location(11) fit_rect: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,

    // Desired LOD
    @location(2) pt_pos: vec2<f32>,
    // Exact tile count (may be fractional): lod_w/256, lod_h/256
    @location(3) tile_count: vec2<f32>,

    @location(4) atlas_region: vec4<f32>,

    // Coarse fallback LOD
    @location(5) coarse_pt_pos: vec2<f32>,
    @location(6) coarse_tile_count: vec2<f32>,
    @location(7) sample_flags: vec4<f32>,
    @location(8) local_uv: vec2<f32>,
    @location(9) fit_rect: vec4<f32>,
};
