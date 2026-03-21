# Security Runbook

## Baseline hardening
- Use strong random values for `JWT_SECRET`, `TURN_SECRET`, and database password.
- Restrict SSH to key-based login and disable password auth.
- Enable UFW and only allow: `22/tcp`, `80/tcp`, `443/tcp`, `3478/tcp`, `3478/udp`, `5349/tcp`, `49152:65535/udp`.
- Keep Ubuntu security updates enabled.

## API and auth
- Rate-limit auth endpoints (`/v1/auth/login`, `/v1/auth/register`) at reverse proxy and app layers.
- Store only password hashes (Argon2).
- Rotate JWT secrets on incident or at a regular interval.

## TURN security
- Use `use-auth-secret` in coturn.
- Rotate `TURN_SECRET` periodically and during incident response.
- Keep TURN logs for abuse tracing and disable open relay behavior.

## TLS
- Certificates are managed by Let's Encrypt.
- Verify cert renewal monthly and monitor expiry alerts.
- If certificate changes unexpectedly, investigate compromise risk.

## Incident response
- Revoke active refresh tokens by rotating JWT secret.
- Rotate TURN secret and restart coturn.
- Export logs before cleanup for forensic review.
