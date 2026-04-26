use super::init::{create_buffers, create_pipeline};
use super::types::InteractionUniform;
use crate::render::{gpu::GpuState, instance::InstanceRaw};

pub struct ComputeCulling {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group: wgpu::BindGroup,

    pub src_buffer: wgpu::Buffer,
    pub indirect_buffer: wgpu::Buffer,
    pub output_buffer: wgpu::Buffer,
    pub interaction_buffer: wgpu::Buffer,

    pub total_count: u32,
}

impl ComputeCulling {
    pub fn new(
        gpu: &GpuState,
        camera_buffer: &wgpu::Buffer,
        all_instances: &[InstanceRaw],
    ) -> Self {
        let (src_buffer, output_buffer, indirect_buffer, interaction_buffer) =
            create_buffers(gpu, all_instances);

        let (pipeline, bind_group) = create_pipeline(
            gpu,
            camera_buffer,
            &src_buffer,
            &output_buffer,
            &indirect_buffer,
            &interaction_buffer,
        );

        Self {
            pipeline,
            bind_group,
            src_buffer,
            indirect_buffer,
            output_buffer,
            interaction_buffer,
            total_count: all_instances.len() as u32,
        }
    }

    pub fn update_src_buffer(&mut self, queue: &wgpu::Queue, all_instances: &[InstanceRaw]) {
        queue.write_buffer(&self.src_buffer, 0, bytemuck::cast_slice(all_instances));
        self.total_count = all_instances.len() as u32;
    }
    pub fn update_src_instance(
        &self,
        queue: &wgpu::Queue,
        instance_idx: u32,
        instance: &InstanceRaw,
    ) {
        let offset = (instance_idx as u64) * (std::mem::size_of::<InstanceRaw>() as u64);
        queue.write_buffer(&self.src_buffer, offset, bytemuck::bytes_of(instance));
    }

    pub fn update_interaction(
        &self,
        queue: &wgpu::Queue,
        hovered_instance_idx: Option<u32>,
        loaded_instance_idx: Option<u32>,
    ) {
        let hovered_instance_idx = hovered_instance_idx.unwrap_or(u32::MAX);
        let loaded_instance_idx = loaded_instance_idx.unwrap_or(u32::MAX);

        let data = InteractionUniform {
            hovered_instance_idx,
            loaded_instance_idx,
            _pad: [0; 2],
        };
        queue.write_buffer(&self.interaction_buffer, 0, bytemuck::cast_slice(&[data]));
    }
}
