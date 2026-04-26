use std::collections::{BTreeSet, BinaryHeap, HashMap, HashSet};

use crate::render::cache::CanvasMediaSlotId;
use crate::render::streaming::canvas_media_slots::CanvasMediaSlotQueueItem;

#[derive(Default)]
pub(crate) struct CanvasMediaSlotRuntimeState {
    pub(crate) pending: HashMap<CanvasMediaSlotId, u64>,
    pub(crate) queue_visible: BinaryHeap<CanvasMediaSlotQueueItem>,
    pub(crate) queue_prefetch: BTreeSet<CanvasMediaSlotQueueItem>,
}

impl CanvasMediaSlotRuntimeState {
    pub(super) fn clear_pending_work(&mut self) {
        self.pending.clear();
        self.queue_visible.clear();
        self.queue_prefetch.clear();
    }

    pub(super) fn remove_deleted_assets(&mut self, asset_keys: &HashSet<u64>) {
        self.pending
            .retain(|tile, _epoch| !asset_keys.contains(&tile.asset_key));
        retain_canvas_media_slot_heap(&mut self.queue_visible, |item| {
            !asset_keys.contains(&item.asset_key)
        });
        self.queue_prefetch
            .retain(|item| !asset_keys.contains(&item.asset_key));
    }

    pub(super) fn has_pending_current(&self, epoch: u64) -> bool {
        self.pending
            .values()
            .any(|&pending_epoch| pending_epoch == epoch)
    }
}

fn retain_canvas_media_slot_heap<F>(heap: &mut BinaryHeap<CanvasMediaSlotQueueItem>, keep: F)
where
    F: Fn(&CanvasMediaSlotQueueItem) -> bool,
{
    if heap.is_empty() {
        return;
    }

    let mut kept = Vec::with_capacity(heap.len());
    while let Some(item) = heap.pop() {
        if keep(&item) {
            kept.push(item);
        }
    }
    *heap = kept.into_iter().collect();
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, BinaryHeap, HashMap};

    use crate::render::cache::CanvasMediaSlotId;
    use crate::render::streaming::canvas_media_slots::CanvasMediaSlotQueueItem;

    use super::CanvasMediaSlotRuntimeState;

    fn sample_tile_id() -> CanvasMediaSlotId {
        CanvasMediaSlotId {
            asset_key: 77,
            lod: 2,
            x: 3,
            y: 4,
        }
    }

    fn sample_queue_item() -> CanvasMediaSlotQueueItem {
        CanvasMediaSlotQueueItem {
            id: 11,
            asset_key: 77,
            item_idx: 5,
            lod: 2,
            x: 3,
            y: 4,
            prio: 10,
            is_prefetch: false,
            epoch: 9,
            queued_frame: 12,
        }
    }

    fn make_canvas_media_slot_state() -> CanvasMediaSlotRuntimeState {
        let tile_id = sample_tile_id();
        let mut pending = HashMap::new();
        pending.insert(tile_id, 9);

        let mut visible_queue = BinaryHeap::new();
        visible_queue.push(sample_queue_item());

        let mut prefetch_queue = BTreeSet::new();
        let mut prefetch_item = sample_queue_item();
        prefetch_item.is_prefetch = true;
        prefetch_queue.insert(prefetch_item);

        CanvasMediaSlotRuntimeState {
            pending,
            queue_visible: visible_queue,
            queue_prefetch: prefetch_queue,
        }
    }

    #[test]
    fn clear_pending_work_empties_pending_and_queues() {
        let mut state = make_canvas_media_slot_state();

        state.clear_pending_work();

        assert!(state.pending.is_empty());
        assert!(state.queue_visible.is_empty());
        assert!(state.queue_prefetch.is_empty());
    }

    #[test]
    fn remove_deleted_assets_prunes_pending_and_queue_entries() {
        let mut state = make_canvas_media_slot_state();
        let asset_keys = [77u64].into_iter().collect();

        state.remove_deleted_assets(&asset_keys);

        assert!(state.pending.is_empty());
        assert!(state.queue_visible.is_empty());
        assert!(state.queue_prefetch.is_empty());
    }
}
