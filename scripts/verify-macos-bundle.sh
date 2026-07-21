#!/usr/bin/env bash
set -euo pipefail

workspace_dir="$(cd "$(dirname "$0")/.." && pwd)"
bundle_dir="$workspace_dir/target/macos/UnvalleyIME.app"
executable="$bundle_dir/Contents/MacOS/Unvalley"

test -x "$executable"
test -f "$bundle_dir/Contents/Frameworks/libime_ffi.dylib"
test -f "$bundle_dir/Contents/Resources/InputMethodIcon.tiff"
test -f "$bundle_dir/Contents/Resources/MOZC_DICTIONARY_LICENSE.txt"
test -f "$bundle_dir/Contents/Resources/LICENSE.txt"
test -f "$bundle_dir/Contents/Resources/English.lproj/InfoPlist.strings"
test -f "$bundle_dir/Contents/Resources/Japanese.lproj/InfoPlist.strings"
test "$(wc -c < "$bundle_dir/Contents/PkgInfo" | tr -d ' ')" = "8"
test "$(< "$bundle_dir/Contents/PkgInfo")" = "APPL????"
plutil -lint "$bundle_dir/Contents/Info.plist"
codesign --verify --deep --strict "$bundle_dir"

entitlements="$(codesign -d --entitlements - "$bundle_dir" 2>/dev/null)"
if [[ "$entitlements" != *"com.apple.security.get-task-allow"* ]]; then
  echo "development input method entitlement is missing" >&2
  exit 1
fi

if ! otool -L "$executable" | grep -q '@rpath/libime_ffi.dylib'; then
  echo "embedded Rust dylib is not linked through @rpath" >&2
  exit 1
fi

echo "macOS input method bundle verification passed"
