#!/usr/bin/env bash
set -euo pipefail

if [ -z "${EMS_DATABASE_URL:-}" ]; then
  EMS_DATABASE_URL="postgresql://ems:admin123@localhost:5432/ems"
  export EMS_DATABASE_URL
fi

echo "postgres: checking..."
pg_isready -d "$EMS_DATABASE_URL"
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -c "select 1;"

echo "redis: checking..."
redis_url="${EMS_REDIS_URL:-redis://default:admin123@localhost:6379}"
redis-cli -u "$redis_url" ping

mqtt_host="${EMS_MQTT_HOST:-127.0.0.1}"
mqtt_port="${EMS_MQTT_PORT:-1883}"
mqtt_user="${EMS_MQTT_USERNAME:-}"
mqtt_password="${EMS_MQTT_PASSWORD:-}"

if command -v mosquitto_sub >/dev/null 2>&1 && command -v mosquitto_pub >/dev/null 2>&1 \
  && [ -n "$mqtt_user" ] && [ -n "$mqtt_password" ]; then
  mosquitto_sub -h "$mqtt_host" -p "$mqtt_port" -u "$mqtt_user" -P "$mqtt_password" \
    -t ems/health-check -C 1 -W 3 >/tmp/mqtt-health.log 2>&1 &
  sub_pid=$!
  sleep 0.2
  mosquitto_pub -h "$mqtt_host" -p "$mqtt_port" -u "$mqtt_user" -P "$mqtt_password" \
    -t ems/health-check -m "ok"
  if wait $sub_pid; then
    echo "mqtt: ok"
  else
    cat /tmp/mqtt-health.log >&2
    exit 1
  fi
elif command -v nc >/dev/null 2>&1; then
  if nc -z "$mqtt_host" "$mqtt_port"; then
    echo "mqtt: ok"
  else
    echo "mqtt: not reachable at ${mqtt_host}:${mqtt_port}"
    exit 1
  fi
else
  echo "mqtt: skipped (mosquitto/nc not installed)"
fi

echo "health check ok"
