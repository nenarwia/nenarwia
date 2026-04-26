use crate::render::instance::InstanceRaw;
use crate::render::streaming::feedback::FeedbackInstance;

pub struct DrawAssemblyState {
    pub slot_backdrop_buffer: wgpu::Buffer,
    pub slot_backdrop_capacity: usize,
    pub slot_backdrop_count: u32,
    pub slot_backdrop_dirty: bool,
    pub slot_backdrop_instances: Vec<InstanceRaw>,
    pub visible_buffer: wgpu::Buffer,
    pub visible_capacity: usize,
    pub visible_count: u32,
    pub visible_instances: Vec<InstanceRaw>,
    pub feedback_instances: Vec<FeedbackInstance>,
    pub feedback_instance_buffer: wgpu::Buffer,
    pub feedback_instance_capacity: usize,
    pub feedback_instance_bind_group: wgpu::BindGroup,
    pub feedback_collect_buf_bind_group: wgpu::BindGroup,
}

impl DrawAssemblyState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        slot_backdrop_capacity: usize,
        slot_backdrop_buffer: wgpu::Buffer,
        visible_capacity: usize,
        visible_buffer: wgpu::Buffer,
        feedback_instance_capacity: usize,
        feedback_instance_buffer: wgpu::Buffer,
        feedback_instance_bind_group: wgpu::BindGroup,
        feedback_collect_buf_bind_group: wgpu::BindGroup,
    ) -> Self {
        Self {
            slot_backdrop_buffer,
            slot_backdrop_capacity,
            slot_backdrop_count: 0,
            slot_backdrop_dirty: true,
            slot_backdrop_instances: Vec::with_capacity(slot_backdrop_capacity),
            visible_buffer,
            visible_capacity,
            visible_count: 0,
            visible_instances: Vec::with_capacity(visible_capacity),
            feedback_instances: Vec::with_capacity(feedback_instance_capacity),
            feedback_instance_buffer,
            feedback_instance_capacity,
            feedback_instance_bind_group,
            feedback_collect_buf_bind_group,
        }
    }

    pub fn clear_visible_instances(&mut self) {
        self.visible_instances.clear();
        self.visible_count = 0;
    }

    pub fn clear_draw_instances(&mut self) {
        self.clear_visible_instances();
        self.feedback_instances.clear();
    }

    pub fn reset_slot_backdrop(&mut self) {
        self.slot_backdrop_instances.clear();
        self.slot_backdrop_count = 0;
        self.slot_backdrop_dirty = true;
    }

    pub fn mark_slot_backdrop_dirty(&mut self) {
        self.slot_backdrop_dirty = true;
    }
}
