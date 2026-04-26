# Third-Party Notices

This project includes third-party components related to JPEG decoding.

## Included components

1. `turbojpeg` crate (Rust)
- Upstream: https://github.com/honzasp/rust-turbojpeg
- Crate: https://crates.io/crates/turbojpeg
- License: `MIT OR Unlicense`
- License files in this repository:
  - `third_party/licenses/turbojpeg/LICENSE-MIT.txt`
  - `third_party/licenses/turbojpeg/UNLICENSE.txt`

2. `turbojpeg-sys` crate (Rust FFI)
- Upstream: https://github.com/honzasp/rust-turbojpeg
- Crate: https://crates.io/crates/turbojpeg-sys
- License: `MIT OR Unlicense`
- License files in this repository:
  - `third_party/licenses/turbojpeg/LICENSE-MIT.txt`
  - `third_party/licenses/turbojpeg/UNLICENSE.txt`

3. `libjpeg-turbo` (native C library used by `turbojpeg-sys`)
- Upstream: https://github.com/libjpeg-turbo/libjpeg-turbo
- License model: IJG License + Modified BSD-3-Clause (see upstream license docs)
- License files in this repository:
  - `third_party/licenses/libjpeg-turbo/LICENSE.md`
  - `third_party/licenses/libjpeg-turbo/README.ijg`

## Required attribution for binary distribution

For binary distribution (including Steam builds), product documentation should include this exact sentence:

`This software is based in part on the work of the Independent JPEG Group.`

This requirement is described in `third_party/licenses/libjpeg-turbo/README.ijg`.

## Steam release checklist (JPEG stack)

- Include this file: `THIRD_PARTY_NOTICES.md`.
- Include license files from `third_party/licenses/turbojpeg/`.
- Include license files from `third_party/licenses/libjpeg-turbo/`.
- Add the IJG attribution sentence above in product docs/store docs/EULA/support page bundled with the build.
- Do not use names of IJG/libjpeg-turbo contributors for endorsement.

## Notes

- This document is an engineering compliance note, not legal advice.
- If licensing policy for your studio/publisher requires legal review, keep this file and ask counsel to sign off before release.
