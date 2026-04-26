use super::manager::ComputeCulling;

impl ComputeCulling {
    pub fn compute(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.clear_buffer(&self.indirect_buffer, 4, Some(4));
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Cull Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);

        let groups = (self.total_count as f32 / 64.0).ceil() as u32;
        if groups > 0 {
            pass.dispatch_workgroups(groups, 1, 1);
        }
    }
}
