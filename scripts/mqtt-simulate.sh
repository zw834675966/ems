#!/usr/bin/env bash
set -euo pipefail

: "${EMS_MQTT_HOST:=127.0.0.1}"
: "${EMS_MQTT_PORT:=1883}"
: "${EMS_MQTT_USERNAME:=}"
: "${EMS_MQTT_PASSWORD:=}"
: "${EMS_MQTT_TOPIC_PREFIX:=ems}"
: "${EMS_MQTT_DATA_TOPIC_PREFIX:=}"
: "${EMS_TENANT_ID:=tenant-1}"
: "${EMS_PROJECT_ID:=project-1}"
: "${EMS_SOURCE_ID:=}"
: "${EMS_POINT_ADDRESS:=demo/topic}"
: "${EMS_PAYLOAD:=12.3}"

if ! command -v mosquitto_pub >/dev/null 2>&1; then
  echo "mosquitto_pub not found" >&2
  exit 1
fi

if [ -n "${EMS_MQTT_TOPIC:-}" ]; then
  topic="$EMS_MQTT_TOPIC"
else
  if [ -n "$EMS_MQTT_DATA_TOPIC_PREFIX" ]; then
    prefix="${EMS_MQTT_DATA_TOPIC_PREFIX%/}"
  else
    prefix="${EMS_MQTT_TOPIC_PREFIX%/}/data"
  fi
  if [ -n "$EMS_SOURCE_ID" ]; then
    topic="${prefix}/${EMS_TENANT_ID}/${EMS_PROJECT_ID}/${EMS_SOURCE_ID}/${EMS_POINT_ADDRESS}"
  else
    topic="${prefix}/${EMS_TENANT_ID}/${EMS_PROJECT_ID}/${EMS_POINT_ADDRESS}"
  fi
fi

args=( -h "$EMS_MQTT_HOST" -p "$EMS_MQTT_PORT" -t "$topic" -m "$EMS_PAYLOAD" )
if [ -n "$EMS_MQTT_USERNAME" ] && [ -n "$EMS_MQTT_PASSWORD" ]; then
  args+=( -u "$EMS_MQTT_USERNAME" -P "$EMS_MQTT_PASSWORD" )
fi

mosquitto_pub "${args[@]}"

echo "mqtt publish ok: $topic"
