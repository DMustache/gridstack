#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/lib.sh"

mkdir -p temp generated/openapi generated/logs

log_file="generated/logs/matrix-openapi.log"

run_logged "$log_file" bash -lc '
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

  cp ./scripts/openapi/api-docs.json ../../generated/openapi/matrix-openapi.json
' 

run_logged "$log_file" docker run --rm \
  -v "$PWD:/spec" \
  redocly/cli build-docs \
  generated/openapi/matrix-openapi.json \
  -o generated/openapi/matrix-redocly.html

test -s generated/openapi/matrix-openapi.json
test -s generated/openapi/matrix-redocly.html
