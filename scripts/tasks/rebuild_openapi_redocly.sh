#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
mkdir -p generated/openapi
run_logged "generated/logs/openapi-redocly-rebuild.log" bash -lc 'cargo run --bin export_openapi -- generated/openapi/gridstack-openapi.json && docker run --rm -v "$PWD:/spec" redocly/cli build-docs generated/openapi/gridstack-openapi.json -o generated/openapi/gridstack-redocly.html'
