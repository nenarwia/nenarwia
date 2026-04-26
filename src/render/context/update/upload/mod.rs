pub mod thumbnails;
pub mod tiles;
use std::time::Instant;

use crate::render::context::state::RenderContext;

pub fn process_loaded_images(ctx: &mut RenderContext) {
    let mut any_activity = false;

    let max_uploads = ctx.streaming.max_uploads_per_frame;
    let limit = if max_uploads == 0 {
        usize::MAX
    } else {
        ctx.streaming_runtime.budgets.upload_budget_remaining
    };
    let budget = ctx.streaming_runtime.budgets.upload_cpu_budget_for_update;
    let start = Instant::now();
    let mut processed = 0usize;

    while processed < limit {
        if let Some(budget) = budget {
            if processed.is_multiple_of(8) && start.elapsed() >= budget {
                break;
            }
        }
        let Some(img) = ctx.loader.try_recv() else {
            break;
        };
        any_activity = true;

        if img.is_detail {
            tiles::upload_tile(ctx, img);
        } else {
            thumbnails::upload_thumbnail(ctx, img);
        }
        processed += 1;
    }
    ctx.streaming_runtime.consume_upload_budget(processed);

    if any_activity
        || ctx.has_pending_slots_current()
        || ctx.has_pending_canvas_media_slots_current()
    {
        ctx.mark_redraw_pending();
    }
}
