use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;

use super::fs::unix_time_ms;
use super::model::MAX_SAVED_WALLPAPERS;
use super::*;

const TEST_BLUR_MAX_DIM: u32 = 480;

fn make_test_root(name: &str) -> PathBuf {
    let unique = format!("canvas_engine_wallpaper_test_{}_{}", name, unix_time_ms());
    let root = std::env::temp_dir().join(unique);
    let _ = fs::remove_dir_all(&root);
    root
}

fn write_test_png(path: &Path, color: [u8; 4]) {
    write_test_png_sized(path, 16, 16, color);
}

fn write_test_png_sized(path: &Path, width: u32, height: u32, color: [u8; 4]) {
    let mut pixels = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for _ in 0..(width * height) {
        pixels.extend_from_slice(&color);
    }
    image::save_buffer_with_format(
        path,
        &pixels,
        width,
        height,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .expect("write test png");
}

fn write_test_jpeg(path: &Path, color: [u8; 3], quality: u8) {
    let mut pixels = Vec::with_capacity(16 * 16 * 3);
    for _ in 0..(16 * 16) {
        pixels.extend_from_slice(&color);
    }
    let file = fs::File::create(path).expect("create test jpeg");
    let writer = io::BufWriter::new(file);
    let mut encoder = JpegEncoder::new_with_quality(writer, quality);
    encoder
        .encode(&pixels, 16, 16, image::ExtendedColorType::Rgb8)
        .expect("write test jpeg");
}

fn test_jpeg_bytes(color: [u8; 3]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(16 * 16 * 3);
    for _ in 0..(16 * 16) {
        pixels.extend_from_slice(&color);
    }
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 80);
    encoder
        .encode(&pixels, 16, 16, image::ExtendedColorType::Rgb8)
        .expect("write test jpeg bytes");
    bytes
}

fn write_test_webp(path: &Path, color: [u8; 4]) {
    let mut pixels = Vec::with_capacity(16 * 16 * 4);
    for _ in 0..(16 * 16) {
        pixels.extend_from_slice(&color);
    }
    image::save_buffer_with_format(
        path,
        &pixels,
        16,
        16,
        image::ColorType::Rgba8,
        image::ImageFormat::WebP,
    )
    .expect("write test webp");
}

#[test]
fn default_wallpaper_seed_becomes_active_in_empty_library() {
    let root = make_test_root("default_seed_empty");
    let default_bytes = test_jpeg_bytes([120, 80, 40]);

    let mut library = WallpaperLibrary::load_from_root(root.join("wallpapers"));
    let entry = library
        .ensure_default_wallpaper(&default_bytes)
        .expect("seed default wallpaper");

    assert_eq!(library.entries().len(), 1);
    assert_eq!(library.active_id(), Some(entry.id));
    assert!(entry.is_default);
    assert!(entry.source_path.exists());

    let reloaded = WallpaperLibrary::load_from_root(root.join("wallpapers"));
    assert_eq!(reloaded.active_id(), Some(entry.id));
    assert_eq!(reloaded.entries().len(), 1);
}

#[test]
fn default_wallpaper_seed_does_not_override_active_custom_wallpaper() {
    let root = make_test_root("default_seed_custom");
    let source_dir = root.join("sources");
    fs::create_dir_all(&source_dir).expect("source dir");

    let custom_source = source_dir.join("custom.png");
    write_test_png(&custom_source, [20, 60, 100, 255]);
    let mut library = WallpaperLibrary::load_from_root(root.join("wallpapers"));
    let custom = library
        .create_from_new_source(&custom_source, false, 0.4, TEST_BLUR_MAX_DIM)
        .expect("create custom wallpaper");

    let default = library
        .ensure_default_wallpaper(&test_jpeg_bytes([180, 140, 90]))
        .expect("seed default wallpaper");

    assert_eq!(library.active_id(), Some(custom.id));
    assert_eq!(library.entries().len(), 2);
    assert_eq!(library.entries()[0].id, custom.id);
    assert_eq!(
        library.entries().last().map(|entry| entry.id),
        Some(default.id)
    );
    assert!(default.is_default);
    assert!(default.source_path.exists());
}

#[test]
fn default_wallpaper_seed_is_kept_at_end_when_user_history_overflows() {
    let root = make_test_root("default_seed_overflow");
    let source_dir = root.join("sources");
    fs::create_dir_all(&source_dir).expect("source dir");

    let mut library = WallpaperLibrary::load_from_root(root.join("wallpapers"));
    let default = library
        .ensure_default_wallpaper(&test_jpeg_bytes([200, 160, 120]))
        .expect("seed default wallpaper");

    let mut latest_custom_id = 0;
    for idx in 0..MAX_SAVED_WALLPAPERS {
        let source = source_dir.join(format!("custom_{idx}.png"));
        write_test_png(&source, [idx as u8, 80, 120, 255]);
        latest_custom_id = library
            .create_from_new_source(&source, false, 0.2, TEST_BLUR_MAX_DIM)
            .expect("create custom wallpaper")
            .id;
    }

    assert_eq!(library.entries().len(), MAX_SAVED_WALLPAPERS);
    assert_eq!(library.active_id(), Some(latest_custom_id));
    assert_eq!(
        library
            .entries()
            .last()
            .map(|entry| (entry.id, entry.is_default)),
        Some((default.id, true))
    );
}

#[test]
fn creates_updates_and_trims_wallpaper_library() {
    let root = make_test_root("library");
    let source_dir = root.join("sources");
    fs::create_dir_all(&source_dir).expect("source dir");

    let mut library = WallpaperLibrary::load_from_root(root.join("wallpapers"));
    for idx in 0..(MAX_SAVED_WALLPAPERS + 1) {
        let source = source_dir.join(format!("input_{idx}.png"));
        write_test_png(&source, [idx as u8, 10, 20, 255]);
        library
            .create_from_new_source(&source, idx % 2 == 0, 0.25, TEST_BLUR_MAX_DIM)
            .expect("create wallpaper");
    }

    assert_eq!(library.entries().len(), MAX_SAVED_WALLPAPERS);
    assert_eq!(
        library.active_id(),
        library.entries().first().map(|entry| entry.id)
    );

    let first_id = library.entries()[0].id;
    let updated = library
        .update_existing(first_id, true, 0.75, TEST_BLUR_MAX_DIM)
        .expect("update existing");
    assert_eq!(updated.id, first_id);
    assert!(updated.blur_enabled);
    assert!((updated.dim_amount - 0.75).abs() < 0.001);
    assert_eq!(library.entries()[0].id, first_id);
    assert_eq!(
        library.entries()[0]
            .source_path
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("jpg")
    );

    let reloaded = WallpaperLibrary::load_from_root(root.join("wallpapers"));
    assert_eq!(reloaded.entries().len(), MAX_SAVED_WALLPAPERS);
    assert_eq!(reloaded.active_id(), Some(first_id));
}

#[test]
fn small_jpeg_is_copied_without_reencoding() {
    let root = make_test_root("jpeg_copy");
    let source_dir = root.join("sources");
    fs::create_dir_all(&source_dir).expect("source dir");

    let source = source_dir.join("input.jpg");
    write_test_jpeg(&source, [90, 140, 210], 73);
    let source_bytes = fs::read(&source).expect("read source jpeg");

    let mut library = WallpaperLibrary::load_from_root(root.join("wallpapers"));
    let entry = library
        .create_from_new_source(&source, false, 0.2, TEST_BLUR_MAX_DIM)
        .expect("save jpeg wallpaper");
    let stored_bytes = fs::read(&entry.source_path).expect("read stored jpeg");

    assert_eq!(stored_bytes, source_bytes);
}

#[test]
fn webp_renamed_to_jpeg_is_reencoded_instead_of_copied_as_is() {
    let root = make_test_root("renamed_webp");
    let source_dir = root.join("sources");
    fs::create_dir_all(&source_dir).expect("source dir");

    let source = source_dir.join("input.jpeg");
    write_test_webp(&source, [12, 34, 56, 255]);

    assert_eq!(
        crate::core::color::image_format_any(&source),
        Some(image::ImageFormat::WebP)
    );

    let mut library = WallpaperLibrary::load_from_root(root.join("wallpapers"));
    let entry = library
        .create_from_new_source(&source, false, 0.2, TEST_BLUR_MAX_DIM)
        .expect("save disguised webp wallpaper");

    assert_eq!(
        crate::core::color::image_format_any(&entry.source_path),
        Some(image::ImageFormat::Jpeg)
    );
}

#[test]
fn duplicate_source_reuses_existing_entry() {
    let root = make_test_root("dedupe");
    let source_dir = root.join("sources");
    fs::create_dir_all(&source_dir).expect("source dir");

    let source_a = source_dir.join("input_a.png");
    let source_b = source_dir.join("input_b.png");
    write_test_png(&source_a, [12, 34, 56, 255]);
    fs::copy(&source_a, &source_b).expect("copy duplicate source");

    let mut library = WallpaperLibrary::load_from_root(root.join("wallpapers"));
    let first = library
        .create_from_new_source(&source_a, false, 0.1, TEST_BLUR_MAX_DIM)
        .expect("create first wallpaper");
    let second = library
        .create_from_new_source(&source_b, true, 0.8, TEST_BLUR_MAX_DIM)
        .expect("reuse duplicate wallpaper");

    assert_eq!(library.entries().len(), 1);
    assert_eq!(first.id, second.id);
    assert!(second.blur_enabled);
    assert!((second.dim_amount - 0.8).abs() < 0.001);
    assert_eq!(library.active_id(), Some(first.id));
}

#[test]
fn blur_copy_is_created_and_removed_with_blur_flag() {
    let root = make_test_root("blur_copy");
    let source_dir = root.join("sources");
    fs::create_dir_all(&source_dir).expect("source dir");

    let source = source_dir.join("input.png");
    write_test_png(&source, [40, 90, 140, 255]);

    let mut library = WallpaperLibrary::load_from_root(root.join("wallpapers"));
    let entry = library
        .create_from_new_source(&source, true, 0.3, TEST_BLUR_MAX_DIM)
        .expect("create wallpaper with blur");
    let blur_path = library.preview_blur_path(entry.id);
    assert!(blur_path.exists());

    let updated = library
        .update_existing(entry.id, false, 0.3, TEST_BLUR_MAX_DIM)
        .expect("disable blur");
    assert_eq!(updated.id, entry.id);
    assert!(!blur_path.exists());
}

#[test]
fn blur_copy_refreshes_when_expected_work_size_changes() {
    let root = make_test_root("blur_refresh");
    let source_dir = root.join("sources");
    fs::create_dir_all(&source_dir).expect("source dir");

    let source = source_dir.join("input.png");
    write_test_png_sized(&source, 1024, 512, [90, 70, 50, 255]);
    let blur_path = root
        .join("wallpapers")
        .join("items")
        .join("manual")
        .join("preview_blur.jpg");
    ensure_saved_wallpaper_preview_blur(&source, &blur_path, 1024).expect("initial blur");
    ensure_saved_wallpaper_preview_blur(&source, &blur_path, TEST_BLUR_MAX_DIM)
        .expect("refresh blur");

    let dims = crate::core::color::image_dimensions_any(&blur_path).expect("blur dims");
    assert_eq!(dims.0.max(dims.1), TEST_BLUR_MAX_DIM);
}
