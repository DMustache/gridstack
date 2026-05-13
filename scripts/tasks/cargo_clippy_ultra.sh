#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
run_logged "generated/logs/cargo-clippy-ultra.log" cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery -A clippy::single_call_fn -A clippy::missing_errors_doc -A clippy::must_use_candidate
