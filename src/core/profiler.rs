use std::time::{Duration, Instant};

pub struct Profiler {
    last_update: Instant,
    frame_count: u32,
    pub fps: u32,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
            frame_count: 0,
            fps: 0,
        }
    }
    pub fn tick(&mut self) -> bool {
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);

        if elapsed >= Duration::from_secs(1) {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.last_update = now;
            true
        } else {
            false
        }
    }
}
