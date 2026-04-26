use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

const DEFAULT_MAX_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_RAM_MEDIA_SLOTS: usize = 36_000;

#[derive(Clone, Copy, Debug)]
pub enum CacheKind {
    Thumb,
    Tile,
}

#[derive(Clone, Debug)]
pub struct CacheKey {
    kind: CacheKind,
    asset_key: u64,
    size: u16,
    lod: u8,
    x: u32,
    y: u32,
}

impl CacheKey {
    pub fn thumb(asset_key: u64, size: u16) -> Self {
        Self {
            kind: CacheKind::Thumb,
            asset_key,
            size,
            lod: 0,
            x: 0,
            y: 0,
        }
    }

    pub fn tile(asset_key: u64, lod: u8, x: u32, y: u32) -> Self {
        Self {
            kind: CacheKind::Tile,
            asset_key,
            size: 0,
            lod,
            x,
            y,
        }
    }
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.kind as u8 == other.kind as u8
            && self.asset_key == other.asset_key
            && self.size == other.size
            && self.lod == other.lod
            && self.x == other.x
            && self.y == other.y
    }
}

impl Eq for CacheKey {}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.kind as u8).hash(state);
        self.asset_key.hash(state);
        self.size.hash(state);
        self.lod.hash(state);
        self.x.hash(state);
        self.y.hash(state);
    }
}

struct Entry {
    bytes: Vec<u8>,
    len: usize,
}

struct MemCache {
    map: HashMap<CacheKey, Entry>,
    lru: VecDeque<(CacheKey, u64)>,
    last_used: HashMap<CacheKey, u64>,
    usage_counter: u64,
    bytes: usize,
    max_bytes: usize,
    ram_media_slots: HashSet<u64>,
    max_ram_media_slots: usize,
}

impl MemCache {
    fn new(max_bytes: usize, max_ram_media_slots: usize) -> Self {
        Self {
            map: HashMap::new(),
            lru: VecDeque::new(),
            last_used: HashMap::new(),
            usage_counter: 0,
            bytes: 0,
            max_bytes,
            ram_media_slots: HashSet::new(),
            max_ram_media_slots,
        }
    }

    fn touch(&mut self, key: &CacheKey) {
        if self.map.contains_key(key) {
            self.usage_counter = self.usage_counter.wrapping_add(1);
            let stamp = self.usage_counter;
            self.last_used.insert(key.clone(), stamp);
            self.lru.push_back((key.clone(), stamp));
        }
    }

    fn get(&mut self, key: &CacheKey) -> Option<Vec<u8>> {
        let bytes = self.map.get(key).map(|entry| entry.bytes.clone())?;
        self.touch(key);
        Some(bytes)
    }

    fn is_ram_media_slot(&self, asset_key: u64) -> bool {
        self.ram_media_slots.contains(&asset_key)
    }

    fn insert(&mut self, key: CacheKey, bytes: Vec<u8>) {
        let len = bytes.len();
        if len == 0 || len > self.max_bytes {
            return;
        }

        if let Some(existing) = self.map.get(&key) {
            self.bytes = self.bytes.saturating_sub(existing.len);
        }

        self.bytes = self.bytes.saturating_add(len);
        self.map.insert(key.clone(), Entry { bytes, len });
        self.touch(&key);

        self.evict_if_needed();
    }

    fn evict_if_needed(&mut self) {
        while self.bytes > self.max_bytes {
            let Some((victim, stamp)) = self.lru.pop_front() else {
                break;
            };
            if self.last_used.get(&victim).copied() != Some(stamp) {
                continue;
            }
            if let Some(entry) = self.map.remove(&victim) {
                self.last_used.remove(&victim);
                self.bytes = self.bytes.saturating_sub(entry.len);
            }
        }
    }

    fn set_ram_media_slots(&mut self, asset_keys: &[u64]) {
        let mut slots = HashSet::with_capacity(self.max_ram_media_slots);
        for asset_key in asset_keys.iter().copied().take(self.max_ram_media_slots) {
            slots.insert(asset_key);
        }
        self.ram_media_slots = slots;
        self.prune_non_ram_entries();
    }

    fn prune_non_ram_entries(&mut self) {
        let mut drop_keys = Vec::new();
        for key in self.map.keys() {
            if !self.ram_media_slots.contains(&key.asset_key) {
                drop_keys.push(key.clone());
            }
        }
        for key in drop_keys {
            if let Some(entry) = self.map.remove(&key) {
                self.last_used.remove(&key);
                self.bytes = self.bytes.saturating_sub(entry.len);
            }
        }
    }
}

static MEM_CACHE: OnceLock<Mutex<MemCache>> = OnceLock::new();

fn cache() -> &'static Mutex<MemCache> {
    MEM_CACHE.get_or_init(|| Mutex::new(MemCache::new(DEFAULT_MAX_BYTES, MAX_RAM_MEDIA_SLOTS)))
}

pub fn get_thumb(asset_key: u64, size: u16) -> Option<Vec<u8>> {
    let key = CacheKey::thumb(asset_key, size);
    let mut cache = cache().lock().ok()?;
    if !cache.is_ram_media_slot(asset_key) {
        return None;
    }
    cache.get(&key)
}

pub fn get_tile(asset_key: u64, lod: u8, x: u32, y: u32) -> Option<Vec<u8>> {
    let key = CacheKey::tile(asset_key, lod, x, y);
    let mut cache = cache().lock().ok()?;
    if !cache.is_ram_media_slot(asset_key) {
        return None;
    }
    cache.get(&key)
}

pub fn get_canvas_media_slot(asset_key: u64, lod: u8, x: u32, y: u32) -> Option<Vec<u8>> {
    get_tile(asset_key, lod, x, y)
}

pub fn is_ram_media_slot_asset(asset_key: u64) -> bool {
    let Ok(cache) = cache().lock() else {
        return false;
    };
    cache.is_ram_media_slot(asset_key)
}

pub fn put_thumb(asset_key: u64, size: u16, bytes: Vec<u8>) {
    let key = CacheKey::thumb(asset_key, size);
    if let Ok(mut cache) = cache().lock() {
        if !cache.is_ram_media_slot(asset_key) {
            return;
        }
        cache.insert(key, bytes);
    }
}

pub fn put_tile(asset_key: u64, lod: u8, x: u32, y: u32, bytes: Vec<u8>) {
    let key = CacheKey::tile(asset_key, lod, x, y);
    if let Ok(mut cache) = cache().lock() {
        if !cache.is_ram_media_slot(asset_key) {
            return;
        }
        cache.insert(key, bytes);
    }
}

pub fn put_canvas_media_slot(asset_key: u64, lod: u8, x: u32, y: u32, bytes: Vec<u8>) {
    put_tile(asset_key, lod, x, y, bytes);
}

pub fn resident_media_slot_asset_keys() -> HashSet<u64> {
    let Ok(cache) = cache().lock() else {
        return HashSet::new();
    };
    cache.ram_media_slots.clone()
}

pub fn set_ram_media_slot_assets(asset_keys: &[u64]) {
    if let Ok(mut cache) = cache().lock() {
        cache.set_ram_media_slots(asset_keys);
    }
}

pub fn clear_ram_media_slot_assets() {
    if let Ok(mut cache) = cache().lock() {
        cache.set_ram_media_slots(&[]);
    }
}

pub fn remove_asset(asset_key: u64) {
    let Ok(mut cache) = cache().lock() else {
        return;
    };

    cache.ram_media_slots.remove(&asset_key);
    let drop_keys: Vec<_> = cache
        .map
        .keys()
        .filter(|key| key.asset_key == asset_key)
        .cloned()
        .collect();
    for key in drop_keys {
        if let Some(entry) = cache.map.remove(&key) {
            cache.last_used.remove(&key);
            cache.bytes = cache.bytes.saturating_sub(entry.len);
        }
    }
}
