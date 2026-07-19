#!/usr/bin/env bash
set -euo pipefail

workspace_dir="$(cd "$(dirname "$0")/.." && pwd)"
source_bundle="$workspace_dir/target/macos/UnvalleyIME.app"
input_methods_dir="$HOME/Library/Input Methods"
destination="$input_methods_dir/Unvalley.app"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS input method can only be installed on macOS" >&2
  exit 1
fi

if [[ "$destination" != "$HOME/Library/Input Methods/Unvalley.app" ]]; then
  echo "refusing to install to unexpected path: $destination" >&2
  exit 1
fi

pkill -x Unvalley 2>/dev/null || true
mkdir -p "$input_methods_dir"
ditto "$source_bundle" "$destination"

"$workspace_dir/target/macos/register-input-source" \
  "$destination" \
  com.unvalley.inputmethod.Unvalley \
  --register

open "$destination" || true
sleep 1
pkill -x Unvalley 2>/dev/null || true

"$workspace_dir/target/macos/register-input-source" \
  "$destination" \
  com.unvalley.inputmethod.Unvalley \
  --select >/dev/null 2>&1 || true

if "$workspace_dir/target/macos/register-input-source" \
  "$destination" \
  com.unvalley.inputmethod.Unvalley \
  --select-id com.unvalley.inputmethod.Unvalley.Japanese; then
  echo "Installed and selected $destination"
else
  echo "Installed $destination"
  echo "First install: add Hiragana (Unvalley IME) from Keyboard > Input Sources."
  open 'x-apple.systempreferences:com.apple.Keyboard-Settings.extension'
fi
