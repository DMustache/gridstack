#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
mkdir -p generated/openapi
run_logged "generated/logs/openapi-export.log" cargo run --bin export_openapi -- generated/openapi/gridstack-openapi.json
