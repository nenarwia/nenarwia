const RENDER_SHADER_SOURCE: &str = include_str!(concat!(env!("OUT_DIR"), "/render_shader.wgsl"));

pub(super) fn render_shader_source() -> &'static str {
    RENDER_SHADER_SOURCE
}

#[cfg(test)]
mod tests {
    use super::render_shader_source;

    #[test]
    fn bundled_render_shader_contains_entry_points_and_ordered_sections() {
        let source = render_shader_source();
        assert!(source.contains("fn vs_main"));
        assert!(source.contains("fn fs_main"));

        let markers = [
            "// BEGIN shader/bindings_and_io.wgsl",
            "// BEGIN shader/vertex.wgsl",
            "// BEGIN shader/sampling_filters.wgsl",
            "// BEGIN shader/detail_tiles.wgsl",
            "// BEGIN shader/fit.wgsl",
            "// BEGIN shader/atlas.wgsl",
            "// BEGIN shader/fragment.wgsl",
        ];

        let mut last_pos = 0usize;
        for marker in markers {
            let pos = source
                .find(marker)
                .unwrap_or_else(|| panic!("missing bundled shader marker: {marker}"));
            assert!(pos >= last_pos, "shader marker order regressed at {marker}");
            last_pos = pos;
        }
    }
}
