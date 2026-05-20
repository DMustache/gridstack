#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
mkdir -p generated/db
run_logged "generated/logs/db-schema-dump.log" bash -lc 'pg_dump --schema-only --no-owner --no-privileges -U admin -d gridstack -h 127.0.0.1 -p 5432 > generated/db/schema.sql'
test -s generated/db/schema.sql
