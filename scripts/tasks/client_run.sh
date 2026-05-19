#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
LOG_DIR="$ROOT_DIR/generated/logs"
LOG_FILE="$LOG_DIR/client-run.log"

mkdir -p "$LOG_DIR"

if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
  echo "No graphical display detected (DISPLAY/WAYLAND_DISPLAY not set)." >"$LOG_FILE"
  echo "Run this task from a desktop session." >>"$LOG_FILE"
  cat "$LOG_FILE"
  exit 1
fi

WINIT_BACKEND="x11"
if [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
  WINIT_BACKEND="wayland"
fi

{
  echo "Client run backend: ${WINIT_BACKEND}"
  echo "DISPLAY=${DISPLAY:-}"
  echo "WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-}"
} >"$LOG_FILE"

if LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND="$WINIT_BACKEND" RUST_LOG=info cargo run --manifest-path "$ROOT_DIR/client/Cargo.toml" >>"$LOG_FILE" 2>&1; then
  exit 0
fi

cat "$LOG_FILE"
exit 1
