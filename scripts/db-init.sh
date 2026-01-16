#!/usr/bin/env bash
set -euo pipefail

if [ -z "${EMS_DATABASE_URL:-}" ]; then
  EMS_DATABASE_URL="postgresql://ems:admin123@localhost:5432/ems"
  export EMS_DATABASE_URL
fi

psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/001_init.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/003_assets.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/004_timescale.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/005_control.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/006_rbac.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/007_auth_sessions.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/002_seed.sql

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
