use std::collections::HashSet;

use super::VisibleItem;

pub struct CommittedViewState {
    pub visible_items: Vec<VisibleItem>,
    pub visible_ids: HashSet<u64>,
    pub slot_visibility_zoom_lock: Option<f64>,
}

impl CommittedViewState {
    pub fn with_visible_capacity(visible_capacity: usize) -> Self {
        Self {
            visible_items: Vec::with_capacity(visible_capacity),
            visible_ids: HashSet::with_capacity(visible_capacity),
            slot_visibility_zoom_lock: None,
        }
    }

    pub fn clear_visible_membership(&mut self) {
        self.visible_items.clear();
        self.visible_ids.clear();
    }

    pub fn clear_visible_ids(&mut self) {
        self.visible_ids.clear();
    }

    pub fn rebuild_visible_ids(&mut self) {
        self.visible_ids.clear();
        for item in self.visible_items.iter().copied() {
            self.visible_ids.insert(item.id);
        }
    }

    pub fn clear_zoom_lock(&mut self) {
        self.slot_visibility_zoom_lock = None;
    }
}
