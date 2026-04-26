use std::time::{Duration, Instant};

use winit::event_loop::{ControlFlow, EventLoopWindowTarget};
use winit::window::Window;

use crate::render::context::state::FramePacingMode;
use crate::render::context::RenderContext;

const UNLIMITED_FPS_CAP: u32 = 500;

pub fn set_idle_control_flow(elwt: &EventLoopWindowTarget<()>) {
    elwt.set_control_flow(ControlFlow::Wait);
}

pub fn request_redraw(ctx: &mut RenderContext) {
    ctx.mark_redraw_pending();
}

pub fn service_redraws(
    window: &Window,
    ctx: &mut RenderContext,
    elwt: &EventLoopWindowTarget<()>,
    continuous_redraw: bool,
) {
    if !continuous_redraw {
        ctx.clear_continuous_redraw_schedule();
    }
    if !continuous_redraw && !ctx.has_pending_redraw() {
        return;
    }

    if continuous_redraw {
        schedule_continuous_redraw(window, ctx, elwt);
        return;
    }

    if ctx.has_pending_redraw() {
        window.request_redraw();
        elwt.set_control_flow(ControlFlow::Wait);
        ctx.clear_pending_redraw();
    }
}

fn schedule_continuous_redraw(
    window: &Window,
    ctx: &mut RenderContext,
    elwt: &EventLoopWindowTarget<()>,
) {
    match continuous_redraw_plan(
        ctx.frame_pacing_mode,
        ctx.last_update_at,
        ctx.next_continuous_redraw_at,
        ctx.viewport.is_animating() || ctx.viewport.runtime().moving_recently,
        Instant::now(),
    ) {
        ContinuousRedrawPlan::RequestNow {
            control_flow,
            next_deadline,
        } => {
            window.request_redraw();
            elwt.set_control_flow(control_flow);
            ctx.next_continuous_redraw_at = next_deadline;
        }
        ContinuousRedrawPlan::WaitUntil(deadline) => {
            elwt.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
        ContinuousRedrawPlan::PollUntil(_) => {
            elwt.set_control_flow(ControlFlow::Poll);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContinuousRedrawPlan {
    RequestNow {
        control_flow: ControlFlow,
        next_deadline: Option<Instant>,
    },
    WaitUntil(Instant),
    PollUntil(Instant),
}

fn continuous_redraw_plan(
    mode: FramePacingMode,
    last_update_at: Option<Instant>,
    next_continuous_redraw_at: Option<Instant>,
    smooth_motion_hint: bool,
    now: Instant,
) -> ContinuousRedrawPlan {
    if mode.is_vsync() {
        return ContinuousRedrawPlan::RequestNow {
            control_flow: ControlFlow::Poll,
            next_deadline: None,
        };
    }
    debug_assert!(mode.is_unlimited());

    let next_redraw_at = next_unlimited_redraw_at(last_update_at, next_continuous_redraw_at, now);
    if next_redraw_at > now {
        if smooth_motion_hint {
            ContinuousRedrawPlan::PollUntil(next_redraw_at)
        } else {
            ContinuousRedrawPlan::WaitUntil(next_redraw_at)
        }
    } else {
        ContinuousRedrawPlan::RequestNow {
            control_flow: if smooth_motion_hint {
                ControlFlow::Poll
            } else {
                ControlFlow::Wait
            },
            next_deadline: Some(next_unlimited_deadline_after(next_redraw_at, now)),
        }
    }
}

fn next_unlimited_redraw_at(
    last_update_at: Option<Instant>,
    next_continuous_redraw_at: Option<Instant>,
    now: Instant,
) -> Instant {
    if let Some(next) = next_continuous_redraw_at {
        return next;
    }

    last_update_at
        .map(|last| (last + unlimited_frame_interval()).max(now))
        .unwrap_or(now)
}

fn next_unlimited_deadline_after(scheduled_at: Instant, now: Instant) -> Instant {
    let interval = unlimited_frame_interval();
    let next = scheduled_at + interval;
    if next > now {
        next
    } else {
        now + interval
    }
}

fn unlimited_frame_interval() -> Duration {
    Duration::from_micros((1_000_000 / UNLIMITED_FPS_CAP) as u64)
}

#[cfg(test)]
mod tests {
    use super::{continuous_redraw_plan, unlimited_frame_interval, ContinuousRedrawPlan};
    use crate::render::context::state::FramePacingMode;
    use std::time::Instant;
    use winit::event_loop::ControlFlow;

    #[test]
    fn vsync_continuous_redraw_requests_immediately_with_poll() {
        assert_eq!(
            continuous_redraw_plan(FramePacingMode::VSync, None, None, false, Instant::now()),
            ContinuousRedrawPlan::RequestNow {
                control_flow: ControlFlow::Poll,
                next_deadline: None,
            }
        );
    }

    #[test]
    fn unlimited_continuous_redraw_waits_until_software_cap() {
        let now = Instant::now();
        assert!(matches!(
            continuous_redraw_plan(
                FramePacingMode::Unlimited,
                Some(now),
                Some(now + unlimited_frame_interval()),
                false,
                now
            ),
            ContinuousRedrawPlan::WaitUntil(_)
        ));
    }

    #[test]
    fn unlimited_continuous_redraw_requests_when_cap_elapsed() {
        let now = Instant::now();
        let last = now - unlimited_frame_interval();
        assert_eq!(
            continuous_redraw_plan(
                FramePacingMode::Unlimited,
                Some(last),
                Some(now),
                false,
                now
            ),
            ContinuousRedrawPlan::RequestNow {
                control_flow: ControlFlow::Wait,
                next_deadline: Some(now + unlimited_frame_interval()),
            }
        );
    }

    #[test]
    fn unlimited_motion_prefers_poll_before_cap_elapses() {
        let now = Instant::now();
        assert_eq!(
            continuous_redraw_plan(
                FramePacingMode::Unlimited,
                Some(now),
                Some(now + unlimited_frame_interval()),
                true,
                now
            ),
            ContinuousRedrawPlan::PollUntil(now + unlimited_frame_interval())
        );
    }

    #[test]
    fn unlimited_motion_requests_with_poll_when_cap_elapsed() {
        let now = Instant::now();
        let last = now - unlimited_frame_interval();
        assert_eq!(
            continuous_redraw_plan(FramePacingMode::Unlimited, Some(last), Some(now), true, now),
            ContinuousRedrawPlan::RequestNow {
                control_flow: ControlFlow::Poll,
                next_deadline: Some(now + unlimited_frame_interval()),
            }
        );
    }
}
