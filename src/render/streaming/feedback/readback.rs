use std::sync::atomic::Ordering;

use super::{FeedbackResult, GpuFeedback, GpuFeedbackTile, HEADER_BYTES, MAX_TILES};

impl GpuFeedback {
    pub fn request_maps(&mut self) {
        for slot in self.readbacks.iter_mut() {
            if !slot.pending || slot.map_requested {
                continue;
            }
            let ready = slot.ready.clone();
            let failed = slot.failed.clone();
            ready.store(false, Ordering::Release);
            failed.store(false, Ordering::Release);
            slot.buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |res| {
                    if res.is_ok() {
                        ready.store(true, Ordering::Release);
                    } else {
                        failed.store(true, Ordering::Release);
                    }
                });
            slot.map_requested = true;
        }
    }

    pub fn collect_ready(&mut self, current_frame: u64) -> Vec<FeedbackResult> {
        let mut out = Vec::new();
        let tile_bytes = MAX_TILES * std::mem::size_of::<GpuFeedbackTile>();

        for slot in self.readbacks.iter_mut() {
            if !slot.pending {
                continue;
            }

            if slot.failed.load(Ordering::Acquire) {
                slot.pending = false;
                slot.map_requested = false;
                slot.ready.store(false, Ordering::Release);
                slot.failed.store(false, Ordering::Release);
                continue;
            }

            if !slot.ready.load(Ordering::Acquire) {
                continue;
            }

            let data = slot.buffer.slice(..).get_mapped_range();
            if data.len() < HEADER_BYTES as usize {
                slot.buffer.unmap();
                slot.pending = false;
                slot.map_requested = false;
                slot.failed.store(false, Ordering::Release);
                continue;
            }

            let header = bytemuck::from_bytes::<[u32; 4]>(&data[0..16]);
            let count = (header[0] as usize).min(MAX_TILES);
            let overflow = header[2] != 0;
            let bytes_needed = count * std::mem::size_of::<GpuFeedbackTile>();
            let tiles = if bytes_needed <= tile_bytes {
                bytemuck::cast_slice(&data[16..16 + bytes_needed]).to_vec()
            } else {
                Vec::new()
            };

            drop(data);
            slot.buffer.unmap();
            slot.pending = false;
            slot.map_requested = false;
            slot.ready.store(false, Ordering::Release);
            slot.failed.store(false, Ordering::Release);

            let latency = current_frame.saturating_sub(slot.submitted_frame) as u32;
            out.push(FeedbackResult {
                tiles,
                overflow,
                latency_frames: latency,
            });
        }

        out
    }

    pub(super) fn next_free_readback(&self) -> Option<usize> {
        self.readbacks.iter().position(|slot| !slot.pending)
    }
}
