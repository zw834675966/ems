#!/usr/bin/env bash
set -euo pipefail

: "${EMS_HTTP_ADDR:=127.0.0.1:18082}"
: "${EMS_DATABASE_URL:=postgresql://ems:admin123@localhost:5432/ems}"
: "${EMS_JWT_SECRET:=dev}"
: "${EMS_JWT_ACCESS_TTL_SECONDS:=3600}"
: "${EMS_JWT_REFRESH_TTL_SECONDS:=7200}"
: "${EMS_REDIS_URL:=redis://default:admin123@localhost:6379}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl not found" >&2
  exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 not found" >&2
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
EMS_INGEST=off \
EMS_CONTROL=off \
EMS_JWT_SECRET="$EMS_JWT_SECRET" \
EMS_JWT_ACCESS_TTL_SECONDS="$EMS_JWT_ACCESS_TTL_SECONDS" \
EMS_JWT_REFRESH_TTL_SECONDS="$EMS_JWT_REFRESH_TTL_SECONDS" \
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

ADMIN_LOGIN=$(
  curl -sS -X POST "$base_url/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"admin123"}'
)
ADMIN_TOKEN=$(
  python3 -c 'import json,sys; data=json.loads(sys.stdin.read()); assert data.get("success") is True, data; print(data["data"]["accessToken"])' \
    <<<"$ADMIN_LOGIN"
)
ADMIN_AUTH="Authorization: Bearer $ADMIN_TOKEN"

ROLE_CODE=$(
  python3 - <<'PY'
import uuid
print(f"operator-{uuid.uuid4().hex[:6]}")
PY
)
export ROLE_CODE
ROLE_NAME="Operator"

ROLE_CREATED=$(
  curl -sS -X POST "$base_url/rbac/roles" \
    -H "Content-Type: application/json" -H "$ADMIN_AUTH" \
    -d "$(python3 -c 'import json; print(json.dumps({"roleCode":"'"$ROLE_CODE"'","name":"'"$ROLE_NAME"'","permissions":["PROJECT.READ"]}))')"
)
python3 -c 'import json,sys; data=json.loads(sys.stdin.read()); assert data.get("success") is True, data; print("create role ok")' \
  <<<"$ROLE_CREATED"

USER_JSON=$(
  python3 - <<'PY'
import uuid, json
u = f"op-{uuid.uuid4().hex[:8]}"
print(json.dumps({"username": u, "password": "admin123", "status": "active", "roles": [__import__("os").environ["ROLE_CODE"]]}))
PY
)

USER_CREATED=$(
  curl -sS -X POST "$base_url/rbac/users" \
    -H "Content-Type: application/json" -H "$ADMIN_AUTH" \
    -d "$USER_JSON"
)
python3 -c 'import json,sys; data=json.loads(sys.stdin.read()); assert data.get("success") is True, data' \
  <<<"$USER_CREATED" >/dev/null
USER_ID=$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["data"]["userId"])' <<<"$USER_CREATED")
USERNAME=$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["data"]["username"])' <<<"$USER_CREATED")

cleanup_db() {
  if [ -z "${USER_ID:-}" ]; then
    return 0
  fi
  psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 \
    -v tenant_id="tenant-1" -v user_id="$USER_ID" -v role_code="${ROLE_CODE:-}" <<'SQL' >/dev/null 2>&1 || true
delete from tenant_user_roles where tenant_id = :'tenant_id' and user_id = :'user_id';
delete from users where tenant_id = :'tenant_id' and user_id = :'user_id';
delete from tenant_role_permissions where tenant_id = :'tenant_id' and role_code = :'role_code';
delete from tenant_roles where tenant_id = :'tenant_id' and role_code = :'role_code';
SQL
}

trap 'cleanup_db; kill "$api_pid" >/dev/null 2>&1 || true' EXIT

OP_LOGIN=$(
  curl -sS -X POST "$base_url/login" \
    -H "Content-Type: application/json" \
    -d "$(python3 -c 'import json,sys; print(json.dumps({"username": sys.argv[1], "password": "admin123"}))' "$USERNAME")"
)
OP_TOKEN=$(
  python3 -c 'import json,sys; data=json.loads(sys.stdin.read()); assert data.get("success") is True, data; print(data["data"]["accessToken"])' \
    <<<"$OP_LOGIN"
)
OP_AUTH="Authorization: Bearer $OP_TOKEN"

python3 -c 'import json,os,sys; data=json.loads(sys.stdin.read())["data"]; assert os.environ["ROLE_CODE"] in (data.get("roles") or []); perms=set(data.get("permissions") or []); assert "PROJECT.READ" in perms; assert "PROJECT.WRITE" not in perms; assert "RBAC.USER.READ" not in perms; print("operator login ok")' \
  <<<"$OP_LOGIN"

curl -fsS "$base_url/projects" -H "$OP_AUTH" >/dev/null
echo "operator projects read ok"

http_code=$(
  curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "$base_url/projects" \
    -H "$OP_AUTH" -H "Content-Type: application/json" \
    -d '{"name":"should-forbidden","timezone":"UTC"}'
)
if [ "$http_code" != "403" ]; then
  echo "expected 403 for operator project write, got: $http_code" >&2
  exit 1
fi
echo "operator projects write forbidden ok"

http_code=$(
  curl -sS -o /dev/null -w '%{http_code}' \
    "$base_url/rbac/users" -H "$OP_AUTH"
)
if [ "$http_code" != "403" ]; then
  echo "expected 403 for operator rbac users, got: $http_code" >&2
  exit 1
fi
echo "operator rbac forbidden ok"

curl -sS "$base_url/rbac/users" -H "$ADMIN_AUTH" \
  | python3 -c 'import json,sys; user_id=sys.argv[1]; data=json.load(sys.stdin); assert data["success"] is True; items=data.get("data") or []; assert any(x.get("userId")==user_id for x in items); print("admin list users ok")' \
  "$USER_ID"

curl -sS -X PUT "$base_url/rbac/users/$USER_ID" \
  -H "Content-Type: application/json" -H "$ADMIN_AUTH" \
  -d '{"status":"disabled"}' \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); assert data["success"] is True; assert data["data"]["status"]=="disabled"; print("disable user ok")'

curl -sS -X DELETE "$base_url/rbac/roles/$ROLE_CODE" \
  -H "$ADMIN_AUTH" >/dev/null
echo "delete role ok"

echo "rbac acceptance ok"
