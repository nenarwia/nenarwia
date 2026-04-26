pub(super) fn write_texture_region(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    dst_rect: [u32; 4],
    pixels: &[u8],
    width: u32,
    height: u32,
) {
    if width == 0 || height == 0 {
        return;
    }
    if pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: dst_rect[0],
                y: dst_rect[1],
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}
