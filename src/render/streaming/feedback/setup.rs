mod buf;
mod common;
mod rt;

use super::{FeedbackMode, GpuFeedback};

impl GpuFeedback {
    pub fn new(device: &wgpu::Device, camera_layout: &wgpu::BindGroupLayout, use_rt: bool) -> Self {
        let feedback_instance_layout = common::create_feedback_instance_layout(device);
        let header = common::create_header_buffer(device);
        let output = common::create_output_buffer(device);
        let (collect_buf_layout, collect_buf_pipeline) = buf::create_collect_buf_resources(device);

        let rt = if use_rt {
            Some(rt::create_rt_resources(
                device,
                camera_layout,
                &feedback_instance_layout,
                &header,
                &output,
            ))
        } else {
            None
        };

        let readbacks = common::create_readbacks(device);

        Self {
            mode: if use_rt {
                FeedbackMode::Rt
            } else {
                FeedbackMode::Buf
            },
            header,
            output,
            readbacks,
            feedback_instance_layout,
            collect_buf_layout,
            collect_buf_pipeline,
            rt,
        }
    }

    pub fn mode_label(&self) -> &'static str {
        match self.mode {
            FeedbackMode::Rt => "RT",
            FeedbackMode::Buf => "BUF",
        }
    }

    pub fn is_rt(&self) -> bool {
        self.mode == FeedbackMode::Rt
    }

    pub fn create_instance_bind_group(
        &self,
        device: &wgpu::Device,
        instance_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Feedback Instance BG"),
            layout: &self.feedback_instance_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: instance_buffer.as_entire_binding(),
            }],
        })
    }

    pub fn create_collect_buf_bind_group(
        &self,
        device: &wgpu::Device,
        instance_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Feedback Collect BUF BG"),
            layout: &self.collect_buf_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.header.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: instance_buffer.as_entire_binding(),
                },
            ],
        })
    }
}
