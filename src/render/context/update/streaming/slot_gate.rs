use std::collections::HashSet;
use std::time::Instant;

use crate::render::context::state::{RenderContext, SlotInteractionTransition};

fn stage0_log_enabled() -> bool {
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let val = std::env::var("CANVAS_STAGE0_LOG")
            .unwrap_or_default()
            .to_lowercase();
        matches!(val.as_str(), "1" | "true" | "yes" | "on")
    })
}

fn clear_slot_interaction_state(ctx: &mut RenderContext) {
    let pending_slots_before = ctx.streaming_runtime.preview.pending_slots.len();
    let pending_media_slots_before = ctx.streaming_runtime.canvas_media_slots.pending.len();
    let visible_queue_before = ctx.streaming_runtime.canvas_media_slots.queue_visible.len();
    let prefetch_queue_before = ctx
        .streaming_runtime
        .canvas_media_slots
        .queue_prefetch
        .len();

    ctx.streaming_runtime.clear_all_pending_work();
    ctx.streaming_runtime.clear_preview_planning_state();
    ctx.clear_quality_visibility_tracking();

    let keep = HashSet::new();
    let (purged_thumb_jobs, canceled_thumb_subscribers) = ctx
        .loader
        .retain_queued_thumbnails_epoch_keys(ctx.streaming_runtime.stream_epoch, &keep);

    if stage0_log_enabled() {
        log::info!(
            "Stage0SlotGate | action=OFF vis={} pending preview/media_slots={} / {} queue vis/pref={} / {} purged_thumb_jobs={} canceled_thumb_subscribers={}",
            ctx.committed_view.visible_items.len(),
            pending_slots_before,
            pending_media_slots_before,
            visible_queue_before,
            prefetch_queue_before,
            purged_thumb_jobs,
            canceled_thumb_subscribers,
        );
    }
}

pub(crate) fn force_disable_slot_interaction_gate(ctx: &mut RenderContext) {
    if ctx.streaming_runtime.slot_interaction_gate.enabled {
        ctx.streaming_runtime.slot_interaction_gate.enabled = false;
        ctx.streaming_runtime.slot_interaction_gate.pending_on_since = None;
        clear_slot_interaction_state(ctx);
        return;
    }

    ctx.streaming_runtime.slot_interaction_gate.pending_on_since = None;
}

pub(super) fn update_slot_interaction_gate(ctx: &mut RenderContext, visible_count: usize) {
    let transition = ctx
        .streaming_runtime
        .slot_interaction_gate
        .update(Instant::now(), visible_count);

    match transition {
        SlotInteractionTransition::TurnedOff => {
            clear_slot_interaction_state(ctx);
        }
        SlotInteractionTransition::TurnedOn => {
            if stage0_log_enabled() {
                log::info!(
                    "Stage0SlotGate | action=ON vis={} mode={} off_thr={} on_immediate={} delay_frames={}",
                    visible_count,
                    if visible_count
                        < ctx
                            .streaming_runtime
                            .slot_interaction_gate
                            .on_immediate_visible_threshold
                    {
                        "immediate"
                    } else {
                        "delayed"
                    },
                    ctx.streaming_runtime
                        .slot_interaction_gate
                        .off_visible_threshold,
                    ctx.streaming_runtime
                        .slot_interaction_gate
                        .on_immediate_visible_threshold,
                    ctx.streaming_runtime.slot_interaction_gate.on_delay_frames,
                );
            }
        }
        SlotInteractionTransition::None => {}
    }
}
