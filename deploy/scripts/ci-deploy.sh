#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEPLOY_DIR="$ROOT_DIR/deploy"
ENV_FILE="$DEPLOY_DIR/.env"
COMPOSE_ARGS=(-f "$DEPLOY_DIR/docker-compose.yml" --env-file "$ENV_FILE")

run_root() {
  if [[ "$(id -u)" -eq 0 ]]; then
    "$@"
    return
  fi

  if command -v sudo >/dev/null 2>&1; then
    sudo "$@"
    return
  fi

  echo "This script requires root or sudo." >&2
  exit 1
}

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "Missing required variable in deploy/.env: $name" >&2
    exit 1
  fi
}

has_compose() {
  docker compose version >/dev/null 2>&1 || command -v docker-compose >/dev/null 2>&1
}

compose() {
  if docker compose version >/dev/null 2>&1; then
    run_root docker compose "$@"
    return
  fi

  if command -v docker-compose >/dev/null 2>&1; then
    run_root docker-compose "$@"
    return
  fi

  echo "Docker Compose is not installed." >&2
  exit 1
}

ensure_packages() {
  local missing=0

  command -v certbot >/dev/null 2>&1 || missing=1
  command -v crontab >/dev/null 2>&1 || missing=1
  docker compose version >/dev/null 2>&1 || missing=1

  if [[ "$missing" -eq 0 ]]; then
    return
  fi

  run_root apt-get update
  run_root apt-get install -y ca-certificates certbot cron curl
  run_root install -m 0755 -d /etc/apt/keyrings

  if [[ ! -f /etc/apt/keyrings/docker.asc ]]; then
    run_root curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
    run_root chmod a+r /etc/apt/keyrings/docker.asc
  fi

  run_root sh -c '. /etc/os-release && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu ${UBUNTU_CODENAME:-$VERSION_CODENAME} stable" > /etc/apt/sources.list.d/docker.list'
  run_root apt-get update
  run_root apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
}

ensure_docker_daemon_config() {
  run_root install -m 0755 -d /etc/docker

  if [[ ! -f /etc/docker/daemon.json ]]; then
    run_root sh -c "printf '%s\n' '{\"userland-proxy-path\": \"/usr/bin/docker-proxy\"}' > /etc/docker/daemon.json"
    return
  fi

  if ! run_root grep -q '"userland-proxy-path"' /etc/docker/daemon.json; then
    echo "Warning: /etc/docker/daemon.json exists without userland-proxy-path; update it manually if Docker fails to start." >&2
  fi
}

ensure_docker() {
  if command -v systemctl >/dev/null 2>&1; then
    run_root systemctl reset-failed docker docker.socket || true
    run_root systemctl stop docker docker.socket || true
    run_root systemctl daemon-reload
    run_root systemctl start docker.socket
    run_root systemctl enable --now docker
  fi
}

register_renew_hook() {
  local hook_path="$DEPLOY_DIR/scripts/renew-hook.sh"
  local cron_line="0 3 * * * certbot renew --deploy-hook '$hook_path'"
  local tmp_file

  tmp_file="$(mktemp)"
  run_root sh -c "crontab -l 2>/dev/null || true" | grep -v "renew-hook.sh" > "$tmp_file" || true
  printf '%s\n' "$cron_line" >> "$tmp_file"
  run_root crontab "$tmp_file"
  rm -f "$tmp_file"
}

ensure_certificate_lineage() {
  local domain="$1"
  local cert_path="/etc/letsencrypt/live/${domain}/fullchain.pem"

  if [[ -f "$cert_path" ]]; then
    return
  fi

  run_root certbot certonly \
    --standalone \
    --non-interactive \
    --agree-tos \
    --email "$LETSENCRYPT_EMAIL" \
    --cert-name "$domain" \
    -d "$domain"
}

ensure_certificates() {
  if command -v docker >/dev/null 2>&1 && has_compose; then
    compose "${COMPOSE_ARGS[@]}" down || true
  fi

  ensure_certificate_lineage "$API_DOMAIN"
  ensure_certificate_lineage "$TURN_DOMAIN"
  register_renew_hook
}

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing $ENV_FILE" >&2
  exit 1
fi

# shellcheck disable=SC1090
source "$ENV_FILE"

require_env API_DOMAIN
require_env TURN_DOMAIN
require_env TURN_EXTERNAL_IP
require_env LETSENCRYPT_EMAIL
require_env JWT_SECRET
require_env TURN_SECRET
require_env POSTGRES_PASSWORD

run_root mkdir -p "$DEPLOY_DIR/acme"
run_root chmod +x "$DEPLOY_DIR/scripts/renew-hook.sh"

ensure_packages
ensure_docker_daemon_config
ensure_docker
ensure_certificates

compose "${COMPOSE_ARGS[@]}" up -d --build --remove-orphans
compose "${COMPOSE_ARGS[@]}" ps
