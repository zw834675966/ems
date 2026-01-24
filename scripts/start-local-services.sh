#!/usr/bin/env bash
set -euo pipefail

SERVICES=(
  "postgresql"
  "redis-server"
  "mosquitto"
)

start_with_systemctl() {
  for svc in "${SERVICES[@]}"; do
    if systemctl is-active --quiet "$svc"; then
      echo "service $svc already active"
    else
      echo "starting $svc via systemctl"
      sudo systemctl start "$svc"
    fi
  done
}

start_with_service() {
  for svc in "${SERVICES[@]}"; do
    if service "$svc" status >/dev/null 2>&1; then
      echo "service $svc already running"
      continue
    fi
    echo "starting $svc via service"
    sudo service "$svc" start
  done
}

start_with_userd() {
  for svc in "${SERVICES[@]}"; do
    if pg_isready -q -d "postgresql://localhost" >/dev/null 2>&1 && [[ "$svc" == "postgresql" ]]; then
      echo "PostgreSQL appears healthy"
    elif [[ "$svc" == "redis-server" ]]; then
      if redis-cli ping >/dev/null 2>&1; then
        echo "Redis appears healthy"
        continue
      fi
      echo "starting redis-server directly"
      sudo service redis-server start
    fi
  done
}

detect_manager() {
  if command -v systemctl >/dev/null 2>&1; then
    echo "detected systemctl command"
    start_with_systemctl
    return 0
  fi

  if command -v service >/dev/null 2>&1; then
    echo "detected service command"
    start_with_service
    return 0
  fi

  echo "neither systemctl nor service command found"
  return 1
}

if ! detect_manager; then
  echo "running custom checks for services"
  start_with_userd
fi
