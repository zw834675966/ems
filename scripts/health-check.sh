#!/usr/bin/env bash
set -euo pipefail

: "${EMS_DATABASE_URL:?EMS_DATABASE_URL is required}"

echo "postgres: checking..."
pg_isready -d "$EMS_DATABASE_URL"
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -c "select 1;"

echo "redis: checking..."
redis_url="${EMS_REDIS_URL:-redis://localhost:6379}"
redis-cli -u "$redis_url" ping

if command -v nc >/dev/null 2>&1; then
  mqtt_host="${EMS_MQTT_HOST:-127.0.0.1}"
  mqtt_port="${EMS_MQTT_PORT:-1883}"
  if nc -z "$mqtt_host" "$mqtt_port"; then
    echo "mqtt: ok"
  else
    echo "mqtt: not reachable at ${mqtt_host}:${mqtt_port}"
    exit 1
  fi
else
  echo "mqtt: skipped (nc not installed)"
fi

echo "health check ok"
