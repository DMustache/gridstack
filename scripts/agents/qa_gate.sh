#!/usr/bin/env bash
# Single QA gate used by workers before handoff and by the lead before merge.
# Runs the registered task scripts in order and stops at the first failure.
# Output: generated/reports/qa-gate.txt (one line per step: PASS|FAIL label)
set -uo pipefail
cd "$(dirname "$0")/../.."
report="generated/reports/qa-gate.txt"
mkdir -p "$(dirname "$report")"
: >"$report"

steps=(
  "Cargo: Format|scripts/tasks/cargo_fmt.sh"
  "Cargo: Check|scripts/tasks/cargo_check.sh"
  "Cargo: Ultra Clippy (Strict)|scripts/tasks/cargo_clippy_ultra.sh"
  "Cargo: Test|scripts/tasks/cargo_test.sh"
)

for step in "${steps[@]}"; do
  label="${step%%|*}"
  script="${step#*|}"
  if "./$script" >/dev/null 2>"generated/reports/qa-gate.stderr"; then
    echo "PASS $label" | tee -a "$report"
  else
    echo "FAIL $label" | tee -a "$report"
    # Print only the tail of the failure to keep agent context small.
    tail -n 60 "generated/reports/qa-gate.stderr"
    exit 1
  fi
done
echo "QA GATE PASSED" | tee -a "$report"
