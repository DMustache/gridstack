#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
run_logged "generated/logs/cargo-check.log" cargo check
