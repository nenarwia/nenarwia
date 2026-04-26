struct CameraUniform { view_proj: mat4x4<f32>, };
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
};

struct InstanceInput {
    @location(5) data: vec4<f32>,
    @location(6) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, 0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, -0.5),
        vec2<f32>(0.5, -0.5),
        vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, 0.5)
    );
    let base_pos = pos[in.vertex_index];
    let world_pos = vec4<f32>(
        (base_pos.x * instance.data.z) + instance.data.x,
        (base_pos.y * instance.data.w) + instance.data.y,
        0.0,
        1.0
    );

    out.clip_position = camera.view_proj * world_pos;
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
