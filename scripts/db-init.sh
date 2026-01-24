#!/usr/bin/env bash
set -euo pipefail

: "${EMS_DATABASE_URL:?EMS_DATABASE_URL is required (e.g. postgresql://user:pass@host:5432/db)}"

gen_password() {
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -base64 24 | tr -d '\n'
  else
    python3 - <<'PY'
import secrets
print(secrets.token_urlsafe(24))
PY
  fi
}

hash_password() {
  local password="$1"
  local -a cmd
  # Prefer debug binary (more likely to match current workspace sources).
  if [ -x "./target/debug/ems-api" ]; then
    cmd=( ./target/debug/ems-api hash-password "$password" )
  elif [ -x "./target/release/ems-api" ]; then
    cmd=( ./target/release/ems-api hash-password "$password" )
  else
    cmd=( cargo run -q -p ems-api -- hash-password "$password" )
  fi

  if command -v timeout >/dev/null 2>&1; then
    timeout 20s "${cmd[@]}" || {
      echo "hash-password timed out or failed; build ems-api then retry, or set EMS_SEED_ADMIN_PASSWORD_HASH/EMS_SEED_ADMIN2_PASSWORD_HASH to bypass hashing" >&2
      return 1
    }
  else
    "${cmd[@]}"
  fi
}

admin_password="${EMS_SEED_ADMIN_PASSWORD:-}"
if [ -z "$admin_password" ]; then
  admin_password="$(gen_password)"
  echo "generated initial tenant-1 admin password (store it safely): $admin_password" >&2
fi
admin_hash="${EMS_SEED_ADMIN_PASSWORD_HASH:-}"
if [ -z "$admin_hash" ]; then
  admin_hash="$(hash_password "$admin_password")"
fi

admin2_password="${EMS_SEED_ADMIN2_PASSWORD:-}"
if [ -z "$admin2_password" ]; then
  admin2_password="$(gen_password)"
  echo "generated initial tenant-2 admin2 password (store it safely): $admin2_password" >&2
fi
admin2_hash="${EMS_SEED_ADMIN2_PASSWORD_HASH:-}"
if [ -z "$admin2_hash" ]; then
  admin2_hash="$(hash_password "$admin2_password")"
fi

psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/001_init.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/003_assets.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/004_timescale.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/005_control.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/006_rbac.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/007_auth_sessions.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 \
  -v EMS_SEED_ADMIN_PASSWORD_HASH="$admin_hash" \
  -v EMS_SEED_ADMIN2_PASSWORD_HASH="$admin2_hash" \
  -f migrations/002_seed.sql

require_timescale="${EMS_REQUIRE_TIMESCALE:-}"
require_timescale="$(echo "$require_timescale" | tr '[:upper:]' '[:lower:]')"
if [ "$require_timescale" = "1" ] || [ "$require_timescale" = "true" ] || [ "$require_timescale" = "on" ]; then
  has_timescale="$(psql "$EMS_DATABASE_URL" -Atqc "select 1 from pg_extension where extname='timescaledb' limit 1" || true)"
  if [ "$has_timescale" != "1" ]; then
    echo "timescaledb extension is required (EMS_REQUIRE_TIMESCALE=on)" >&2
    exit 1
  fi
fi

echo "db init ok"
