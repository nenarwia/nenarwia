use std::path::PathBuf;

use crate::render::context::state::RenderContext;

impl RenderContext {
    pub(crate) fn slot_path_for_explorer_reveal(&self, slot_id: u64) -> Result<PathBuf, String> {
        let idx = self
            .scene
            .index_for_id(slot_id)
            .ok_or_else(|| format!("Slot {slot_id} no longer exists."))?;
        let path = self
            .slot_paths
            .get(idx)
            .and_then(|path| path.live_path())
            .ok_or_else(|| format!("Slot {slot_id} is already empty."))?;

        Ok(path.to_path_buf())
    }
}
