use super::manager::{LoadError, LoadResult, RequestKey};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering as AtomicOrdering;
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use super::manager::JobQueue;
use super::processor::{self, WorkerState};
use super::types::LoadedImage;
use crate::core::metrics;

/// The main loop for a loader thread.
/// Pops jobs from the shared priority queue and delegates to the processor.
pub fn worker_loop(
    queue: Arc<(Mutex<JobQueue>, Condvar)>,
    tx: mpsc::Sender<LoadedImage>,
    pending_jobs: Arc<AtomicUsize>,
    ready_results: Arc<AtomicUsize>,
    epoch: Arc<AtomicU64>,
    inflight_canvas_media_slots: Arc<AtomicUsize>,
) {
    let mut st = WorkerState::default();
    loop {
        let job_opt = {
            let (lock, cvar) = &*queue;
            let mut q = lock.lock().unwrap();
            while q.heap.is_empty() {
                q = cvar.wait(q).unwrap();
            }
            q.heap.pop()
        };

        let Some(job) = job_opt else {
            continue;
        };

        // If subscribers were canceled after the job was queued (e.g. viewport changed),
        // skip expensive decode work for this request.
        if !job_has_subscribers(&queue, &job.key) {
            pending_jobs.fetch_sub(1, AtomicOrdering::Relaxed);
            if matches!(job.key, RequestKey::CanvasMediaSlot(_)) {
                inflight_canvas_media_slots.fetch_sub(1, AtomicOrdering::Relaxed);
            }
            continue;
        }

        // Skip outdated requests (zoom epoch changed)
        let current_epoch = epoch.load(AtomicOrdering::Relaxed);
        if job.req.epoch() != current_epoch {
            finish_in_flight(&queue, &job.key, Err(LoadError::Canceled));
            pending_jobs.fetch_sub(1, AtomicOrdering::Relaxed);
            if matches!(job.key, RequestKey::CanvasMediaSlot(_)) {
                inflight_canvas_media_slots.fetch_sub(1, AtomicOrdering::Relaxed);
            }
            continue;
        }

        // Delegate heavy lifting to the processor module
        let start = Instant::now();
        if matches!(job.key, RequestKey::CanvasMediaSlot(_)) {
            metrics::record_tiles_started();
        }
        let result = processor::process_request(job.req, &mut st);
        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;
        metrics::record_decode_ms(elapsed_ms);
        metrics::record_decode_job();
        match job.key {
            RequestKey::CanvasMediaSlot(_) => {
                metrics::record_tile_build_ms(elapsed_ms);
                metrics::record_tile_job_ms(elapsed_ms);
                inflight_canvas_media_slots.fetch_sub(1, AtomicOrdering::Relaxed);
            }
            RequestKey::Thumbnail(_) => {
                metrics::record_thumb_job_ms(elapsed_ms);
            }
        }
        let _ = tx.send(result.clone());
        finish_in_flight(&queue, &job.key, Ok(result));
        ready_results.fetch_add(1, AtomicOrdering::Relaxed);
        pending_jobs.fetch_sub(1, AtomicOrdering::Relaxed);
    }
}

fn finish_in_flight(queue: &Arc<(Mutex<JobQueue>, Condvar)>, key: &RequestKey, result: LoadResult) {
    let subscribers = {
        let (lock, _) = &**queue;
        let mut q = lock.lock().unwrap();
        match key {
            RequestKey::CanvasMediaSlot(k) => q.in_flight_canvas_media_slots.remove(k),
            RequestKey::Thumbnail(k) => q.in_flight_thumbs.remove(k),
        }
    };

    if let Some(list) = subscribers {
        for tx in list {
            let _ = tx.send(result.clone());
        }
    }
}

fn job_has_subscribers(queue: &Arc<(Mutex<JobQueue>, Condvar)>, key: &RequestKey) -> bool {
    let (lock, _) = &**queue;
    let q = lock.lock().unwrap();
    match key {
        RequestKey::CanvasMediaSlot(k) => q.in_flight_canvas_media_slots.contains_key(k),
        RequestKey::Thumbnail(k) => q.in_flight_thumbs.contains_key(k),
    }
}
