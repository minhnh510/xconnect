# XConnect

Monorepo MVP for cross-platform remote desktop:
- Windows/macOS desktop runtime
- Rust control-plane API + signaling
- Ubuntu self-host deployment (Nginx + Let's Encrypt + coturn)

## Layout
- `crates/protocol`: Shared API and signaling types
- `services/control-plane`: Auth/device/session/signaling backend
- `crates/host-runtime`: Host-side capture/input/clipboard abstraction + WebRTC host wiring
- `crates/viewer-runtime`: Viewer-side render/input/clipboard abstraction + WebRTC viewer wiring
- `apps/desktop-tauri`: Desktop shell scaffold
- `deploy/`: Docker compose + infra configs/scripts

## Dev Verification

Prerequisites:
- Rust `stable` with `rustfmt` and `clippy` components installed (see `rust-toolchain.toml`).
- `cargo` available in `PATH`.

Recommended local verification:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p control-plane -p host-runtime -p viewer-runtime
cargo test -p xconnect-protocol -p control-plane -p host-runtime -p viewer-runtime
```

CI coverage in `.github/workflows/ci.yml` currently includes:
- Linux: format check, workspace clippy, protocol tests, and workspace cargo check.
- macOS: `cargo check -p host-runtime` and `cargo test -p host-runtime`.

### Windows note
If `cargo` is not already in `PATH` for PowerShell, prepend it with
`$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`.

If this machine has legacy Visual Studio SDK env vars (`INCLUDE`/`LIB`) pre-set, native dependencies can fail to compile.
Use command-scoped overrides before running cargo commands:
`$env:INCLUDE = ""; $env:LIB = ""`.

### Local verification note
The implementation and CI configuration referenced above are present in the repository, but this machine has not re-run local cargo verification on 2026-03-21 because `cargo` is not currently in `PATH`.

## Encoder backend selection (host-runtime)

Set `XCONNECT_H264_BACKEND` before launching host runtime:
- `auto` (default): Windows tries `nvenc`, then `qsv`, then `media_foundation`, then `software`; macOS tries `videotoolbox`, then `software`.
- `media_foundation`, `nvenc`, `qsv` on Windows.
- `videotoolbox` on macOS.
- `software` on all platforms.

Current implementation:
- Windows uses real Media Foundation H.264 MFT encode paths, including vendor-filtered selection flows for `nvenc` and `qsv`.
- macOS uses a real VideoToolbox `VTCompressionSession` H.264 encode path and emits Annex-B NAL units.
- `software` remains the fallback path when hardware encode is unavailable or explicitly requested.

Evidence for the statements above lives in `crates/host-runtime/src/encoder_h264.rs`, `crates/host-runtime/src/lib.rs`, and `.github/workflows/ci.yml`.
