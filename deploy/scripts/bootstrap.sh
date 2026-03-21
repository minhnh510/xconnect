#!/usr/bin/env bash
set -euo pipefail

if [[ ! -f "deploy/.env" ]]; then
  echo "Missing deploy/.env. Copy deploy/env/.env.example to deploy/.env and update values."
  exit 1
fi

DOCKER_NEEDS_RESTART=0

compose() {
  if sudo docker compose version >/dev/null 2>&1; then
    sudo docker compose "$@"
    return
  fi

  if command -v docker-compose >/dev/null 2>&1; then
    sudo docker-compose "$@"
    return
  fi

  echo "Docker Compose is not installed." >&2
  exit 1
}

docker_is_ready() {
  command -v docker >/dev/null 2>&1 && sudo docker info >/dev/null 2>&1
}

install_docker_stack() {
  sudo apt-get update
  sudo apt-get install -y ca-certificates certbot cron curl
  sudo install -m 0755 -d /etc/apt/keyrings

  if [[ ! -f /etc/apt/keyrings/docker.asc ]]; then
    sudo curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
    sudo chmod a+r /etc/apt/keyrings/docker.asc
  fi

  sudo sh -c '. /etc/os-release && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu ${UBUNTU_CODENAME:-$VERSION_CODENAME} stable" > /etc/apt/sources.list.d/docker.list'
  sudo apt-get update
  sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
}

ensure_docker_daemon_config() {
  sudo install -m 0755 -d /etc/docker

  if [[ ! -f /etc/docker/daemon.json ]]; then
    sudo sh -c "printf '%s\n' '{\"userland-proxy-path\": \"/usr/bin/docker-proxy\"}' > /etc/docker/daemon.json"
    DOCKER_NEEDS_RESTART=1
    return
  fi

  if ! sudo grep -q '"userland-proxy-path"' /etc/docker/daemon.json; then
    echo "Warning: /etc/docker/daemon.json exists without userland-proxy-path; update it manually if Docker fails to start." >&2
  fi
}

ensure_docker_service() {
  if docker_is_ready && [[ "$DOCKER_NEEDS_RESTART" -eq 0 ]]; then
    return
  fi

  sudo systemctl daemon-reload
  sudo systemctl enable docker.socket || true
  sudo systemctl enable docker || true

  if [[ "$DOCKER_NEEDS_RESTART" -eq 1 ]]; then
    sudo systemctl reset-failed docker docker.socket || true
    sudo systemctl stop docker docker.socket || true
    sudo systemctl start docker.socket
    sudo systemctl start docker
  else
    sudo systemctl start docker.socket || true
    sudo systemctl start docker || true
  fi

  local attempt
  for attempt in {1..10}; do
    if docker_is_ready; then
      return
    fi

    sleep 2
  done

  echo "Docker daemon is not ready." >&2
  sudo systemctl status docker --no-pager || true
  exit 1
}

ensure_certificate_lineage() {
  local domain="$1"
  local cert_path="/etc/letsencrypt/live/${domain}/fullchain.pem"

  if [[ -f "$cert_path" ]]; then
    return
  fi

  sudo certbot certonly \
    --standalone \
    --non-interactive \
    --agree-tos \
    --email "$LETSENCRYPT_EMAIL" \
    --cert-name "$domain" \
    -d "$domain"
}

source deploy/.env

for var in API_DOMAIN TURN_DOMAIN LETSENCRYPT_EMAIL JWT_SECRET TURN_SECRET TURN_EXTERNAL_IP; do
  if [[ -z "${!var:-}" ]]; then
    echo "Missing required variable: $var"
    exit 1
  fi
done

echo "[1/4] Installing dependencies (docker, certbot)"
install_docker_stack
ensure_docker_daemon_config
ensure_docker_service

echo "[2/4] Requesting Let's Encrypt certificates"
ensure_certificate_lineage "$API_DOMAIN"
ensure_certificate_lineage "$TURN_DOMAIN"

echo "[3/4] Starting stack"
(cd deploy && compose --env-file .env up -d --build)

echo "[4/4] Registering renew hook"
HOOK_PATH="$(pwd)/deploy/scripts/renew-hook.sh"
CRON_LINE="0 3 * * * certbot renew --deploy-hook '$HOOK_PATH'"
( sudo crontab -l 2>/dev/null | grep -v "renew-hook.sh"; echo "$CRON_LINE" ) | sudo crontab -

if sudo docker compose version >/dev/null 2>&1; then
  echo "Done. Verify with: sudo docker compose -f deploy/docker-compose.yml --env-file deploy/.env ps"
else
  echo "Done. Verify with: sudo docker-compose -f deploy/docker-compose.yml --env-file deploy/.env ps"
fi
