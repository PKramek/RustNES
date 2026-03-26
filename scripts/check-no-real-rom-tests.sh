#!/bin/sh

set -eu

if rg -n --glob 'tests/**/*.rs' 'join\("roms/|join\("nestest/|PathBuf::from\(env!\("CARGO_MANIFEST_DIR"\)\)\.join\("roms/|PathBuf::from\(env!\("CARGO_MANIFEST_DIR"\)\)\.join\("nestest/' tests; then
  echo
  echo 'Committed tests must not load local roms/ or nestest/ assets.' >&2
  echo 'Use generated ROM bytes or synthetic fixtures under tests/support instead.' >&2
  exit 1
fi
