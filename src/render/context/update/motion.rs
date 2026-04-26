use std::time::Duration;

use crate::render::context::state::{PreviewMotionTier, RenderContext};

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

pub(super) fn preview_motion_ema_alpha(frame_dt: Duration) -> f32 {
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

pub(super) fn classify_preview_motion_tier(
    ema_px: f32,
    prev: PreviewMotionTier,
) -> PreviewMotionTier {
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

pub(super) fn frame_pan_delta_px(
    ctx: &RenderContext,
    cam_x: f64,
    cam_y: f64,
    cam_z: f64,
    frame_dt: Duration,
) -> f32 {
    let dt_secs = frame_dt.as_secs_f32();
    if dt_secs <= 0.0 {
        return 0.0;
    }

    let dx_world = (cam_x - ctx.last_camera_eye.0).abs() as f32;
    let dy_world = (cam_y - ctx.last_camera_eye.1).abs() as f32;
    if dx_world <= 0.0 && dy_world <= 0.0 {
        return 0.0;
    }

    let zoom = cam_z.max(1e-6) as f32;
    let viewport_world_w = 2.0 * ctx.camera.aspect as f32 / zoom;
    let viewport_world_h = 2.0 / zoom;
    let px_per_world_x = ctx.gpu.size.width.max(1) as f32 / viewport_world_w.max(1e-6);
    let px_per_world_y = ctx.gpu.size.height.max(1) as f32 / viewport_world_h.max(1e-6);
    let dx_px = dx_world * px_per_world_x;
    let dy_px = dy_world * px_per_world_y;
    dx_px.hypot(dy_px) / dt_secs
}

pub(super) fn frame_zoom_delta_px(ctx: &RenderContext, cam_z: f64, frame_dt: Duration) -> f32 {
    let dt_secs = frame_dt.as_secs_f32();
    if dt_secs <= 0.0 {
        return 0.0;
    }

    let prev = ctx.last_camera_zoom.max(1e-9);
    let curr = cam_z.max(1e-9);
    let octaves = (curr / prev).log2().abs() as f32;
    let ref_px = ctx.gpu.size.width.min(ctx.gpu.size.height).max(1) as f32;
    (octaves * ref_px * 0.75) / dt_secs
}
