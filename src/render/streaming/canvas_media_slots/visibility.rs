use crate::render::cache::{math, CanvasMediaSlotId};
use crate::render::context::state::RenderContext;

#[derive(Clone, Copy, Debug, Default)]
pub struct VisibleTileStats {
    pub missing: u32,
    pub total: u32,
}

impl VisibleTileStats {
    pub fn ratio(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.missing as f32 / self.total as f32
        }
    }
}

pub fn visible_tiles_stats(
    ctx: &RenderContext,
    asset_key: u64,
    lod: u8,
    v: &math::VisibleTiles,
) -> VisibleTileStats {
    let mut stats = VisibleTileStats::default();
    let tiles_x = v.max_tx.saturating_sub(v.min_tx);
    let tiles_y = v.max_ty.saturating_sub(v.min_ty);
    let total = tiles_x.saturating_mul(tiles_y);
    if total == 0 {
        return stats;
    }

    const MAX_STAT_TILES: u32 = 1024;

    if total <= MAX_STAT_TILES {
        for ty in v.min_ty..v.max_ty {
            for tx in v.min_tx..v.max_tx {
                stats.total = stats.total.saturating_add(1);
                let tile_id = CanvasMediaSlotId {
                    asset_key,
                    lod,
                    x: tx,
                    y: ty,
                };
                if ctx.page_table.get_slot(tile_id).is_none() {
                    stats.missing = stats.missing.saturating_add(1);
                }
            }
        }
        return stats;
    }

    let stride = ((total as f32 / MAX_STAT_TILES as f32).sqrt().ceil() as u32).max(1);
    let mut checked: u32 = 0;
    let mut missing: u32 = 0;

    let step = stride as usize;
    for ty in (v.min_ty..v.max_ty).step_by(step) {
        for tx in (v.min_tx..v.max_tx).step_by(step) {
            checked = checked.saturating_add(1);
            let tile_id = CanvasMediaSlotId {
                asset_key,
                lod,
                x: tx,
                y: ty,
            };
            if ctx.page_table.get_slot(tile_id).is_none() {
                missing = missing.saturating_add(1);
            }
        }
    }

    if checked == 0 {
        return stats;
    }

    let ratio = missing as f32 / checked as f32;
    stats.total = total;
    stats.missing = (ratio * total as f32).round() as u32;
    stats
}
