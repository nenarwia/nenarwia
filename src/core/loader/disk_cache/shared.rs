use std::cell::RefCell;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::core::pack::{MediaPack, MediaPackReader, PackKind};

use super::paths::{cache_root, library_root};

static LIB_PACK_WRITER: OnceLock<Mutex<Option<MediaPack>>> = OnceLock::new();
static LIB_PACK_GEN: AtomicU64 = AtomicU64::new(1);

static RUNTIME_PACK_WRITER: OnceLock<Mutex<Option<MediaPack>>> = OnceLock::new();
static RUNTIME_PACK_GEN: AtomicU64 = AtomicU64::new(1);

static RUNTIME_USAGE_EPOCH: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static LIB_PACK_READER: RefCell<Option<(u64, MediaPackReader)>> = const { RefCell::new(None) };
    static RUNTIME_PACK_READER: RefCell<Option<(u64, MediaPackReader)>> = const { RefCell::new(None) };
}

pub(super) fn with_library_reader<F, R>(f: F) -> io::Result<R>
where
    F: FnOnce(&mut MediaPackReader) -> io::Result<R>,
{
    let gen = LIB_PACK_GEN.load(Ordering::Acquire);
    let root = library_root();

    LIB_PACK_READER.with(|cell| {
        let mut slot = cell.borrow_mut();
        let needs_open = match slot.as_ref() {
            Some((g, _)) => *g != gen,
            None => true,
        };
        if needs_open {
            let reader = MediaPackReader::open_read(&root)?;
            *slot = Some((gen, reader));
        }
        let (_, reader) = slot
            .as_mut()
            .ok_or_else(|| io::Error::other("library pack reader unavailable"))?;
        f(reader)
    })
}

pub(super) fn with_runtime_reader<F, R>(f: F) -> io::Result<R>
where
    F: FnOnce(&mut MediaPackReader) -> io::Result<R>,
{
    let gen = RUNTIME_PACK_GEN.load(Ordering::Acquire);
    let root = cache_root();

    RUNTIME_PACK_READER.with(|cell| {
        let mut slot = cell.borrow_mut();
        let needs_open = match slot.as_ref() {
            Some((g, _)) => *g != gen,
            None => true,
        };
        if needs_open {
            let reader = MediaPackReader::open_read(&root)?;
            *slot = Some((gen, reader));
        }
        let (_, reader) = slot
            .as_mut()
            .ok_or_else(|| io::Error::other("runtime pack reader unavailable"))?;
        f(reader)
    })
}

pub(super) fn with_runtime_writer<F, R>(f: F) -> io::Result<R>
where
    F: FnOnce(&mut MediaPack) -> io::Result<R>,
{
    let lock = RUNTIME_PACK_WRITER.get_or_init(|| Mutex::new(None));
    let mut guard = lock
        .lock()
        .map_err(|_| io::Error::other("runtime pack lock poisoned"))?;
    if guard.is_none() {
        *guard = Some(MediaPack::open(&cache_root(), PackKind::Runtime)?);
    }
    let pack = guard
        .as_mut()
        .ok_or_else(|| io::Error::other("runtime pack writer unavailable"))?;
    f(pack)
}

pub(super) fn close_library_handles() {
    if let Some(lock) = LIB_PACK_WRITER.get() {
        if let Ok(mut guard) = lock.lock() {
            *guard = None;
        }
    }
    LIB_PACK_READER.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

pub(super) fn close_runtime_handles() {
    if let Some(lock) = RUNTIME_PACK_WRITER.get() {
        if let Ok(mut guard) = lock.lock() {
            *guard = None;
        }
    }
    RUNTIME_PACK_READER.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

pub(super) fn bump_library_generation() {
    LIB_PACK_GEN.fetch_add(1, Ordering::AcqRel);
}

pub(super) fn bump_runtime_generation() {
    RUNTIME_PACK_GEN.fetch_add(1, Ordering::AcqRel);
}

pub(super) fn next_usage_epoch() -> u64 {
    RUNTIME_USAGE_EPOCH.fetch_add(1, Ordering::Relaxed)
}
