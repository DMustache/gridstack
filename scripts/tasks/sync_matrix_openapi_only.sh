#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
mkdir -p generated/openapi generated/logs temp
run_logged "generated/logs/matrix-openapi-only.log" bash -lc '
  cd temp
  if [ ! -d "matrix-spec" ]; then
    git clone https://github.com/matrix-org/matrix-spec.git
  fi
  cd matrix-spec
  git pull
  python3 -m venv venv
  source venv/bin/activate
  pip install -r ./scripts/requirements.txt
  python ./scripts/dump-openapi.py
  mkdir -p ../../generated/openapi/
  cp ./scripts/openapi/api-docs.json ../../generated/openapi/matrix-openapi.json
'
test -s generated/openapi/matrix-openapi.json
