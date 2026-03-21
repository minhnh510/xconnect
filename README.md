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

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
$env:INCLUDE = ""
$env:LIB = ""

cargo fmt --all --check
cargo check -p control-plane -p host-runtime -p viewer-runtime
cargo test -p xconnect-protocol -p control-plane -p host-runtime -p viewer-runtime
```

### Windows note
If this machine has legacy Visual Studio SDK env vars (`INCLUDE`/`LIB`) pre-set, native dependencies can fail to compile.
Use the same command-scoped override above (`$env:INCLUDE = ""; $env:LIB = ""`) before running cargo commands.

## Encoder backend selection (host-runtime)

Set `XCONNECT_H264_BACKEND` before launching host runtime:
- `auto` (default): Windows tries `Media Foundation` then software fallback; macOS tries `VideoToolbox` capability probe then software fallback.
- `media_foundation`, `nvenc`, `qsv` on Windows.
- `videotoolbox` on macOS.
- `software` on all platforms.

Current state:
- `Media Foundation` and `VideoToolbox` paths are initialized/probed and wired into backend selection.
- Actual frame payload is still software fallback until next phase hardware bitstream encode implementation.
