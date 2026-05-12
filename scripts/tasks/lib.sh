#!/usr/bin/env bash
set -euo pipefail

run_logged() {
  local log_file="$1"
  shift

  mkdir -p "$(dirname "$log_file")"

  if "$@" >"$log_file" 2>&1; then
    return 0
  fi

  local exit_code=$?
  echo "Task failed (exit $exit_code). Log: $log_file" >&2
  if [ -f "$log_file" ]; then
    echo "----- BEGIN LOG -----" >&2
    cat "$log_file" >&2
    echo "----- END LOG -----" >&2
  fi
  return "$exit_code"
}
