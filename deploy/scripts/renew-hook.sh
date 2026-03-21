#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

compose() {
  if docker compose version >/dev/null 2>&1; then
    docker compose "$@"
    return
  fi

  if command -v docker-compose >/dev/null 2>&1; then
    docker-compose "$@"
    return
  fi

  echo "Docker Compose is not installed." >&2
  exit 1
}

cd "$ROOT_DIR/deploy"

compose --env-file .env exec -T nginx nginx -s reload || true
compose --env-file .env restart coturn || true
