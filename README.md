# nenarwia

A fast GPU canvas/image wall viewer built with Rust and wgpu.

## Status

Early open-source release. The project is currently focused on Windows desktop usage.

## Features

- GPU rendering with `wgpu`
- Zoomable image canvas / image wall
- Custom window chrome
- Thumbnail and tile caching
- RGBA/LZ4 tile cache
- JPEG decoding acceleration through `turbojpeg` / `libjpeg-turbo`

## Build

Install Rust, then run:

```powershell
cargo build --release --locked
```

The executable will be created at:

```text
target/release/nenarwia.exe
```

## License

This project is licensed under the MIT License.

Third-party components are distributed under their respective licenses. See:

- `THIRD_PARTY_NOTICES.md`
- `third_party/licenses/`
