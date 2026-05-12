#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/lib.sh"

mkdir -p temp generated/logs

run_logged "generated/logs/tuwunel-sync.log" bash -lc '
  cd temp

  if [ ! -d "tuwunel" ]; then
    git clone https://github.com/matrix-construct/tuwunel.git
  else
    cd tuwunel
    git pull
  fi
'

test -s temp/tuwunel/.git/HEAD
