#!/usr/bin/env bash
set -euo pipefail

echo "scripts/db-init.sh is deprecated. Use SQLx migrations instead:" >&2
echo "  sqlx migrate run" >&2
exit 1