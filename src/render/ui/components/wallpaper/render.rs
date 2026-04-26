use crate::render::ui::WallpaperUi;

impl WallpaperUi {
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if !self.enabled {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}
