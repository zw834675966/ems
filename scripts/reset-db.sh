#!/bin/bash
set -e

# Ensure cargo bin is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

echo "Recreating DB..."

sudo -u postgres psql -p 5433 -c "DROP DATABASE IF EXISTS ems;"
sudo -u postgres psql -p 5433 -c "DROP USER IF EXISTS ems;"
sudo -u postgres psql -p 5433 -c "CREATE USER ems WITH PASSWORD 'admin123';"
sudo -u postgres psql -p 5433 -c "CREATE DATABASE ems OWNER ems;"
sudo -u postgres psql -p 5433 -c "ALTER USER ems WITH SUPERUSER;"
sudo -u postgres psql -p 5433 -d ems -c "CREATE EXTENSION IF NOT EXISTS timescaledb;"

echo "Running migrations..."
export DATABASE_URL=postgres://ems:admin123@localhost:5433/ems

sqlx migrate run

echo "Done."
