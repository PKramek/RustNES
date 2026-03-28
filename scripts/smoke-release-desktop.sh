#!/bin/sh

set -eu

run_cmd() {
  echo "+ $*"
  "$@"
}

run_cmd sh scripts/check-no-real-rom-tests.sh
run_cmd cargo build --release
run_cmd cargo test --release --test desktop_product --test runtime_view --test shell_diagnostics --test trace_cli