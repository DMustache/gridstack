#!/usr/bin/env bash
# Compact, token-cheap map of the repository for agents.
# Output: generated/reports/repo-map.md
set -euo pipefail
cd "$(dirname "$0")/../.."
out="generated/reports/repo-map.md"
mkdir -p "$(dirname "$out")"
{
  echo "# Repo map ($(git rev-parse --short HEAD))"
  echo
  echo "## Source files (lines)"
  find src tests response_derive/src -name '*.rs' 2>/dev/null | sort | xargs wc -l | sed '$d' | awk '{printf "- %s (%s)\n", $2, $1}'
  echo
  echo "## Routes"
  grep -rn --include=*.rs -A1 '\.route(' src | grep -oE '"/_matrix/[^"]+"' | sort -u | sed 's/^/- /'
  echo
  echo "## Public traits (ports)"
  grep -rn --include=*.rs -E '^\s*pub(\(crate\))? trait ' src | sed -E 's/\{.*//; s/^/- /'
  echo
  echo "## Migrations"
  ls migrations | sed 's/^/- /'
  echo
  echo "## Test locations"
  grep -rln --include=*.rs -E '#\[(tokio::)?test' src tests | sort | sed 's/^/- /'
} >"$out"
test -s "$out"
echo "wrote $out"
