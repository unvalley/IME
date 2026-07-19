#!/usr/bin/env bash
set -euo pipefail

workspace_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$workspace_dir"

cargo build --release -p ime-ffi
cc \
  -std=c11 \
  -Wall \
  -Wextra \
  -Werror \
  -I crates/ime-ffi/include \
  crates/ime-ffi/tests/ffi_smoke.c \
  -L target/release \
  -lime_ffi \
  -o target/ffi-smoke

DYLD_LIBRARY_PATH="$workspace_dir/target/release" target/ffi-smoke

