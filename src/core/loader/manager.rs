use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

use super::types::{LoadRequest, LoadedImage};
use crate::core::metrics;

/// Dedup key for requests.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct CanvasMediaSlotKey {
    pub asset_key: u64,
    pub lod: u8,
    pub x: u32,
    pub y: u32,
    pub pipeline_version: u64,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct ThumbKey {
    pub asset_key: u64,
    pub tier: u16,
    pub pipeline_version: u64,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum RequestKey {
    Thumbnail(ThumbKey),
    CanvasMediaSlot(CanvasMediaSlotKey),
}

#[derive(Clone, Debug)]
pub enum LoadError {
    Canceled,
}

impl From<&LoadRequest> for RequestKey {
    fn from(r: &LoadRequest) -> Self {
        match r {
            LoadRequest::Thumbnail {
                asset_key,
                size,
                epoch,
                ..
            } => RequestKey::Thumbnail(ThumbKey {
                asset_key: *asset_key,
                tier: *size,
                pipeline_version: *epoch,
            }),
            LoadRequest::CanvasMediaSlot {
                asset_key,
                lod,
                tile_x,
                tile_y,
                epoch,
                ..
            } => RequestKey::CanvasMediaSlot(CanvasMediaSlotKey {
                asset_key: *asset_key,
                lod: *lod,
                x: *tile_x,
                y: *tile_y,
                pipeline_version: *epoch,
            }),
        }
    }
}

pub type LoadResult = Result<LoadedImage, LoadError>;
pub type OneshotReceiver = mpsc::Receiver<LoadResult>;
type OneshotSender = mpsc::Sender<LoadResult>;

/// Job stored in the priority queue.
#[derive(Clone, Debug)]
pub(crate) struct Job {
    pub prio: i32,
    pub seq: u64,
    pub key: RequestKey,
    pub req: LoadRequest,
}

impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        self.prio
            .cmp(&other.prio)
            .then_with(|| self.seq.cmp(&other.seq))
    }
}

impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.prio == other.prio && self.seq == other.seq
    }
}

impl Eq for Job {}

pub(crate) struct JobQueue {
    pub heap: BinaryHeap<Job>,
    pub seq: u64,
    pub in_flight_canvas_media_slots: HashMap<CanvasMediaSlotKey, Vec<OneshotSender>>,
    pub in_flight_thumbs: HashMap<ThumbKey, Vec<OneshotSender>>,
}

pub struct AsyncLoader {
    receiver: mpsc::Receiver<LoadedImage>,
    queue: Arc<(Mutex<JobQueue>, Condvar)>,
    epoch: Arc<AtomicU64>,

    pending_jobs: Arc<AtomicUsize>,
    ready_results: Arc<AtomicUsize>,
    inflight_canvas_media_slots: Arc<AtomicUsize>,
}

fn atomic_saturating_sub(atom: &AtomicUsize, amount: usize) {
    if amount == 0 {
        return;
    }
    let _ = atom.fetch_update(
        AtomicOrdering::Relaxed,
        AtomicOrdering::Relaxed,
        |current| Some(current.saturating_sub(amount)),
    );
}

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

impl AsyncLoader {
    pub fn start() -> Self {
        let (res_tx, res_rx) = mpsc::channel::<LoadedImage>();

        let queue = Arc::new((
            Mutex::new(JobQueue {
                heap: BinaryHeap::new(),
                seq: 0,
                in_flight_canvas_media_slots: HashMap::new(),
                in_flight_thumbs: HashMap::new(),
            }),
            Condvar::new(),
        ));

        let pending_jobs = Arc::new(AtomicUsize::new(0));
        let ready_results = Arc::new(AtomicUsize::new(0));
        let inflight_canvas_media_slots = Arc::new(AtomicUsize::new(0));
        let epoch = Arc::new(AtomicU64::new(0));

        let workers = std::env::var("CANVAS_LOADER_WORKERS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .map(|v| v.clamp(1, 32))
            .unwrap_or_else(|| super::processor::max_decode_jobs_value().clamp(1, 12));

        for _ in 0..workers {
            let (q_clone, tx_clone) = (queue.clone(), res_tx.clone());
            let pj = pending_jobs.clone();
            let rr = ready_results.clone();
            let ep = epoch.clone();
            let inflight = inflight_canvas_media_slots.clone();
            thread::spawn(move || {
                super::worker::worker_loop(q_clone, tx_clone, pj, rr, ep, inflight)
            });
        }

        Self {
            receiver: res_rx,
            queue,
            epoch,
            pending_jobs,
            ready_results,
            inflight_canvas_media_slots,
        }
    }

    pub fn request_prio(&self, req: LoadRequest, prio: i32) -> OneshotReceiver {
        let (tx_one, rx_one) = mpsc::channel();
        let (lock, cvar) = &*self.queue;
        let mut q = lock.lock().unwrap();

        let key = RequestKey::from(&req);
        match &key {
            RequestKey::CanvasMediaSlot(k) => {
                metrics::record_tiles_requested();
                self.inflight_canvas_media_slots
                    .fetch_add(1, AtomicOrdering::Relaxed);
                if let Some(list) = q.in_flight_canvas_media_slots.get_mut(k) {
                    list.push(tx_one);
                    metrics::record_tiles_singleflight_hit();
                    log::debug!("Singleflight: canvas media slot hit {:?}", k);
                    self.inflight_canvas_media_slots
                        .fetch_sub(1, AtomicOrdering::Relaxed);
                    return rx_one;
                }
                q.in_flight_canvas_media_slots
                    .insert(k.clone(), vec![tx_one]);
            }
            RequestKey::Thumbnail(k) => {
                if let Some(list) = q.in_flight_thumbs.get_mut(k) {
                    list.push(tx_one);
                    metrics::record_thumbs_singleflight_hit();
                    log::debug!("Singleflight: thumb hit {:?}", k);
                    return rx_one;
                }
                q.in_flight_thumbs.insert(k.clone(), vec![tx_one]);
            }
        }

        q.seq = q.seq.wrapping_add(1);
        let seq = q.seq;
        q.heap.push(Job {
            prio,
            seq,
            key,
            req,
        });
        self.pending_jobs.fetch_add(1, AtomicOrdering::Relaxed);
        cvar.notify_one();
        rx_one
    }

    pub fn try_recv(&self) -> Option<LoadedImage> {
        match self.receiver.try_recv() {
            Ok(v) => {
                self.ready_results.fetch_sub(1, AtomicOrdering::Relaxed);
                Some(v)
            }
            Err(_) => None,
        }
    }

    pub fn has_pending_work(&self) -> bool {
        self.pending_jobs.load(AtomicOrdering::Relaxed) > 0
            || self.ready_results.load(AtomicOrdering::Relaxed) > 0
    }

    pub fn set_epoch(&self, epoch: u64) {
        let prev = self.epoch.swap(epoch, AtomicOrdering::Relaxed);
        if prev == epoch {
            return;
        }

        let (
            removed_jobs,
            removed_slot_jobs,
            canceled_slot_subscribers,
            canceled_thumb_subscribers,
        ) = {
            let (lock, cvar) = &*self.queue;
            let mut q = lock.lock().unwrap();

            let mut removed_jobs = 0usize;
            let mut removed_slot_jobs = 0usize;
            let mut canceled_slot_subscribers = 0usize;
            let mut canceled_thumb_subscribers = 0usize;
            let mut kept = BinaryHeap::with_capacity(q.heap.len());
            while let Some(job) = q.heap.pop() {
                if job.req.epoch() == epoch {
                    kept.push(job);
                } else {
                    removed_jobs = removed_jobs.saturating_add(1);
                    if matches!(job.key, RequestKey::CanvasMediaSlot(_)) {
                        removed_slot_jobs = removed_slot_jobs.saturating_add(1);
                    }
                }
            }
            q.heap = kept;

            q.in_flight_canvas_media_slots.retain(|key, subscribers| {
                if key.pipeline_version == epoch {
                    true
                } else {
                    canceled_slot_subscribers =
                        canceled_slot_subscribers.saturating_add(subscribers.len());
                    for tx in subscribers.drain(..) {
                        let _ = tx.send(Err(LoadError::Canceled));
                    }
                    false
                }
            });
            q.in_flight_thumbs.retain(|key, subscribers| {
                if key.pipeline_version == epoch {
                    true
                } else {
                    canceled_thumb_subscribers =
                        canceled_thumb_subscribers.saturating_add(subscribers.len());
                    for tx in subscribers.drain(..) {
                        let _ = tx.send(Err(LoadError::Canceled));
                    }
                    false
                }
            });

            cvar.notify_all();
            (
                removed_jobs,
                removed_slot_jobs,
                canceled_slot_subscribers,
                canceled_thumb_subscribers,
            )
        };

        atomic_saturating_sub(&self.pending_jobs, removed_jobs);
        atomic_saturating_sub(&self.inflight_canvas_media_slots, removed_slot_jobs);
        if stage0_log_enabled() {
            log::info!(
                "Stage0Reset | loader epoch={} purged queued jobs={} (slots={}) canceled subs slot/thumb={}/{}",
                epoch,
                removed_jobs,
                removed_slot_jobs,
                canceled_slot_subscribers,
                canceled_thumb_subscribers,
            );
        }
    }

    pub fn retain_queued_thumbnails_epoch_keys(
        &self,
        epoch: u64,
        keep: &HashSet<(u64, u16)>,
    ) -> (usize, usize) {
        let (removed_jobs, canceled_thumb_subscribers) = {
            let (lock, cvar) = &*self.queue;
            let mut q = lock.lock().unwrap();

            let mut removed_jobs = 0usize;
            let mut canceled_thumb_subscribers = 0usize;
            let mut kept = BinaryHeap::with_capacity(q.heap.len());
            while let Some(job) = q.heap.pop() {
                let should_drop_thumb = match &job.key {
                    RequestKey::Thumbnail(key) if key.pipeline_version == epoch => {
                        !keep.contains(&(key.asset_key, key.tier))
                    }
                    _ => false,
                };
                if should_drop_thumb {
                    removed_jobs = removed_jobs.saturating_add(1);
                } else {
                    kept.push(job);
                }
            }
            q.heap = kept;

            q.in_flight_thumbs.retain(|key, subscribers| {
                if key.pipeline_version != epoch || keep.contains(&(key.asset_key, key.tier)) {
                    true
                } else {
                    canceled_thumb_subscribers =
                        canceled_thumb_subscribers.saturating_add(subscribers.len());
                    for tx in subscribers.drain(..) {
                        let _ = tx.send(Err(LoadError::Canceled));
                    }
                    false
                }
            });
            cvar.notify_all();
            (removed_jobs, canceled_thumb_subscribers)
        };

        atomic_saturating_sub(&self.pending_jobs, removed_jobs);
        (removed_jobs, canceled_thumb_subscribers)
    }

    pub fn cancel_epoch_assets(
        &self,
        epoch: u64,
        asset_keys: &HashSet<u64>,
    ) -> (usize, usize, usize, usize) {
        if asset_keys.is_empty() {
            return (0, 0, 0, 0);
        }

        let (
            removed_jobs,
            removed_slot_jobs,
            canceled_slot_subscribers,
            canceled_thumb_subscribers,
        ) = {
            let (lock, cvar) = &*self.queue;
            let mut q = lock.lock().unwrap();

            let mut removed_jobs = 0usize;
            let mut removed_slot_jobs = 0usize;
            let mut canceled_slot_subscribers = 0usize;
            let mut canceled_thumb_subscribers = 0usize;
            let mut kept = BinaryHeap::with_capacity(q.heap.len());
            while let Some(job) = q.heap.pop() {
                let should_drop = match &job.key {
                    RequestKey::Thumbnail(key) => {
                        key.pipeline_version == epoch && asset_keys.contains(&key.asset_key)
                    }
                    RequestKey::CanvasMediaSlot(key) => {
                        key.pipeline_version == epoch && asset_keys.contains(&key.asset_key)
                    }
                };
                if should_drop {
                    removed_jobs = removed_jobs.saturating_add(1);
                    if matches!(job.key, RequestKey::CanvasMediaSlot(_)) {
                        removed_slot_jobs = removed_slot_jobs.saturating_add(1);
                    }
                } else {
                    kept.push(job);
                }
            }
            q.heap = kept;

            q.in_flight_canvas_media_slots.retain(|key, subscribers| {
                if key.pipeline_version != epoch || !asset_keys.contains(&key.asset_key) {
                    true
                } else {
                    canceled_slot_subscribers =
                        canceled_slot_subscribers.saturating_add(subscribers.len());
                    for tx in subscribers.drain(..) {
                        let _ = tx.send(Err(LoadError::Canceled));
                    }
                    false
                }
            });
            q.in_flight_thumbs.retain(|key, subscribers| {
                if key.pipeline_version != epoch || !asset_keys.contains(&key.asset_key) {
                    true
                } else {
                    canceled_thumb_subscribers =
                        canceled_thumb_subscribers.saturating_add(subscribers.len());
                    for tx in subscribers.drain(..) {
                        let _ = tx.send(Err(LoadError::Canceled));
                    }
                    false
                }
            });

            cvar.notify_all();
            (
                removed_jobs,
                removed_slot_jobs,
                canceled_slot_subscribers,
                canceled_thumb_subscribers,
            )
        };

        atomic_saturating_sub(&self.pending_jobs, removed_jobs);
        atomic_saturating_sub(&self.inflight_canvas_media_slots, removed_slot_jobs);
        (
            removed_jobs,
            removed_slot_jobs,
            canceled_slot_subscribers,
            canceled_thumb_subscribers,
        )
    }

    pub fn inflight_canvas_media_slots(&self) -> usize {
        self.inflight_canvas_media_slots
            .load(AtomicOrdering::Relaxed)
    }
}
