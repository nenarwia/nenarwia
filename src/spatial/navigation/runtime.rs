use std::time::{Duration, Instant};

use crate::spatial::view::{ViewMetrics, ViewState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewMotionTier {
    Fast,
    Medium,
    Slow,
}

impl PreviewMotionTier {
    pub fn as_str(self) -> &'static str {
        match self {
            PreviewMotionTier::Fast => "fast",
            PreviewMotionTier::Medium => "medium",
            PreviewMotionTier::Slow => "slow",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ViewRuntimeConfig {
    pub zoom_reset_settle_frames: u64,
    pub zoom_reset_cooldown_frames: u64,
    pub preview_soft_reset_pan_delta_px: f32,
    pub preview_soft_reset_cooldown_frames: u64,
}

impl ViewRuntimeConfig {
    pub fn new(
        zoom_reset_settle_frames: u64,
        zoom_reset_cooldown_frames: u64,
        preview_soft_reset_pan_delta_px: f32,
        preview_soft_reset_cooldown_frames: u64,
    ) -> Self {
        Self {
            zoom_reset_settle_frames,
            zoom_reset_cooldown_frames,
            preview_soft_reset_pan_delta_px,
            preview_soft_reset_cooldown_frames,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ViewRuntimeUpdate {
    pub pan_changed: bool,
}

pub struct ViewRuntime {
    pub moving_recently: bool,
    pub preview_motion_tier: PreviewMotionTier,
    pub preview_motion_px_ema: f32,
    pub last_view: ViewState,
    pub last_changed_frame: u64,
    pub last_changed_at: Option<Instant>,
    pub last_zoom_changed_frame: u64,
    pub last_zoom_changed_at: Option<Instant>,
    pub zoom_reset_settle_frames: u64,
    pub zoom_reset_cooldown_frames: u64,
    pub last_zoom_reset_frame: u64,
    pub last_zoom_reset_at: Option<Instant>,
    pub last_soft_reset_center: (f64, f64),
    pub preview_soft_reset_pan_delta_px: f32,
    pub preview_soft_reset_cooldown_frames: u64,
    pub last_preview_soft_reset_frame: u64,
    pub last_preview_soft_reset_at: Option<Instant>,
}

impl ViewRuntime {
    pub fn new(initial_view: ViewState, config: ViewRuntimeConfig) -> Self {
        Self {
            moving_recently: false,
            preview_motion_tier: PreviewMotionTier::Slow,
            preview_motion_px_ema: 0.0,
            last_view: initial_view,
            last_changed_frame: 0,
            last_changed_at: None,
            last_zoom_changed_frame: 0,
            last_zoom_changed_at: None,
            zoom_reset_settle_frames: config.zoom_reset_settle_frames,
            zoom_reset_cooldown_frames: config.zoom_reset_cooldown_frames,
            last_zoom_reset_frame: 0,
            last_zoom_reset_at: None,
            last_soft_reset_center: (initial_view.center.x, initial_view.center.y),
            preview_soft_reset_pan_delta_px: config.preview_soft_reset_pan_delta_px,
            preview_soft_reset_cooldown_frames: config.preview_soft_reset_cooldown_frames,
            last_preview_soft_reset_frame: 0,
            last_preview_soft_reset_at: None,
        }
    }

    pub fn reset_for_loaded_view(
        &mut self,
        view: ViewState,
        frame_count: u64,
        awaiting_first_autoframe: bool,
    ) {
        self.preview_motion_px_ema = 0.0;
        self.preview_motion_tier = PreviewMotionTier::Slow;
        self.moving_recently = false;
        self.last_view = view;
        let now = Instant::now();
        self.last_changed_frame = if awaiting_first_autoframe {
            0
        } else {
            frame_count
        };
        self.last_changed_at = if awaiting_first_autoframe {
            None
        } else {
            Some(now)
        };
        self.last_zoom_changed_frame = if awaiting_first_autoframe {
            0
        } else {
            frame_count
        };
        self.last_zoom_changed_at = if awaiting_first_autoframe {
            None
        } else {
            Some(now)
        };
        self.last_zoom_reset_frame = frame_count;
        self.last_zoom_reset_at = Some(now);
        self.last_preview_soft_reset_frame = frame_count;
        self.last_preview_soft_reset_at = Some(now);
        self.last_soft_reset_center = (view.center.x, view.center.y);
    }

    pub fn update(
        &mut self,
        view: ViewState,
        metrics: ViewMetrics,
        frame_count: u64,
        frame_dt: Duration,
        now: Instant,
    ) -> ViewRuntimeUpdate {
        let pan_changed = (view.center.x - self.last_view.center.x).abs() > 1.0e-9
            || (view.center.y - self.last_view.center.y).abs() > 1.0e-9;
        let zoom_changed = (view.zoom - self.last_view.zoom).abs() > 1.0e-9;
        let moved = pan_changed || zoom_changed;

        let frame_pan_px = frame_pan_delta_px(metrics, self.last_view, view, frame_dt);
        let frame_zoom_px = frame_zoom_delta_px(metrics, self.last_view.zoom, view.zoom, frame_dt);
        let frame_motion_px = frame_pan_px.hypot(frame_zoom_px);
        let motion_alpha = preview_motion_ema_alpha(frame_dt);
        self.preview_motion_px_ema =
            self.preview_motion_px_ema * (1.0 - motion_alpha) + frame_motion_px * motion_alpha;
        self.preview_motion_tier =
            classify_preview_motion_tier(self.preview_motion_px_ema, self.preview_motion_tier);

        if moved {
            self.last_view = view;
            self.last_changed_frame = frame_count;
            self.last_changed_at = Some(now);
            if zoom_changed {
                self.last_zoom_changed_frame = frame_count;
                self.last_zoom_changed_at = Some(now);
            }
        } else {
            self.last_view = view;
        }

        self.moving_recently = self
            .last_changed_at
            .map(|changed_at| {
                now.saturating_duration_since(changed_at) <= self.moving_grace_duration()
            })
            .unwrap_or(false);

        ViewRuntimeUpdate { pan_changed }
    }

    pub fn moving_grace_duration(&self) -> Duration {
        Self::duration_for_reference_frames(2)
    }

    pub fn zoom_reset_settle_duration(&self) -> Duration {
        Self::duration_for_reference_frames(self.zoom_reset_settle_frames.max(1))
    }

    pub fn zoom_reset_cooldown_duration(&self) -> Duration {
        Self::duration_for_reference_frames(self.zoom_reset_cooldown_frames)
    }

    pub fn preview_soft_reset_cooldown_duration(&self) -> Duration {
        Self::duration_for_reference_frames(self.preview_soft_reset_cooldown_frames)
    }

    pub fn center_pan_delta_since_soft_reset_px(
        &self,
        metrics: ViewMetrics,
        view: ViewState,
    ) -> f32 {
        metrics.world_delta_to_pixel_distance(
            view.center.x - self.last_soft_reset_center.0,
            view.center.y - self.last_soft_reset_center.1,
        )
    }

    pub fn needs_settle_redraw(&self, view: ViewState, last_epoch_zoom: f64) -> bool {
        if self.moving_recently {
            return true;
        }

        let cur_bucket = view.zoom.max(1.0e-9).log2().floor() as i32;
        let last_bucket = last_epoch_zoom.max(1.0e-9).log2().floor() as i32;
        if cur_bucket == last_bucket {
            return false;
        }

        let now = Instant::now();
        let zoom_stable = self
            .last_zoom_changed_at
            .map(|changed_at| {
                now.saturating_duration_since(changed_at) >= self.zoom_reset_settle_duration()
            })
            .unwrap_or(true);
        if !zoom_stable {
            return true;
        }

        let cooldown_ready = self
            .last_zoom_reset_at
            .map(|reset_at| {
                now.saturating_duration_since(reset_at) >= self.zoom_reset_cooldown_duration()
            })
            .unwrap_or(true);
        !cooldown_ready
    }

    pub fn duration_for_reference_frames(frames: u64) -> Duration {
        Duration::from_secs_f64(frames as f64 / 60.0)
    }
}

fn parse_preview_motion_f32(var: &str, default: f32, min: f32, max: f32) -> f32 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .map(|v| v.clamp(min, max))
        .unwrap_or(default)
}

fn reference_time_scaled_alpha(alpha_at_60hz: f32, frame_dt: Duration) -> f32 {
    let dt_secs = frame_dt.as_secs_f32().max(1.0e-6);
    let remain = (1.0 - alpha_at_60hz).clamp(1.0e-6, 1.0 - 1.0e-6);
    (1.0 - remain.powf(dt_secs * 60.0)).clamp(0.0, 1.0)
}

fn reference_motion_rate(px_per_reference_frame: f32) -> f32 {
    px_per_reference_frame * 60.0
}

fn preview_motion_ema_alpha(frame_dt: Duration) -> f32 {
    use std::sync::OnceLock;
    static VAL: OnceLock<f32> = OnceLock::new();
    let alpha_at_60hz = *VAL.get_or_init(|| {
        parse_preview_motion_f32("CANVAS_PREVIEW_SPEED_EMA_ALPHA", 0.25, 0.05, 0.95)
    });
    reference_time_scaled_alpha(alpha_at_60hz, frame_dt)
}

fn preview_motion_medium_enter_px() -> f32 {
    use std::sync::OnceLock;
    static VAL: OnceLock<f32> = OnceLock::new();
    *VAL.get_or_init(|| {
        parse_preview_motion_f32("CANVAS_PREVIEW_SPEED_MEDIUM_ENTER_PX", 8.0, 1.0, 200.0)
    })
}

fn preview_motion_medium_exit_px() -> f32 {
    use std::sync::OnceLock;
    static VAL: OnceLock<f32> = OnceLock::new();
    *VAL.get_or_init(|| {
        parse_preview_motion_f32("CANVAS_PREVIEW_SPEED_MEDIUM_EXIT_PX", 5.0, 0.5, 200.0)
    })
}

fn preview_motion_fast_enter_px() -> f32 {
    use std::sync::OnceLock;
    static VAL: OnceLock<f32> = OnceLock::new();
    *VAL.get_or_init(|| {
        parse_preview_motion_f32("CANVAS_PREVIEW_SPEED_FAST_ENTER_PX", 28.0, 2.0, 400.0)
    })
}

fn preview_motion_fast_exit_px() -> f32 {
    use std::sync::OnceLock;
    static VAL: OnceLock<f32> = OnceLock::new();
    *VAL.get_or_init(|| {
        parse_preview_motion_f32("CANVAS_PREVIEW_SPEED_FAST_EXIT_PX", 20.0, 1.0, 400.0)
    })
}

fn classify_preview_motion_tier(ema_px: f32, prev: PreviewMotionTier) -> PreviewMotionTier {
    let medium_enter = reference_motion_rate(preview_motion_medium_enter_px());
    let medium_exit = reference_motion_rate(
        preview_motion_medium_exit_px().min(preview_motion_medium_enter_px()),
    );
    let fast_enter = reference_motion_rate(
        preview_motion_fast_enter_px().max(preview_motion_medium_enter_px() + 0.5),
    );
    let fast_exit = reference_motion_rate(
        preview_motion_fast_exit_px()
            .min(preview_motion_fast_enter_px().max(preview_motion_medium_enter_px() + 0.5)),
    );

    match prev {
        PreviewMotionTier::Fast => {
            if ema_px >= fast_exit {
                PreviewMotionTier::Fast
            } else if ema_px >= medium_exit {
                PreviewMotionTier::Medium
            } else {
                PreviewMotionTier::Slow
            }
        }
        PreviewMotionTier::Medium => {
            if ema_px >= fast_enter {
                PreviewMotionTier::Fast
            } else if ema_px >= medium_exit {
                PreviewMotionTier::Medium
            } else {
                PreviewMotionTier::Slow
            }
        }
        PreviewMotionTier::Slow => {
            if ema_px >= fast_enter {
                PreviewMotionTier::Fast
            } else if ema_px >= medium_enter {
                PreviewMotionTier::Medium
            } else {
                PreviewMotionTier::Slow
            }
        }
    }
}

fn frame_pan_delta_px(
    metrics: ViewMetrics,
    prev_view: ViewState,
    current_view: ViewState,
    frame_dt: Duration,
) -> f32 {
    let dt_secs = frame_dt.as_secs_f32();
    if dt_secs <= 0.0 {
        return 0.0;
    }

    metrics.world_delta_to_pixel_distance(
        current_view.center.x - prev_view.center.x,
        current_view.center.y - prev_view.center.y,
    ) / dt_secs
}

fn frame_zoom_delta_px(
    metrics: ViewMetrics,
    prev_zoom: f64,
    current_zoom: f64,
    frame_dt: Duration,
) -> f32 {
    let dt_secs = frame_dt.as_secs_f32();
    if dt_secs <= 0.0 {
        return 0.0;
    }

    let prev = prev_zoom.max(1.0e-9);
    let curr = current_zoom.max(1.0e-9);
    let octaves = (curr / prev).log2().abs() as f32;
    let surface = metrics.surface_size();
    let ref_px = surface.width.min(surface.height).max(1) as f32;
    (octaves * ref_px * 0.75) / dt_secs
}
