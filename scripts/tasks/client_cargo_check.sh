#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
LOG_DIR="$ROOT_DIR/generated/logs"
LOG_FILE="$LOG_DIR/client-cargo-check.log"

mkdir -p "$LOG_DIR"

if cargo check --manifest-path "$ROOT_DIR/client/Cargo.toml" >"$LOG_FILE" 2>&1; then
  exit 0
fi

cat "$LOG_FILE"
exit 1
