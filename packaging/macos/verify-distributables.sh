#!/usr/bin/env bash
set -euo pipefail

# CI packaging smoke test. This is NOT an application launch or Gatekeeper test.
label="${1:?Pass macOS-Apple-Silicon or macOS-Intel}"
case "$label" in
  macOS-Apple-Silicon) arch=arm64 ;;
  macOS-Intel) arch=x86_64 ;;
  *) echo "Unsupported label: $label" >&2; exit 2 ;;
esac

zip="dist/YTRemote-${label}.zip"
dmg="dist/YTRemote-${label}.dmg"
for archive in "$zip" "$dmg"; do
  test -s "$archive" || { echo "Missing or empty archive: $archive" >&2; exit 1; }
done

workdir="$(mktemp -d)"
mounted=0
cleanup() {
  if [[ "$mounted" == 1 ]]; then
    hdiutil detach "$workdir/mount" -quiet || echo 'WARNING: DMG detach failed' >&2
  fi
  rm -rf "$workdir"
}
trap cleanup EXIT
mkdir -p "$workdir/unzip" "$workdir/mount"

ditto -x -k "$zip" "$workdir/unzip"
check_app() {
  local app="$1" binary="$1/Contents/MacOS/YTRemote"
  test -x "$binary" || { echo "Missing executable in $app" >&2; exit 1; }
  test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app/Contents/Info.plist")" = YTRemote
  lipo "$binary" -verify_arch "$arch"
  codesign --verify --deep --strict "$app"
}
check_app "$workdir/unzip/YTRemote.app"

# Verify the distributable disk image itself, not only the staging .app.
hdiutil attach "$dmg" -readonly -nobrowse -quiet -mountpoint "$workdir/mount"
mounted=1
check_app "$workdir/mount/YTRemote.app"
echo "Verified ZIP and mounted DMG contents for $label ($arch); runtime launch not tested."
