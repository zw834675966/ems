#!/usr/bin/env bash
set -euo pipefail

: "${EMS_DATABASE_URL:?EMS_DATABASE_URL is required}"

psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/001_init.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/003_assets.sql
psql "$EMS_DATABASE_URL" -v ON_ERROR_STOP=1 -f migrations/002_seed.sql

echo "db init ok"
