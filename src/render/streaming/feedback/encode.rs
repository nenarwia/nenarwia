use super::{
    FeedbackEncodeInput, FeedbackMode, GpuFeedback, GpuFeedbackTile, FEEDBACK_BLOCK,
    FEEDBACK_BUF_WORKGROUP, FEEDBACK_TEX_H, FEEDBACK_TEX_W, HEADER_BYTES, MAX_TILES,
};

impl GpuFeedback {
    pub fn encode(&mut self, input: FeedbackEncodeInput<'_>) {
        let slot = match self.next_free_readback() {
            Some(idx) => idx,
            None => return,
        };

        let header = self.header_words(input.instance_count);
        input
            .queue
            .write_buffer(&self.header, 0, bytemuck::cast_slice(&header));

        match self.mode {
            FeedbackMode::Rt => {
                self.encode_rt(
                    input.encoder,
                    input.camera_bg,
                    input.instance_buffer,
                    input.instance_count,
                    input.feedback_instance_bg,
                );
            }
            FeedbackMode::Buf => {
                self.encode_buf(
                    input.encoder,
                    input.feedback_collect_buf_bg,
                    input.instance_count,
                );
            }
        }

        let rb = &self.readbacks[slot];
        input
            .encoder
            .copy_buffer_to_buffer(&self.header, 0, &rb.buffer, 0, HEADER_BYTES);
        input.encoder.copy_buffer_to_buffer(
            &self.output,
            0,
            &rb.buffer,
            HEADER_BYTES,
            (MAX_TILES * std::mem::size_of::<GpuFeedbackTile>()) as u64,
        );

        self.readbacks[slot].pending = true;
        self.readbacks[slot].map_requested = false;
        self.readbacks[slot].submitted_frame = input.frame_index;
    }

    fn header_words(&self, instance_count: u32) -> [u32; 4] {
        if self.mode == FeedbackMode::Buf {
            [0u32, MAX_TILES as u32, 0u32, instance_count]
        } else {
            [0u32, MAX_TILES as u32, 0u32, 0u32]
        }
    }

    fn encode_rt(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        camera_bg: &wgpu::BindGroup,
        instance_buffer: &wgpu::Buffer,
        instance_count: u32,
        feedback_instance_bg: &wgpu::BindGroup,
    ) {
        let Some(rt) = self.rt.as_ref() else {
            return;
        };

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Feedback Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &rt.feedback_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &rt.feedback_valid_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                multiview_mask: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&rt.feedback_pipeline);
            pass.set_bind_group(0, camera_bg, &[]);
            pass.set_bind_group(1, feedback_instance_bg, &[]);
            pass.set_vertex_buffer(0, instance_buffer.slice(..));
            pass.draw(0..6, 0..instance_count);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Feedback Collect RT Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&rt.collect_rt_pipeline);
            pass.set_bind_group(0, &rt.collect_rt_bind_group, &[]);
            let gx = FEEDBACK_TEX_W.div_ceil(FEEDBACK_BLOCK);
            let gy = FEEDBACK_TEX_H.div_ceil(FEEDBACK_BLOCK);
            pass.dispatch_workgroups(gx, gy, 1);
        }
    }

    fn encode_buf(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        feedback_collect_buf_bg: &wgpu::BindGroup,
        instance_count: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Feedback Collect BUF Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.collect_buf_pipeline);
        pass.set_bind_group(0, feedback_collect_buf_bg, &[]);
        let gx = instance_count.div_ceil(FEEDBACK_BUF_WORKGROUP);
        if gx > 0 {
            pass.dispatch_workgroups(gx, 1, 1);
        }
    }
}
