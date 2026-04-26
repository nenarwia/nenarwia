use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const RENDER_SHADER_PARTS: &[&str] = &[
    "src/render/shader/bindings_and_io.wgsl",
    "src/render/shader/vertex.wgsl",
    "src/render/shader/sampling_filters.wgsl",
    "src/render/shader/detail_tiles.wgsl",
    "src/render/shader/fit.wgsl",
    "src/render/shader/atlas.wgsl",
    "src/render/shader/fragment.wgsl",
];
const APP_NAME: &str = "nenarwia";
const APP_EXE_NAME: &str = "nenarwia.exe";

fn main() {
    generate_render_shader_bundle();
    configure_platform_resources();
}

fn generate_render_shader_bundle() {
    println!("cargo:rerun-if-changed=build.rs");
    for part in RENDER_SHADER_PARTS {
        println!("cargo:rerun-if-changed={part}");
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let output_path = out_dir.join("render_shader.wgsl");
    let mut bundle = String::new();

    for part in RENDER_SHADER_PARTS {
        let source = fs::read_to_string(part)
            .unwrap_or_else(|err| panic!("Failed to read shader part {part}: {err}"));
        let label = part.strip_prefix("src/render/").unwrap_or(part);

        if !bundle.is_empty() {
            bundle.push('\n');
        }

        bundle.push_str("// BEGIN ");
        bundle.push_str(label);
        bundle.push('\n');
        bundle.push_str(&source);
        if !source.ends_with('\n') {
            bundle.push('\n');
        }
        bundle.push_str("// END ");
        bundle.push_str(label);
        bundle.push('\n');
    }

    fs::write(&output_path, bundle).unwrap_or_else(|err| {
        panic!(
            "Failed to write bundled render shader to {}: {err}",
            output_path.display()
        )
    });
}

#[cfg(target_os = "windows")]
fn configure_platform_resources() {
    let icon_path = Path::new("assets").join("app.ico");
    println!("cargo:rerun-if-changed={}", icon_path.display());

    if !icon_path.exists() {
        println!(
            "cargo:warning=Windows app icon is missing at {}",
            icon_path.display()
        );
        return;
    }

    if let Some(path_str) = icon_path.to_str() {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(path_str);
        res.set("ProductName", APP_NAME);
        res.set("FileDescription", APP_NAME);
        res.set("InternalName", APP_NAME);
        res.set("OriginalFilename", APP_EXE_NAME);
        if let Err(err) = res.compile() {
            panic!("Failed to compile Windows resources: {err}");
        }
    } else {
        panic!("Icon path is not valid UTF-8: {}", icon_path.display());
    }
}

#[cfg(not(target_os = "windows"))]
fn configure_platform_resources() {}
