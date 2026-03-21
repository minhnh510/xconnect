#!/usr/bin/env bash
set -euo pipefail

if [[ ! -f "deploy/.env" ]]; then
  echo "Missing deploy/.env. Copy deploy/env/.env.example to deploy/.env and update values."
  exit 1
fi

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

source deploy/.env

for var in API_DOMAIN TURN_DOMAIN LETSENCRYPT_EMAIL JWT_SECRET TURN_SECRET TURN_EXTERNAL_IP; do
  if [[ -z "${!var:-}" ]]; then
    echo "Missing required variable: $var"
    exit 1
  fi
done

echo "[1/4] Installing dependencies (docker, certbot)"
sudo apt-get update
if apt-cache show docker-compose-plugin >/dev/null 2>&1; then
  sudo apt-get install -y docker.io docker-compose-plugin certbot
else
  sudo apt-get install -y docker.io docker-compose certbot
fi
sudo systemctl enable --now docker

echo "[2/4] Requesting Let's Encrypt certificate for $API_DOMAIN and $TURN_DOMAIN"
sudo certbot certonly --standalone \
  --non-interactive \
  --agree-tos \
  --email "$LETSENCRYPT_EMAIL" \
  -d "$API_DOMAIN" \
  -d "$TURN_DOMAIN"

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
