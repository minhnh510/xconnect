# Alpha Release Checklist

## Build and sign
- Build Windows installer (MSI) from `apps/desktop-tauri`.
- Build macOS app bundle and notarize.
- Verify artifacts are signed and installable on clean machines.

## Backend readiness
- Deploy `deploy/docker-compose.yml` on Ubuntu 22.04.
- Validate HTTPS endpoint at `https://api.<domain>/v1/health`.
- Validate TURN on `turn:<turn.domain>:3478` and `turns:<turn.domain>:5349`.

## Functional tests
- Register/login/refresh/logout flow.
- Register two devices, trust one device, enable unattended on target.
- Start session and verify signaling over WSS.
- Validate clipboard text sync both directions.

## Performance checks
- LAN test: target 1080p60, observe FPS stability.
- WAN test: monitor p95 latency and frame drop ratio.

## Operational checks
- Confirm daily backups for Postgres.
- Confirm cert renewal hook reloads nginx and coturn.
- Confirm log retention and disk usage alarms.
