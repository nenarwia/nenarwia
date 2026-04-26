const DEFAULT_PIXEL_PERFECT: bool = false;
const DEFAULT_MAG_FILTER_MODE: u8 = 1; // 0=linear, 1=mitchell, 2=catmull

pub(crate) fn is_undersampled(obj_px_w: f32, obj_px_h: f32, src_w: u32, src_h: u32) -> bool {
    if src_w == 0 || src_h == 0 {
        return true;
    }
    obj_px_w > src_w as f32 || obj_px_h > src_h as f32
}

fn pixel_perfect_enabled() -> bool {
    use std::sync::OnceLock;
    static PIXEL_PERFECT: OnceLock<bool> = OnceLock::new();
    *PIXEL_PERFECT.get_or_init(|| {
        let value = std::env::var("CANVAS_PIXEL_PERFECT")
            .or_else(|_| std::env::var("CANVAS_STRICT_NO_BLUR"))
            .unwrap_or_default()
            .to_lowercase();
        if value.is_empty() {
            return DEFAULT_PIXEL_PERFECT;
        }
        matches!(value.as_str(), "1" | "true" | "yes" | "on")
    })
}

fn mag_filter_mode() -> u8 {
    use std::sync::OnceLock;
    static MAG_FILTER: OnceLock<u8> = OnceLock::new();
    *MAG_FILTER.get_or_init(|| {
        let value = std::env::var("CANVAS_MAG_FILTER")
            .unwrap_or_else(|_| "mitchell".to_string())
            .to_lowercase();
        match value.as_str() {
            "linear" => 0,
            "mitchell" => 1,
            "catmull" => 2,
            "" => DEFAULT_MAG_FILTER_MODE,
            _ => DEFAULT_MAG_FILTER_MODE,
        }
    })
}

#[inline]
pub(crate) fn sample_mode(undersample: bool) -> f32 {
    if pixel_perfect_enabled() {
        return 3.0;
    }
    if undersample {
        return 2.0;
    }
    match mag_filter_mode() {
        1 => 1.0,
        2 => 2.0,
        _ => 0.0,
    }
}
