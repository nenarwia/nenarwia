use std::path::{Path, PathBuf};

use crate::render::context::state::RenderContext;

impl RenderContext {
    pub(crate) fn validate_manual_canvas_import_paths(
        &self,
        _paths: &[PathBuf],
    ) -> Result<(), String> {
        Ok(())
    }

    pub(crate) fn validate_manual_canvas_fill_path(&self, path: &Path) -> Result<(), String> {
        let owned = [path.to_path_buf()];
        self.validate_manual_canvas_import_paths(&owned)
    }
}
