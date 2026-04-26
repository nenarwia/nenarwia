use crate::render::context::state::RenderContext;
use crate::render::instance::InstanceRaw;
use crate::render::streaming::canvas_media_slots::calculator::lod_info;
use crate::render::streaming::feedback::FeedbackInstance;

const SLOT_BACKDROP_DISK_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.4];
const SLOT_BACKDROP_RAM_COLOR: [f32; 4] = [0.62, 0.9, 0.25, 0.4];

pub(super) fn update_slot_backdrop_buffer(ctx: &mut RenderContext) {
    if !ctx.draw_assembly.slot_backdrop_dirty {
        return;
    }

    let backdrop_required = if ctx.debug_slot_backdrop_enabled {
        ctx.scene.all_items_raw.len()
    } else if ctx.scene.total_count > 0 {
        1
    } else {
        0
    };

    ensure_slot_backdrop_capacity(ctx, backdrop_required.max(1));
    ctx.draw_assembly.slot_backdrop_instances.clear();

    if backdrop_required > ctx.draw_assembly.slot_backdrop_instances.capacity() {
        ctx.draw_assembly
            .slot_backdrop_instances
            .reserve(backdrop_required - ctx.draw_assembly.slot_backdrop_instances.capacity());
    }

    let view = ctx.view();
    let render_origin = (view.center.x, view.center.y);

    if ctx.debug_slot_backdrop_enabled {
        let ram_assets = crate::core::loader::mem_cache::resident_media_slot_asset_keys();
        let backdrop_instances: Vec<_> = ctx
            .scene
            .all_items_raw
            .iter()
            .enumerate()
            .filter_map(|(idx, _)| {
                let raw = ctx.scene.item_draw_raw(idx, render_origin)?;
                if raw.data[2] <= 0.0 || raw.data[3] <= 0.0 {
                    return None;
                }
                let asset_key = ctx.scene.asset_keys.get(idx).copied().unwrap_or(0);
                let color = if ram_assets.contains(&asset_key) {
                    SLOT_BACKDROP_RAM_COLOR
                } else {
                    SLOT_BACKDROP_DISK_COLOR
                };
                Some(backdrop_instance(raw.data, color))
            })
            .collect();

        ctx.draw_assembly
            .slot_backdrop_instances
            .extend(backdrop_instances);
    } else if let Some([min_x, min_y, max_x, max_y]) = ctx.scene.layout_bounds() {
        push_bounds_backdrop(
            &mut ctx.draw_assembly.slot_backdrop_instances,
            [
                min_x - render_origin.0 as f32,
                min_y - render_origin.1 as f32,
                max_x - render_origin.0 as f32,
                max_y - render_origin.1 as f32,
            ],
            SLOT_BACKDROP_DISK_COLOR,
        );
    }

    ctx.draw_assembly.slot_backdrop_count = ctx.draw_assembly.slot_backdrop_instances.len() as u32;
    if ctx.draw_assembly.slot_backdrop_count > 0 {
        ctx.gpu.queue.write_buffer(
            &ctx.draw_assembly.slot_backdrop_buffer,
            0,
            bytemuck::cast_slice(&ctx.draw_assembly.slot_backdrop_instances),
        );
    }

    ctx.draw_assembly.slot_backdrop_dirty = false;
}

fn backdrop_instance(data: [f32; 4], color: [f32; 4]) -> InstanceRaw {
    InstanceRaw {
        data,
        color,
        uv_region: [0.0, 0.0, 0.0, 0.0],
        params: [-1.0, -1.0, 0.0, 0.0],
        params2: [-1.0, -1.0, 0.0, 0.0],
        sample_flags: [0.0, 0.0, 0.0, 0.0],
        fit_rect: InstanceRaw::FULL_SLOT_FIT_RECT,
    }
}

fn push_bounds_backdrop(instances: &mut Vec<InstanceRaw>, bounds: [f32; 4], color: [f32; 4]) {
    let width = (bounds[2] - bounds[0]).max(0.0);
    let height = (bounds[3] - bounds[1]).max(0.0);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let center_x = (bounds[0] + bounds[2]) * 0.5;
    let center_y = (bounds[1] + bounds[3]) * 0.5;
    instances.push(backdrop_instance(
        [center_x, center_y, width, height],
        color,
    ));
}

pub(super) fn update_visible_buffer(ctx: &mut RenderContext) {
    let count = ctx.committed_view.visible_items.len();
    if count == 0 {
        ctx.draw_assembly.clear_draw_instances();
        return;
    }

    ensure_visible_capacity(ctx, count);
    ensure_feedback_instance_capacity(ctx, count);

    ctx.draw_assembly.visible_instances.clear();
    ctx.draw_assembly.feedback_instances.clear();
    let view = ctx.view();
    let render_origin = (view.center.x, view.center.y);

    let cap = ctx.draw_assembly.visible_instances.capacity();
    if count > cap {
        ctx.draw_assembly.visible_instances.reserve(count - cap);
    }
    let fcap = ctx.draw_assembly.feedback_instances.capacity();
    if count > fcap {
        ctx.draw_assembly.feedback_instances.reserve(count - fcap);
    }

    for item in ctx.committed_view.visible_items.iter().copied() {
        let idx = item.idx;
        if idx >= ctx.scene.all_items_raw.len() {
            continue;
        }
        let Some(scene_inst) = ctx.scene.item_draw_raw(idx, render_origin) else {
            continue;
        };

        if !super::slot_has_media_content(scene_inst) {
            continue;
        }

        let mut inst = scene_inst;
        inst.color = [1.0, 1.0, 1.0, 1.0];
        ctx.draw_assembly.visible_instances.push(inst);

        let asset_key = ctx.scene.asset_keys.get(idx).copied().unwrap_or(0);
        let mut desired_lod = ctx.scene.last_lod.get(idx).copied().unwrap_or(0) as u32;
        let mut tiles_x_f = 0.0;
        let mut tiles_y_f = 0.0;
        if inst.params[0] >= 0.0 {
            let (orig_w, orig_h) = ctx
                .scene
                .item_dimensions
                .get(idx)
                .copied()
                .unwrap_or((0, 0));
            if orig_w > 0 && orig_h > 0 && desired_lod <= u8::MAX as u32 {
                let (_, _, _tx, _ty, tx_f, ty_f) = lod_info(orig_w, orig_h, desired_lod as u8);
                tiles_x_f = tx_f;
                tiles_y_f = ty_f;
            } else {
                desired_lod = u32::MAX;
            }
        } else {
            desired_lod = u32::MAX;
        }
        ctx.draw_assembly
            .feedback_instances
            .push(FeedbackInstance::new(
                asset_key,
                desired_lod,
                tiles_x_f,
                tiles_y_f,
            ));
    }

    ctx.draw_assembly.visible_count = ctx.draw_assembly.visible_instances.len() as u32;
    if ctx.draw_assembly.visible_count > 0 {
        ctx.gpu.queue.write_buffer(
            &ctx.draw_assembly.visible_buffer,
            0,
            bytemuck::cast_slice(&ctx.draw_assembly.visible_instances),
        );
        if !ctx.draw_assembly.feedback_instances.is_empty() {
            ctx.gpu.queue.write_buffer(
                &ctx.draw_assembly.feedback_instance_buffer,
                0,
                bytemuck::cast_slice(&ctx.draw_assembly.feedback_instances),
            );
        }
    }
}

fn ensure_visible_capacity(ctx: &mut RenderContext, required: usize) {
    if required <= ctx.draw_assembly.visible_capacity {
        return;
    }

    let new_cap = required.next_power_of_two().max(1);
    ctx.draw_assembly.visible_capacity = new_cap;
    ctx.draw_assembly.visible_buffer = ctx.gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Visible Instances Buffer"),
        size: (new_cap * std::mem::size_of::<crate::render::instance::InstanceRaw>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
}

fn ensure_slot_backdrop_capacity(ctx: &mut RenderContext, required: usize) {
    if required <= ctx.draw_assembly.slot_backdrop_capacity {
        return;
    }

    let new_cap = required.next_power_of_two().max(1);
    ctx.draw_assembly.slot_backdrop_capacity = new_cap;
    ctx.draw_assembly.slot_backdrop_buffer =
        ctx.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Slot Backdrop Buffer"),
            size: (new_cap * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
}

fn ensure_feedback_instance_capacity(ctx: &mut RenderContext, required: usize) {
    if required <= ctx.draw_assembly.feedback_instance_capacity {
        return;
    }

    let new_cap = required.next_power_of_two().max(1);
    ctx.draw_assembly.feedback_instance_capacity = new_cap;
    ctx.draw_assembly.feedback_instance_buffer =
        ctx.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Feedback Instance Buffer"),
            size: (new_cap * std::mem::size_of::<FeedbackInstance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    if let Some(fb) = ctx.gpu_feedback.as_ref() {
        ctx.draw_assembly.feedback_instance_bind_group = fb.create_instance_bind_group(
            &ctx.gpu.device,
            &ctx.draw_assembly.feedback_instance_buffer,
        );
        ctx.draw_assembly.feedback_collect_buf_bind_group = fb.create_collect_buf_bind_group(
            &ctx.gpu.device,
            &ctx.draw_assembly.feedback_instance_buffer,
        );
    }
}
