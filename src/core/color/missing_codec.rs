use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use super::MissingCodecKind;

fn missing_codec_set() -> &'static Mutex<HashSet<MissingCodecKind>> {
    static SET: OnceLock<Mutex<HashSet<MissingCodecKind>>> = OnceLock::new();
    SET.get_or_init(|| Mutex::new(HashSet::new()))
}

pub fn missing_codec_kinds() -> Vec<MissingCodecKind> {
    let Ok(set) = missing_codec_set().lock() else {
        return Vec::new();
    };
    let mut out: Vec<MissingCodecKind> = set.iter().copied().collect();
    out.sort_by_key(|kind| match kind {
        MissingCodecKind::Heif => 0,
        MissingCodecKind::Avif => 1,
        MissingCodecKind::Raw => 2,
        MissingCodecKind::Jpeg2000 => 3,
        MissingCodecKind::JpegXr => 4,
        MissingCodecKind::Generic => 5,
    });
    out
}

pub fn clear_missing_codec_kind(kind: MissingCodecKind) {
    if let Ok(mut set) = missing_codec_set().lock() {
        set.remove(&kind);
    }
}

pub(super) fn register_missing_codec(kind: MissingCodecKind) {
    if let Ok(mut set) = missing_codec_set().lock() {
        set.insert(kind);
    }
}
