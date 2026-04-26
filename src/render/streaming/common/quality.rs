use crate::render::context::state::RenderContext;

pub(crate) fn update_quality_debt(ctx: &mut RenderContext, item_idx: usize, ratio: f32) {
    if item_idx >= ctx.scene.quality_debt.len() {
        return;
    }
    let ratio = ratio.clamp(0.0, 1.0);
    let debt = &mut ctx.scene.quality_debt[item_idx];
    if ratio > 0.0 {
        *debt = (*debt * 0.85) + ratio;
        if *debt > 10.0 {
            *debt = 10.0;
        }
    } else {
        *debt *= 0.5;
    }
}

pub(crate) fn record_ttfq_if_ready(ctx: &mut RenderContext, id: u64, ready: bool) {
    if !ready {
        return;
    }
    if let Some(start) = ctx.quality_visible_since.remove(&id) {
        let frames = ctx.frame_count.saturating_sub(start);
        ctx.quality_stats.record_ttfq(frames);
    }
}
