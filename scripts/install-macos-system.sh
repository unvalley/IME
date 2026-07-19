#!/usr/bin/env bash
set -euo pipefail

workspace_dir="$(cd "$(dirname "$0")/.." && pwd)"
source_bundle="$workspace_dir/target/macos/UnvalleyIME.app"
user_bundle="$HOME/Library/Input Methods/Unvalley.app"
system_bundle="/Library/Input Methods/Unvalley.app"
user_backup="$workspace_dir/target/macos/UnvalleyIME.user-install-backup.$$.app"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS input method can only be installed on macOS" >&2
  exit 1
fi

pkill -x Unvalley 2>/dev/null || true

osascript \
  "$workspace_dir/platforms/macos/InstallSystem.applescript" \
  "$source_bundle" \
  "$system_bundle"

if [[ -d "$user_bundle" ]]; then
  mv "$user_bundle" "$user_backup"
  echo "Moved the previous user install to $user_backup"
fi

"$workspace_dir/target/macos/register-input-source" \
  "$system_bundle" \
  com.unvalley.inputmethod.Unvalley \
  --register

"$workspace_dir/target/macos/register-input-source" \
  "$system_bundle" \
  com.unvalley.inputmethod.Unvalley \
  --select >/dev/null 2>&1 || true

if "$workspace_dir/target/macos/register-input-source" \
  "$system_bundle" \
  com.unvalley.inputmethod.Unvalley \
  --select-id com.unvalley.inputmethod.Unvalley.Japanese; then
  echo "Installed and selected $system_bundle"
else
  echo "Installed $system_bundle"
  echo "macOS refused to enable the input source."
  codesign -dv --verbose=2 "$system_bundle" 2>&1 \
    | grep -E '^(Identifier|Authority|TeamIdentifier|Signature)=' || true
fi
