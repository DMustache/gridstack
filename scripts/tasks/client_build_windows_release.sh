#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/lib.sh"

LOG_FILE="generated/logs/client-build-windows-release.log"
TARGET_TRIPLE="x86_64-pc-windows-gnu"
ARTIFACT_PATH="client/target/${TARGET_TRIPLE}/release/client.exe"

run_logged "$LOG_FILE" rustup target add "$TARGET_TRIPLE"
run_logged "$LOG_FILE" cargo build --manifest-path client/Cargo.toml --target "$TARGET_TRIPLE" --release

if [ ! -s "$ARTIFACT_PATH" ]; then
  echo "Task failed: expected artifact missing: $ARTIFACT_PATH" >&2
  if [ -f "$LOG_FILE" ]; then
    echo "----- BEGIN LOG -----" >&2
    cat "$LOG_FILE" >&2
    echo "----- END LOG -----" >&2
  fi
  exit 1
fi
