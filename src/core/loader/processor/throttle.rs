use std::sync::{Condvar, Mutex, OnceLock};

use super::max_decode_jobs_value;

struct DecodeLimiter {
    active: Mutex<usize>,
    cv: Condvar,
}

fn decode_limiter() -> &'static DecodeLimiter {
    static LIMITER: OnceLock<DecodeLimiter> = OnceLock::new();
    LIMITER.get_or_init(|| DecodeLimiter {
        active: Mutex::new(0),
        cv: Condvar::new(),
    })
}

pub(super) struct DecodeGuard;

impl Drop for DecodeGuard {
    fn drop(&mut self) {
        let limiter = decode_limiter();
        if let Ok(mut active) = limiter.active.lock() {
            if *active > 0 {
                *active -= 1;
            }
            limiter.cv.notify_one();
        }
    }
}

pub(super) fn acquire_decode_guard() -> DecodeGuard {
    let max_jobs = max_decode_jobs_value().max(1);
    let limiter = decode_limiter();
    let mut active = limiter.active.lock().unwrap();
    while *active >= max_jobs {
        active = limiter.cv.wait(active).unwrap();
    }
    *active += 1;
    DecodeGuard
}
