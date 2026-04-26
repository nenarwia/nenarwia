use std::collections::HashSet;
use std::time::Duration;

use crate::render::atlas::ThumbClass;

use super::{SlotInteractionGate, StreamingConfig};

mod budgets;
mod canvas_media_slots;
mod preview;
mod residency;

#[cfg(test)]
mod tests;

use budgets::StreamingBudgetState;
use canvas_media_slots::CanvasMediaSlotRuntimeState;
use preview::PreviewRuntimeState;
use residency::SlotResidencyRuntimeState;

pub(crate) struct StreamingRuntimeInit {
    pub(crate) last_epoch_zoom: f64,
    pub(crate) preview_coverage_upload_ema: f32,
    pub(crate) preview_coverage_outstanding_min: usize,
    pub(crate) preview_coverage_outstanding_max: usize,
    pub(crate) slot_residency_update_interval_frames: u64,
    pub(crate) slot_residency_grace_frames_idle: u64,
    pub(crate) slot_residency_grace_frames_moving: u64,
    pub(crate) slot_interaction_gate: SlotInteractionGate,
}

pub struct StreamingRuntimeState {
    pub(crate) stream_epoch: u64,
    pub(crate) last_epoch_zoom: f64,
    pub(crate) preview: PreviewRuntimeState,
    pub(crate) canvas_media_slots: CanvasMediaSlotRuntimeState,
    pub(crate) budgets: StreamingBudgetState,
    pub(crate) residency: SlotResidencyRuntimeState,
    pub(crate) slot_interaction_gate: SlotInteractionGate,
}

impl StreamingRuntimeState {
    pub(crate) fn new(init: StreamingRuntimeInit) -> Self {
        Self {
            stream_epoch: 0,
            last_epoch_zoom: init.last_epoch_zoom,
            preview: PreviewRuntimeState::new(
                init.preview_coverage_upload_ema,
                init.preview_coverage_outstanding_min,
                init.preview_coverage_outstanding_max,
            ),
            canvas_media_slots: CanvasMediaSlotRuntimeState::default(),
            budgets: StreamingBudgetState::default(),
            residency: SlotResidencyRuntimeState::new(
                init.slot_residency_update_interval_frames,
                init.slot_residency_grace_frames_idle,
                init.slot_residency_grace_frames_moving,
            ),
            slot_interaction_gate: init.slot_interaction_gate,
        }
    }

    pub fn prepare_frame_budgets(
        &mut self,
        streaming: &StreamingConfig,
        camera_moving: bool,
        frame_dt: Duration,
    ) {
        self.budgets
            .prepare_frame_budgets(streaming, camera_moving, frame_dt);
    }

    pub fn advance_epoch(&mut self) -> u64 {
        self.stream_epoch = self.stream_epoch.wrapping_add(1);
        self.stream_epoch
    }

    pub fn clear_thumbnail_work(&mut self) {
        self.preview.clear_pending_work();
    }

    pub fn clear_canvas_media_slot_work(&mut self) {
        self.canvas_media_slots.clear_pending_work();
    }

    pub fn clear_all_pending_work(&mut self) {
        self.clear_thumbnail_work();
        self.clear_canvas_media_slot_work();
    }

    pub fn clear_preview_completion_tracking(&mut self) {
        self.preview.clear_completion_tracking();
    }

    pub fn clear_preview_planning_state(&mut self) {
        self.preview.clear_planning_state();
    }

    pub fn clear_preview_cache_state(&mut self) {
        self.preview.clear_cache_state();
    }

    pub fn reset_slot_residency(&mut self) {
        self.residency.reset();
    }

    pub fn remove_deleted_assets(&mut self, asset_keys: &HashSet<u64>, slot_ids: &HashSet<u64>) {
        if asset_keys.is_empty() && slot_ids.is_empty() {
            return;
        }

        self.preview.remove_deleted_assets(asset_keys, slot_ids);
        self.canvas_media_slots.remove_deleted_assets(asset_keys);
        self.residency.remove_deleted_assets(asset_keys);
    }

    pub fn consume_thumb_budget(&mut self, class: ThumbClass) -> bool {
        self.budgets.consume_thumb_budget(class)
    }

    pub fn consume_canvas_media_slot_budget(&mut self, count: usize) {
        self.budgets.consume_canvas_media_slot_budget(count);
    }

    pub fn consume_canvas_media_slot_min_visible_budget(&mut self, count: usize) {
        self.budgets
            .consume_canvas_media_slot_min_visible_budget(count);
    }

    pub fn consume_upload_budget(&mut self, count: usize) {
        self.budgets.consume_upload_budget(count);
    }

    pub(crate) fn has_pending_slots_current(&self) -> bool {
        self.preview.has_pending_current(self.stream_epoch)
    }

    pub(crate) fn has_pending_canvas_media_slots_current(&self) -> bool {
        self.canvas_media_slots
            .has_pending_current(self.stream_epoch)
    }
}
