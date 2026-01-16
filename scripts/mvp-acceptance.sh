#!/usr/bin/env bash
set -euo pipefail

: "${EMS_HTTP_ADDR:=127.0.0.1:18080}"
: "${EMS_DATABASE_URL:=postgresql://ems:admin123@localhost:5432/ems}"
: "${EMS_JWT_SECRET:=dev}"
: "${EMS_JWT_ACCESS_TTL_SECONDS:=3600}"
: "${EMS_JWT_REFRESH_TTL_SECONDS:=7200}"
: "${EMS_REDIS_URL:=redis://default:admin123@localhost:6379}"
: "${EMS_MQTT_HOST:=127.0.0.1}"
: "${EMS_MQTT_PORT:=1883}"
: "${EMS_MQTT_USERNAME:=ems}"
: "${EMS_MQTT_PASSWORD:=admin123}"
: "${EMS_MQTT_TOPIC_PREFIX:=ems}"
: "${EMS_TENANT_ID:=tenant-1}"
: "${EMS_POINT_ADDRESS:=demo/topic}"
: "${EMS_POINT_PAYLOAD:=12.3}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl not found" >&2
  exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 not found" >&2
  exit 1
fi
if ! command -v mosquitto_pub >/dev/null 2>&1; then
  echo "mosquitto_pub not found" >&2
  exit 1
fi

host="${EMS_HTTP_ADDR%:*}"
port="${EMS_HTTP_ADDR##*:}"
base_url="http://${host}:${port}"

export EMS_DATABASE_URL

scripts/db-init.sh
scripts/health-check.sh

cargo build -p ems-api >/dev/null

log_file="${TMPDIR:-/tmp}/ems-api-${port}.log"
EMS_HTTP_ADDR="$EMS_HTTP_ADDR" \
EMS_DATABASE_URL="$EMS_DATABASE_URL" \
EMS_REDIS_URL="$EMS_REDIS_URL" \
EMS_INGEST=on \
EMS_CONTROL=on \
EMS_JWT_SECRET="$EMS_JWT_SECRET" \
EMS_JWT_ACCESS_TTL_SECONDS="$EMS_JWT_ACCESS_TTL_SECONDS" \
EMS_JWT_REFRESH_TTL_SECONDS="$EMS_JWT_REFRESH_TTL_SECONDS" \
EMS_MQTT_HOST="$EMS_MQTT_HOST" \
EMS_MQTT_PORT="$EMS_MQTT_PORT" \
EMS_MQTT_USERNAME="$EMS_MQTT_USERNAME" \
EMS_MQTT_PASSWORD="$EMS_MQTT_PASSWORD" \
EMS_MQTT_TOPIC_PREFIX="$EMS_MQTT_TOPIC_PREFIX" \
RUST_LOG=info \
target/debug/ems-api >"$log_file" 2>&1 &
api_pid=$!
trap 'kill "$api_pid" >/dev/null 2>&1 || true' EXIT

echo "ems-api started: $base_url (pid=$api_pid, log=$log_file)"

for _ in $(seq 1 60); do
  if curl -fsS "$base_url/readyz" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

if ! curl -fsS "$base_url/readyz" >/dev/null 2>&1; then
  echo "ems-api not ready, see log: $log_file" >&2
  exit 1
fi

ACCESS_TOKEN=$(
  curl -fsS -X POST "$base_url/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"admin123"}' \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["accessToken"])'
)
AUTH_HEADER="Authorization: Bearer $ACCESS_TOKEN"

METRICS_BEFORE_INGEST=$(curl -fsS "$base_url/metrics" -H "$AUTH_HEADER")
RAW_BEFORE_INGEST=$(
  python3 -c 'import json,sys; print((json.load(sys.stdin).get("data") or {}).get("rawEvents", 0))' \
    <<<"$METRICS_BEFORE_INGEST"
)

PROJECT_ID=$(
  curl -fsS -X POST "$base_url/projects" \
    -H "Content-Type: application/json" -H "$AUTH_HEADER" \
    -d '{"name":"acceptance-project","timezone":"UTC"}' \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["projectId"])'
)

cleanup_db() {
  if [ -z "${PROJECT_ID:-}" ]; then
    return 0
  fi
  psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 \
    -v tenant_id="$EMS_TENANT_ID" -v project_id="$PROJECT_ID" <<'SQL' >/dev/null 2>&1 || true
delete from command_receipts where tenant_id = :'tenant_id' and project_id = :'project_id';
delete from commands where tenant_id = :'tenant_id' and project_id = :'project_id';
delete from audit_logs where tenant_id = :'tenant_id' and project_id = :'project_id';
delete from event where tenant_id = :'tenant_id' and project_id = :'project_id';
delete from measurement where tenant_id = :'tenant_id' and project_id = :'project_id';
delete from point_sources where tenant_id = :'tenant_id' and project_id = :'project_id';
delete from points where tenant_id = :'tenant_id' and project_id = :'project_id';
delete from devices where tenant_id = :'tenant_id' and project_id = :'project_id';
delete from gateways where tenant_id = :'tenant_id' and project_id = :'project_id';
delete from projects where tenant_id = :'tenant_id' and project_id = :'project_id';
SQL
}

trap 'cleanup_db; kill "$api_pid" >/dev/null 2>&1 || true' EXIT

GATEWAY_ID=$(
  curl -fsS -X POST "$base_url/projects/$PROJECT_ID/gateways" \
    -H "Content-Type: application/json" -H "$AUTH_HEADER" \
    -d '{"name":"gw-acceptance","status":"online"}' \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["gatewayId"])'
)

DEVICE_ID=$(
  curl -fsS -X POST "$base_url/projects/$PROJECT_ID/devices" \
    -H "Content-Type: application/json" -H "$AUTH_HEADER" \
    -d "$(python3 -c 'import json,sys; print(json.dumps({"gatewayId": sys.argv[1], "name": "dev-acceptance", "model": "demo"}))' "$GATEWAY_ID")" \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["deviceId"])'
)

POINT_ID=$(
  curl -fsS -X POST "$base_url/projects/$PROJECT_ID/points" \
    -H "Content-Type: application/json" -H "$AUTH_HEADER" \
    -d "$(python3 -c 'import json,sys; print(json.dumps({"deviceId": sys.argv[1], "key": "p1", "dataType": "f64", "unit": ""}))' "$DEVICE_ID")" \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["pointId"])'
)

curl -fsS -X POST "$base_url/projects/$PROJECT_ID/point-mappings" \
  -H "Content-Type: application/json" -H "$AUTH_HEADER" \
  -d "$(python3 -c 'import json,sys; print(json.dumps({"pointId": sys.argv[1], "sourceType": "mqtt", "address": sys.argv[2]}))' "$POINT_ID" "$EMS_POINT_ADDRESS")" \
  >/dev/null

echo "resources created: projectId=$PROJECT_ID gatewayId=$GATEWAY_ID deviceId=$DEVICE_ID pointId=$POINT_ID"

# publish enough messages to trigger batch flush immediately, and also rely on periodic flush.
for _ in $(seq 1 5); do
  EMS_TENANT_ID="$EMS_TENANT_ID" \
  EMS_PROJECT_ID="$PROJECT_ID" \
  EMS_POINT_ADDRESS="$EMS_POINT_ADDRESS" \
  EMS_PAYLOAD="$EMS_POINT_PAYLOAD" \
  EMS_MQTT_HOST="$EMS_MQTT_HOST" \
  EMS_MQTT_PORT="$EMS_MQTT_PORT" \
  EMS_MQTT_USERNAME="$EMS_MQTT_USERNAME" \
  EMS_MQTT_PASSWORD="$EMS_MQTT_PASSWORD" \
  EMS_MQTT_TOPIC_PREFIX="$EMS_MQTT_TOPIC_PREFIX" \
  scripts/mqtt-simulate.sh >/dev/null
done

sleep 2

curl -fsS "$base_url/metrics" -H "$AUTH_HEADER" \
  | python3 -c 'import json,sys; before=int(sys.argv[1]); data=json.load(sys.stdin).get("data") or {}; raw=int(data.get("rawEvents") or 0); assert raw >= before + 1; print(f"metrics ingest ok (rawEvents={raw})")' \
  "$RAW_BEFORE_INGEST"

for _ in $(seq 1 20); do
  resp=$(curl -fsS "$base_url/projects/$PROJECT_ID/gateways?limit=50" -H "$AUTH_HEADER" || true)
  if [ -n "$resp" ] && python3 -c 'import json,sys; gateway_id=sys.argv[1]; data=json.loads(sys.stdin.read()); items=data.get("data") or []; ok=data.get("success") is True and any(x.get("gatewayId")==gateway_id and x.get("online") is True and isinstance(x.get("lastSeenAtMs"), int) for x in items); sys.exit(0 if ok else 1)' "$GATEWAY_ID" <<<"$resp"; then
    break
  fi
  sleep 0.5
done

curl -fsS "$base_url/projects/$PROJECT_ID/gateways?limit=50" \
  -H "$AUTH_HEADER" \
  | python3 -c 'import json,sys; gateway_id=sys.argv[1]; data=json.load(sys.stdin); items=data.get("data") or []; assert data.get("success") is True; assert any(x.get("gatewayId")==gateway_id and x.get("online") is True and isinstance(x.get("lastSeenAtMs"), int) for x in items); print("gateway online ok")' \
  "$GATEWAY_ID"

for _ in $(seq 1 20); do
  resp=$(curl -fsS "$base_url/projects/$PROJECT_ID/devices?limit=50" -H "$AUTH_HEADER" || true)
  if [ -n "$resp" ] && python3 -c 'import json,sys; device_id=sys.argv[1]; data=json.loads(sys.stdin.read()); items=data.get("data") or []; ok=data.get("success") is True and any(x.get("deviceId")==device_id and x.get("online") is True and isinstance(x.get("lastSeenAtMs"), int) for x in items); sys.exit(0 if ok else 1)' "$DEVICE_ID" <<<"$resp"; then
    break
  fi
  sleep 0.5
done

curl -fsS "$base_url/projects/$PROJECT_ID/devices?limit=50" \
  -H "$AUTH_HEADER" \
  | python3 -c 'import json,sys; device_id=sys.argv[1]; data=json.load(sys.stdin); items=data.get("data") or []; assert data.get("success") is True; assert any(x.get("deviceId")==device_id and x.get("online") is True and isinstance(x.get("lastSeenAtMs"), int) for x in items); print("device online ok")' \
  "$DEVICE_ID"

curl -fsS "$base_url/projects/$PROJECT_ID/realtime?pointId=$POINT_ID" \
  -H "$AUTH_HEADER" \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); assert data["success"] is True; assert data["data"] is not None; print("realtime ok")'

curl -fsS "$base_url/projects/$PROJECT_ID/measurements?pointId=$POINT_ID&limit=10" \
  -H "$AUTH_HEADER" \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); assert data["success"] is True; assert isinstance(data.get("data"), list) and len(data["data"]) > 0; print("measurements ok")'

curl -fsS "$base_url/projects/$PROJECT_ID/measurements?pointId=$POINT_ID&bucketMs=1000&agg=count&limit=10" \
  -H "$AUTH_HEADER" \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); assert data["success"] is True; items=data.get("data") or []; assert len(items) > 0; assert all("tsMs" in x and "value" in x for x in items); print("measurements aggregate ok")'

METRICS_BEFORE_CONTROL=$(curl -fsS "$base_url/metrics" -H "$AUTH_HEADER")
COMMANDS_BEFORE=$(
  python3 -c 'import json,sys; print((json.load(sys.stdin).get("data") or {}).get("commandsIssued", 0))' \
    <<<"$METRICS_BEFORE_CONTROL"
)
RECEIPTS_BEFORE=$(
  python3 -c 'import json,sys; print((json.load(sys.stdin).get("data") or {}).get("receiptsProcessed", 0))' \
    <<<"$METRICS_BEFORE_CONTROL"
)

COMMAND_ID=$(
  curl -fsS -X POST "$base_url/projects/$PROJECT_ID/commands" \
    -H "Content-Type: application/json" -H "$AUTH_HEADER" \
    -d '{"target":"demo-target","payload":{"action":"set","value":42}}' \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["commandId"])'
)

EMS_TENANT_ID="$EMS_TENANT_ID" \
EMS_PROJECT_ID="$PROJECT_ID" \
EMS_COMMAND_ID="$COMMAND_ID" \
EMS_MQTT_HOST="$EMS_MQTT_HOST" \
EMS_MQTT_PORT="$EMS_MQTT_PORT" \
EMS_MQTT_USERNAME="$EMS_MQTT_USERNAME" \
EMS_MQTT_PASSWORD="$EMS_MQTT_PASSWORD" \
EMS_MQTT_TOPIC_PREFIX="$EMS_MQTT_TOPIC_PREFIX" \
scripts/control-receipt-simulate.sh >/dev/null

for _ in $(seq 1 20); do
  resp=$(curl -fsS "$base_url/projects/$PROJECT_ID/commands/$COMMAND_ID/receipts" -H "$AUTH_HEADER" || true)
  if [ -n "$resp" ] && python3 -c 'import json,sys; data=json.loads(sys.stdin.read()); ok=data.get("success") is True and isinstance(data.get("data"), list) and len(data["data"])>0; sys.exit(0 if ok else 1)' <<<"$resp"; then
    break
  fi
  sleep 0.5
done

curl -fsS "$base_url/projects/$PROJECT_ID/commands/$COMMAND_ID/receipts" \
  -H "$AUTH_HEADER" \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); assert data["success"] is True; assert isinstance(data.get("data"), list) and len(data["data"]) > 0; print("receipts ok")'

for _ in $(seq 1 20); do
  resp=$(curl -fsS "$base_url/projects/$PROJECT_ID/audit?limit=50" -H "$AUTH_HEADER" || true)
  if [ -n "$resp" ] && python3 -c 'import json,sys; data=json.loads(sys.stdin.read()); items=data.get("data") or []; ok=data.get("success") is True and any(x.get("action")=="CONTROL.COMMAND.RECEIPT" for x in items); sys.exit(0 if ok else 1)' <<<"$resp"; then
    break
  fi
  sleep 0.5
done

curl -fsS "$base_url/projects/$PROJECT_ID/audit?limit=50" \
  -H "$AUTH_HEADER" \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); assert data["success"] is True; items=data.get("data") or []; assert any(x.get("action")=="CONTROL.COMMAND.RECEIPT" for x in items); print("audit ok")'

for _ in $(seq 1 20); do
  resp=$(curl -fsS "$base_url/projects/$PROJECT_ID/commands?limit=50" -H "$AUTH_HEADER" || true)
  if [ -z "$resp" ]; then
    sleep 0.5
    continue
  fi
  if python3 -c 'import json,sys; command_id=sys.argv[1]; data=json.loads(sys.stdin.read()); items=data.get("data") or []; ok=(data.get("success") is True and any(x.get("commandId")==command_id and x.get("status") in ("success","accepted","failed","timeout") for x in items)); sys.exit(0 if ok else 1)' "$COMMAND_ID" <<<"$resp"; then
    break
  fi
  sleep 0.5
done

curl -fsS "$base_url/projects/$PROJECT_ID/commands?limit=50" \
  -H "$AUTH_HEADER" \
  | python3 -c 'import json,sys; command_id=sys.argv[1]; data=json.load(sys.stdin); assert data["success"] is True; items=data.get("data") or []; assert any(x.get("commandId")==command_id and x.get("status") in ("success","accepted","failed","timeout") for x in items); print("command status ok")' \
  "$COMMAND_ID"

curl -fsS "$base_url/metrics" -H "$AUTH_HEADER" \
  | python3 -c 'import json,sys; commands_before=int(sys.argv[1]); receipts_before=int(sys.argv[2]); data=json.load(sys.stdin).get("data") or {}; commands=int(data.get("commandsIssued") or 0); receipts=int(data.get("receiptsProcessed") or 0); assert commands >= commands_before + 1; assert receipts >= receipts_before + 1; print(f"metrics control ok (commandsIssued={commands}, receiptsProcessed={receipts})")' \
  "$COMMANDS_BEFORE" "$RECEIPTS_BEFORE"

echo "acceptance ok"
