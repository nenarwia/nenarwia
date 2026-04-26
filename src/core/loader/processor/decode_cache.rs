use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Instant;

use crate::core::{color, metrics};

use super::settings::{
    asset_key_or_hash, decode_cache_byte_limit, decode_cache_item_limit, is_jpeg,
    lod_cache_byte_limit, lod_cache_item_limit,
};
use super::throttle::acquire_decode_guard;
use super::WorkerState;

#[derive(Clone)]
struct DecodedLod {
    rgba: Arc<color::RgbaImage>,
    width: u32,
    height: u32,
}

#[derive(Clone)]
struct DecodedFull {
    rgba: Arc<color::RgbaImage>,
    width: u32,
    height: u32,
    bytes: u64,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct DecodeKey {
    asset_key: u64,
    len: u64,
    modified_ms: u64,
}

struct DecodeCache {
    map: HashMap<DecodeKey, Arc<DecodedFull>>,
    lru: VecDeque<(DecodeKey, u64)>,
    last_used: HashMap<DecodeKey, u64>,
    usage_counter: u64,
    bytes: u64,
}

impl DecodeCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            lru: VecDeque::new(),
            last_used: HashMap::new(),
            usage_counter: 0,
            bytes: 0,
        }
    }

    fn touch(&mut self, key: &DecodeKey) {
        if self.map.contains_key(key) {
            self.usage_counter = self.usage_counter.wrapping_add(1);
            let stamp = self.usage_counter;
            self.last_used.insert(*key, stamp);
            self.lru.push_back((*key, stamp));
        }
    }

    fn get(&mut self, key: &DecodeKey) -> Option<Arc<DecodedFull>> {
        let entry = self.map.get(key).cloned()?;
        self.touch(key);
        Some(entry)
    }

    fn insert(&mut self, key: DecodeKey, entry: Arc<DecodedFull>) {
        if let Some(prev) = self.map.insert(key, entry.clone()) {
            self.bytes = self.bytes.saturating_sub(prev.bytes);
        }
        self.bytes = self.bytes.saturating_add(entry.bytes);
        self.touch(&key);
        self.evict_if_needed();
    }

    fn evict_if_needed(&mut self) {
        let item_limit = decode_cache_item_limit();
        let byte_limit = decode_cache_byte_limit();
        while !self.map.is_empty() && (self.map.len() > item_limit || self.bytes > byte_limit) {
            let Some((victim, stamp)) = self.lru.pop_front() else {
                break;
            };
            if self.last_used.get(&victim).copied() != Some(stamp) {
                continue;
            }
            if let Some(entry) = self.map.remove(&victim) {
                self.last_used.remove(&victim);
                self.bytes = self.bytes.saturating_sub(entry.bytes);
            }
        }
    }
}

static DECODE_CACHE: OnceLock<Mutex<DecodeCache>> = OnceLock::new();

fn decode_cache() -> &'static Mutex<DecodeCache> {
    DECODE_CACHE.get_or_init(|| Mutex::new(DecodeCache::new()))
}

struct Flight<T> {
    state: Mutex<FlightState<T>>,
    cvar: Condvar,
}

enum FlightState<T> {
    Pending,
    Done(T),
}

impl<T: Clone> Flight<T> {
    fn new() -> Self {
        Self {
            state: Mutex::new(FlightState::Pending),
            cvar: Condvar::new(),
        }
    }

    fn wait(&self) -> T {
        let mut guard = self.state.lock().unwrap();
        loop {
            match &*guard {
                FlightState::Pending => {
                    guard = self.cvar.wait(guard).unwrap();
                }
                FlightState::Done(v) => return v.clone(),
            }
        }
    }

    fn finish(&self, value: T) {
        let mut guard = self.state.lock().unwrap();
        *guard = FlightState::Done(value);
        self.cvar.notify_all();
    }
}

type DecodeFlight = Flight<Result<Arc<DecodedFull>, ()>>;
type DecodeInflightMap = HashMap<DecodeKey, Arc<DecodeFlight>>;

static DECODE_INFLIGHT: OnceLock<Mutex<DecodeInflightMap>> = OnceLock::new();

fn decode_inflight() -> &'static Mutex<DecodeInflightMap> {
    DECODE_INFLIGHT.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct LodKey {
    asset_key: u64,
    lod: u8,
}

type LodFlight = Flight<Result<DecodedLod, ()>>;
type LodInflightMap = HashMap<LodKey, Arc<LodFlight>>;

static LOD_INFLIGHT: OnceLock<Mutex<LodInflightMap>> = OnceLock::new();

fn lod_inflight() -> &'static Mutex<LodInflightMap> {
    LOD_INFLIGHT.get_or_init(|| Mutex::new(HashMap::new()))
}

struct LodCache {
    map: HashMap<LodKey, DecodedLod>,
    lru: Vec<LodKey>,
    bytes: u64,
}

static LOD_CACHE: OnceLock<Mutex<LodCache>> = OnceLock::new();

fn lod_cache() -> &'static Mutex<LodCache> {
    LOD_CACHE.get_or_init(|| {
        Mutex::new(LodCache {
            map: HashMap::new(),
            lru: Vec::new(),
            bytes: 0,
        })
    })
}

fn lod_cache_key(asset_key: u64, path: &Path, lod: u8) -> LodKey {
    let key_asset = asset_key_or_hash(asset_key, path);
    LodKey {
        asset_key: key_asset,
        lod,
    }
}

fn lod_cache_get(key: LodKey) -> Option<DecodedLod> {
    let mut cache = lod_cache().lock().ok()?;
    let entry = cache.map.get(&key).cloned()?;
    if let Some(pos) = cache.lru.iter().position(|k| *k == key) {
        cache.lru.remove(pos);
    }
    cache.lru.insert(0, key);
    Some(entry)
}

fn lod_cache_insert(key: LodKey, entry: DecodedLod) {
    let mut cache = match lod_cache().lock() {
        Ok(v) => v,
        Err(_) => return,
    };
    if let Some(prev) = cache.map.insert(key, entry.clone()) {
        cache.bytes = cache.bytes.saturating_sub(decoded_lod_bytes(&prev));
    }
    cache.bytes = cache.bytes.saturating_add(decoded_lod_bytes(&entry));
    if let Some(pos) = cache.lru.iter().position(|k| *k == key) {
        cache.lru.remove(pos);
    }
    cache.lru.insert(0, key);

    let item_limit = lod_cache_item_limit();
    let byte_limit = lod_cache_byte_limit();
    while !cache.map.is_empty() && (cache.map.len() > item_limit || cache.bytes > byte_limit) {
        if let Some(evict_key) = cache.lru.pop() {
            if let Some(evicted) = cache.map.remove(&evict_key) {
                cache.bytes = cache.bytes.saturating_sub(decoded_lod_bytes(&evicted));
            }
        } else {
            break;
        }
    }
}

pub(super) fn decode_cached(
    st: &mut WorkerState,
    asset_key: u64,
    path: &Path,
) -> Option<(Arc<color::RgbaImage>, u32, u32)> {
    let _ = st;
    let (len, modified_ms) = super::settings::file_meta(path)?;
    let key_asset = asset_key_or_hash(asset_key, path);
    let key = DecodeKey {
        asset_key: key_asset,
        len,
        modified_ms,
    };

    if let Ok(mut cache) = decode_cache().lock() {
        if let Some(entry) = cache.get(&key) {
            metrics::record_decode_cache_hit();
            metrics::set_decode_cache_items(cache.map.len() as u64);
            metrics::set_decode_cache_bytes(cache.bytes);
            return Some((entry.rgba.clone(), entry.width, entry.height));
        }
    }

    let (flight, should_decode) = {
        let mut map = decode_inflight().lock().ok()?;
        if let Some(f) = map.get(&key) {
            metrics::record_decode_cache_hit();
            (f.clone(), false)
        } else {
            metrics::record_decode_cache_miss();
            let f = Arc::new(Flight::new());
            map.insert(key, f.clone());
            (f, true)
        }
    };

    if !should_decode {
        let result = flight.wait();
        return result
            .ok()
            .map(|entry| (entry.rgba.clone(), entry.width, entry.height));
    }

    let result = (|| {
        let _decode_guard = acquire_decode_guard();
        let decoded = if is_jpeg(path) {
            match color::decode_jpeg_scaled(path, u32::MAX, u32::MAX) {
                Ok(v) => v,
                Err(_) => color::decode_rgba8_srgb(path).ok()?,
            }
        } else {
            color::decode_rgba8_srgb(path).ok()?
        };
        metrics::record_io_read(len);
        let rgba = Arc::new(decoded.rgba);
        let bytes = (decoded.width as u64)
            .saturating_mul(decoded.height as u64)
            .saturating_mul(4);
        let entry = Arc::new(DecodedFull {
            rgba,
            width: decoded.width,
            height: decoded.height,
            bytes,
        });

        if let Ok(mut cache) = decode_cache().lock() {
            cache.insert(key, entry.clone());
            metrics::set_decode_cache_items(cache.map.len() as u64);
            metrics::set_decode_cache_bytes(cache.bytes);
        }

        Some(entry)
    })();

    let out = result.ok_or(());
    flight.finish(out.clone());
    if let Ok(mut map) = decode_inflight().lock() {
        map.remove(&key);
    }
    out.ok()
        .map(|entry| (entry.rgba.clone(), entry.width, entry.height))
}

pub(super) fn decode_lod_cached(
    asset_key: u64,
    lod: u8,
    path: &Path,
    lod_w: u32,
    lod_h: u32,
) -> Option<(Arc<color::RgbaImage>, u32, u32)> {
    let key = lod_cache_key(asset_key, path, lod);
    if let Some(entry) = lod_cache_get(key) {
        return Some((entry.rgba, entry.width, entry.height));
    }
    let (flight, should_decode) = {
        let mut map = lod_inflight().lock().ok()?;
        if let Some(f) = map.get(&key) {
            (f.clone(), false)
        } else {
            let f = Arc::new(Flight::new());
            map.insert(key, f.clone());
            (f, true)
        }
    };

    if !should_decode {
        let result = flight.wait();
        return result
            .ok()
            .map(|entry| (entry.rgba, entry.width, entry.height));
    }

    let result = (|| {
        let len = fs::metadata(path).ok().map(|m| m.len()).unwrap_or(0);
        let _decode_guard = acquire_decode_guard();
        let mut decoded = if let Ok(Some(wic)) = color::decode_wic_scaled(path, lod_w, lod_h) {
            wic
        } else if is_jpeg(path) {
            color::decode_jpeg_scaled(path, lod_w, lod_h).ok()?
        } else {
            color::decode_rgba8_srgb(path).ok()?
        };
        metrics::record_io_read(len);

        if decoded.width != lod_w || decoded.height != lod_h {
            let mut resized = None;
            if color::gpu_resize_should_use(decoded.width, decoded.height, lod_w, lod_h) {
                metrics::record_gpu_resize_job();
                let gpu_start = Instant::now();
                if let Some(bytes) = color::resize_rgba8_srgb_gpu(
                    decoded.rgba.as_raw(),
                    decoded.width,
                    decoded.height,
                    lod_w,
                    lod_h,
                ) {
                    if let Some(img) = color::RgbaImage::from_raw(lod_w, lod_h, bytes) {
                        resized = Some(img);
                        metrics::record_gpu_resize_ok(gpu_start.elapsed().as_millis() as u64);
                    } else {
                        metrics::record_gpu_resize_fallback_cpu();
                    }
                } else {
                    metrics::record_gpu_resize_fallback_cpu();
                }
            }
            decoded.rgba = if let Some(img) = resized {
                img
            } else {
                color::resize_linear_rgba8_exact(&decoded.rgba, lod_w, lod_h)
            };
            decoded.width = lod_w;
            decoded.height = lod_h;
        }

        let rgba = Arc::new(decoded.rgba);
        let entry = DecodedLod {
            rgba: rgba.clone(),
            width: decoded.width,
            height: decoded.height,
        };
        lod_cache_insert(key, entry.clone());
        Some(entry)
    })();

    let out = result.ok_or(());
    flight.finish(out.clone());
    if let Ok(mut map) = lod_inflight().lock() {
        map.remove(&key);
    }
    out.ok()
        .map(|entry| (entry.rgba, entry.width, entry.height))
}

fn decoded_lod_bytes(d: &DecodedLod) -> u64 {
    (d.width as u64)
        .saturating_mul(d.height as u64)
        .saturating_mul(4)
}
