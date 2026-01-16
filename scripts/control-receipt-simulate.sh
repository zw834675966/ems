#!/usr/bin/env bash
set -euo pipefail

: "${EMS_MQTT_HOST:=127.0.0.1}"
: "${EMS_MQTT_PORT:=1883}"
: "${EMS_MQTT_USERNAME:=}"
: "${EMS_MQTT_PASSWORD:=}"
: "${EMS_MQTT_TOPIC_PREFIX:=ems}"
: "${EMS_MQTT_RECEIPT_TOPIC_PREFIX:=}"
: "${EMS_MQTT_RECEIPT_QOS:=1}"
: "${EMS_TENANT_ID:=tenant-1}"
: "${EMS_PROJECT_ID:=project-1}"
: "${EMS_COMMAND_ID:=}"
: "${EMS_RECEIPT_TOPIC_EXTRA:=}"
: "${EMS_RECEIPT_STATUS:=success}"
: "${EMS_RECEIPT_MESSAGE:=applied}"
: "${EMS_RECEIPT_TS_MS:=}"
: "${EMS_RECEIPT_PAYLOAD_MODE:=json}"

if ! command -v mosquitto_pub >/dev/null 2>&1; then
  echo "mosquitto_pub not found" >&2
  exit 1
fi

if [ -z "$EMS_COMMAND_ID" ]; then
  echo "EMS_COMMAND_ID is required" >&2
  exit 1
fi

if [ -z "$EMS_MQTT_RECEIPT_TOPIC_PREFIX" ]; then
  prefix="${EMS_MQTT_TOPIC_PREFIX%/}/receipts"
else
  prefix="${EMS_MQTT_RECEIPT_TOPIC_PREFIX%/}"
fi

topic="${prefix}/${EMS_TENANT_ID}/${EMS_PROJECT_ID}"
if [ -n "$EMS_RECEIPT_TOPIC_EXTRA" ]; then
  extra="${EMS_RECEIPT_TOPIC_EXTRA#/}"
  extra="${extra%/}"
  topic="${topic}/${extra}"
fi
topic="${topic}/${EMS_COMMAND_ID}"

payload=$(
  python3 - <<'PY'
import json
import os
import time

status = os.environ.get("EMS_RECEIPT_STATUS", "success")
message = os.environ.get("EMS_RECEIPT_MESSAGE", "")
ts_ms = os.environ.get("EMS_RECEIPT_TS_MS")
mode = os.environ.get("EMS_RECEIPT_PAYLOAD_MODE", "json").strip().lower()
if ts_ms:
    ts_ms = int(ts_ms)
else:
    ts_ms = int(time.time() * 1000)

if mode == "text":
    print(status)
elif mode == "json_snake":
    data = {"status": status, "ts_ms": ts_ms}
    if message:
        data["message"] = message
    print(json.dumps(data, separators=(",", ":")))
elif mode == "json_alt":
    data = {"result": status, "timestamp": ts_ms}
    if message:
        data["msg"] = message
    print(json.dumps(data, separators=(",", ":")))
else:
    data = {"status": status, "tsMs": ts_ms}
    if message:
        data["message"] = message
    print(json.dumps(data, separators=(",", ":")))
PY
)

args=( -h "$EMS_MQTT_HOST" -p "$EMS_MQTT_PORT" -t "$topic" -m "$payload" )
if [ -n "$EMS_MQTT_USERNAME" ] && [ -n "$EMS_MQTT_PASSWORD" ]; then
  args+=( -u "$EMS_MQTT_USERNAME" -P "$EMS_MQTT_PASSWORD" )
fi

args+=( -q "$EMS_MQTT_RECEIPT_QOS" )
mosquitto_pub "${args[@]}"

echo "receipt published: $topic"
