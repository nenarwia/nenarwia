use std::sync::atomic::{AtomicU8, Ordering};

use winit::window::ResizeDirection;

pub(super) static NATIVE_CURSOR_MODE: AtomicU8 = AtomicU8::new(CURSOR_MODE_CUSTOM);

pub(super) const CURSOR_MODE_CUSTOM: u8 = 0;
pub(super) const CURSOR_MODE_RESIZE_EW: u8 = 1;
pub(super) const CURSOR_MODE_RESIZE_NS: u8 = 2;
pub(super) const CURSOR_MODE_RESIZE_NESW: u8 = 3;
pub(super) const CURSOR_MODE_RESIZE_NWSE: u8 = 4;

pub fn native_cursor_resize_direction() -> Option<ResizeDirection> {
    decode_resize_direction(current_mode())
}

pub(super) fn current_mode() -> u8 {
    NATIVE_CURSOR_MODE.load(Ordering::Relaxed)
}

pub(super) fn set_custom_mode() {
    NATIVE_CURSOR_MODE.store(CURSOR_MODE_CUSTOM, Ordering::Relaxed);
}

pub(super) fn set_resize_mode(direction: ResizeDirection) {
    NATIVE_CURSOR_MODE.store(encode_resize_direction(direction), Ordering::Relaxed);
}

fn encode_resize_direction(direction: ResizeDirection) -> u8 {
    match direction {
        ResizeDirection::East | ResizeDirection::West => CURSOR_MODE_RESIZE_EW,
        ResizeDirection::North | ResizeDirection::South => CURSOR_MODE_RESIZE_NS,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CURSOR_MODE_RESIZE_NESW,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CURSOR_MODE_RESIZE_NWSE,
    }
}

fn decode_resize_direction(mode: u8) -> Option<ResizeDirection> {
    match mode {
        CURSOR_MODE_RESIZE_EW => Some(ResizeDirection::East),
        CURSOR_MODE_RESIZE_NS => Some(ResizeDirection::North),
        CURSOR_MODE_RESIZE_NESW => Some(ResizeDirection::NorthEast),
        CURSOR_MODE_RESIZE_NWSE => Some(ResizeDirection::NorthWest),
        _ => None,
    }
}
