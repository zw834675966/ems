#!/usr/bin/env bash
set -euo pipefail

: "${EMS_MQTT_HOST:=127.0.0.1}"
: "${EMS_MQTT_PORT:=1883}"
: "${EMS_MQTT_USERNAME:=}"
: "${EMS_MQTT_PASSWORD:=}"
: "${EMS_MQTT_TOPIC_PREFIX:=ems}"
: "${EMS_MQTT_COMMAND_TOPIC_PREFIX:=}"
: "${EMS_MQTT_RECEIPT_TOPIC_PREFIX:=}"
: "${EMS_DEVICE_CLIENT_ID:=}"
: "${EMS_DEVICE_COMMAND_QOS:=1}"
: "${EMS_DEVICE_RECEIPT_QOS:=1}"
: "${EMS_DEVICE_PROCESSING_DELAY_MS:=0}"
: "${EMS_DEVICE_RECEIPT_TOPIC_EXTRA:=}"
: "${EMS_DEVICE_RECEIPT_STATUS:=success}"
: "${EMS_DEVICE_RECEIPT_MESSAGE:=applied}"
: "${EMS_DEVICE_RECEIPT_TS_MS:=}"
: "${EMS_DEVICE_RECEIPT_PAYLOAD_MODE:=json}"

if ! command -v mosquitto_sub >/dev/null 2>&1; then
  echo "mosquitto_sub not found" >&2
  exit 1
fi
if ! command -v mosquitto_pub >/dev/null 2>&1; then
  echo "mosquitto_pub not found" >&2
  exit 1
fi

if [ -z "$EMS_MQTT_COMMAND_TOPIC_PREFIX" ]; then
  command_prefix="${EMS_MQTT_TOPIC_PREFIX%/}/commands"
else
  command_prefix="${EMS_MQTT_COMMAND_TOPIC_PREFIX%/}"
fi

if [ -z "$EMS_MQTT_RECEIPT_TOPIC_PREFIX" ]; then
  receipt_prefix="${EMS_MQTT_TOPIC_PREFIX%/}/receipts"
else
  receipt_prefix="${EMS_MQTT_RECEIPT_TOPIC_PREFIX%/}"
fi

sub_args=( -h "$EMS_MQTT_HOST" -p "$EMS_MQTT_PORT" -v -t "${command_prefix}/#" )
pub_args=( -h "$EMS_MQTT_HOST" -p "$EMS_MQTT_PORT" )
if [ -n "$EMS_DEVICE_CLIENT_ID" ]; then
  sub_args+=( -i "$EMS_DEVICE_CLIENT_ID" )
fi
if [ -n "$EMS_MQTT_USERNAME" ] && [ -n "$EMS_MQTT_PASSWORD" ]; then
  sub_args+=( -u "$EMS_MQTT_USERNAME" -P "$EMS_MQTT_PASSWORD" )
  pub_args+=( -u "$EMS_MQTT_USERNAME" -P "$EMS_MQTT_PASSWORD" )
fi
sub_args+=( -q "$EMS_DEVICE_COMMAND_QOS" )

echo "device emulator started"
echo "  subscribe: ${command_prefix}/#"
echo "  publish receipts to: ${receipt_prefix}/<tenant>/<project>/[extra]/<command>"

mosquitto_sub "${sub_args[@]}" | while IFS=' ' read -r topic payload; do
  rest="${topic#${command_prefix}/}"
  IFS='/' read -r -a parts <<<"$rest"
  if [ "${#parts[@]}" -lt 3 ]; then
    echo "skip topic: $topic" >&2
    continue
  fi
  tenant_id="${parts[0]}"
  project_id="${parts[1]}"
  command_id="${parts[$((${#parts[@]} - 1))]}"
  if [ -z "$tenant_id" ] || [ -z "$project_id" ] || [ -z "$command_id" ]; then
    echo "skip topic: $topic" >&2
    continue
  fi

  receipt_topic="${receipt_prefix}/${tenant_id}/${project_id}"
  if [ -n "$EMS_DEVICE_RECEIPT_TOPIC_EXTRA" ]; then
    extra="${EMS_DEVICE_RECEIPT_TOPIC_EXTRA#/}"
    extra="${extra%/}"
    receipt_topic="${receipt_topic}/${extra}"
  fi
  receipt_topic="${receipt_topic}/${command_id}"
  receipt_payload=$(
    python3 - <<'PY'
import json
import os
import time

status = os.environ.get("EMS_DEVICE_RECEIPT_STATUS", "success")
message = os.environ.get("EMS_DEVICE_RECEIPT_MESSAGE", "")
ts_ms = os.environ.get("EMS_DEVICE_RECEIPT_TS_MS")
mode = os.environ.get("EMS_DEVICE_RECEIPT_PAYLOAD_MODE", "json").strip().lower()
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

  if [ "${EMS_DEVICE_PROCESSING_DELAY_MS}" -gt 0 ]; then
    sleep "$(python3 -c 'import os; print(max(int(os.environ.get("EMS_DEVICE_PROCESSING_DELAY_MS","0")),0)/1000)')"
  fi

  mosquitto_pub "${pub_args[@]}" -q "$EMS_DEVICE_RECEIPT_QOS" -t "$receipt_topic" -m "$receipt_payload" >/dev/null
  echo "command received: topic=$topic payload_size=${#payload} -> receipt published: $receipt_topic"
done
