#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "$ROOT_DIR/deploy"

docker compose --env-file .env exec -T nginx nginx -s reload || true
docker compose --env-file .env restart coturn || true
