// FILE: src/render/cull.wgsl

struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct ObjectData {
    data: vec4<f32>,       // x, y, w, h
    color: vec4<f32>,
    uv_region: vec4<f32>,  // Atlas region
    params: vec4<f32>,     // desired LOD params
    params2: vec4<f32>,    // coarse fallback params
    sample_flags: vec4<f32>,
    fit_rect: vec4<f32>,
};

@group(0) @binding(1)
var<storage, read> input_objects: array<ObjectData>;

@group(0) @binding(2)
var<storage, read_write> output_instances: array<ObjectData>;

struct DrawIndirectArgs {
    vertex_count: u32,
    instance_count: atomic<u32>,
    first_vertex: u32,
    first_instance: u32,
};

@group(0) @binding(3)
var<storage, read_write> draw_cmd: DrawIndirectArgs;

struct InteractionUniform {
    hovered_instance_idx: u32,
    loaded_instance_idx: u32,
    _pad1: u32,
    _pad2: u32,
};
@group(0) @binding(4)
var<uniform> interaction: InteractionUniform;


fn is_visible(pos: vec2<f32>, size: vec2<f32>) -> bool {
    let world_pos = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
    let clip_pos = camera.view_proj * world_pos;
    let w_val = 1.0 + (max(size.x, size.y) * 2.0); 
    return clip_pos.x > -w_val && clip_pos.x < w_val &&
           clip_pos.y > -w_val && clip_pos.y < w_val;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let total_objects = arrayLength(&input_objects);
    
    if (index >= total_objects) { return; }

    var obj = input_objects[index];
    obj.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);

    // 1. CULLING
    if (is_visible(vec2<f32>(obj.data.x, obj.data.y), vec2<f32>(obj.data.z, obj.data.w))) {
        
        // Interaction highlight
        if (index == interaction.hovered_instance_idx) {
            // Use alpha as a lightweight hover flag (handled in fragment shader).
            obj.color = vec4<f32>(1.0, 1.0, 1.0, 2.0);
        }

        // 2. OUTPUT
        let out_index = atomicAdd(&draw_cmd.instance_count, 1u);
        output_instances[out_index] = obj;
    }
}
