use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

pub(super) fn probe_ext_once(ext: &str) -> bool {
    static PROBED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let mut set = match PROBED.get_or_init(|| Mutex::new(HashSet::new())).lock() {
        Ok(s) => s,
        Err(_) => return false,
    };
    if set.contains(ext) {
        return false;
    }
    set.insert(ext.to_string());
    true
}

pub(super) fn warn_once(key: &str) -> bool {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let mut set = match WARNED.get_or_init(|| Mutex::new(HashSet::new())).lock() {
        Ok(s) => s,
        Err(_) => return true,
    };
    if set.contains(key) {
        return false;
    }
    set.insert(key.to_string());
    true
}
