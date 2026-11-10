#!/usr/bin/env bash
set -euo pipefail
# Unsigned distribution (ad-hoc code signature only). No Apple Developer account required.
# This does not bypass Gatekeeper; a real device owner may need to approve the app.
LABEL="${1:?Expected platform label}"
case "$LABEL" in
  macOS-Apple-Silicon) ARCH=arm64 ;;
  macOS-Intel) ARCH=x86_64 ;;
  *) echo "Unknown macOS platform label: $LABEL" >&2; exit 1 ;;
esac
# Keep a verifiable source pointer for the AGPL component and this exact build.
git diff --quiet HEAD -- || { echo "Package from a clean committed checkout." >&2; exit 1; }
SOURCE_REVISION="$(git rev-parse --verify HEAD)"
[[ "$SOURCE_REVISION" =~ ^[0-9a-f]{40}$ ]] || { echo "Invalid source revision" >&2; exit 1; }
APP="dist/YTRemote.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "dist"
cp target/release/yt-remote-desktop "$APP/Contents/MacOS/YTRemote"
chmod +x "$APP/Contents/MacOS/YTRemote"
lipo "$APP/Contents/MacOS/YTRemote" -verify_arch "$ARCH"
cp COPYING THIRD_PARTY_NOTICES.md "$APP/Contents/Resources/"
cp docs/MACOS-PERMISSIONS.md "$APP/Contents/Resources/"
cat > "$APP/Contents/Resources/SOURCE.txt" <<SOURCE
YT Remote 3.0 experimental build; not notarized or physically verified.
Corresponding source revision: ${SOURCE_REVISION}
Source: https://github.com/klckylmz-art/yt-remote-2/tree/${SOURCE_REVISION}
Archive: https://github.com/klckylmz-art/yt-remote-2/archive/${SOURCE_REVISION}.tar.gz
Build instructions and dependencies: README.md and Cargo.lock at that revision.
On the matching macOS architecture, install Rust and Apple's command-line tools,
then run: cargo build --locked --release -p yt-remote-desktop
Package from that clean checkout using: bash packaging/macos/package.sh ${LABEL}
See COPYING and THIRD_PARTY_NOTICES.md for license terms and upstream source.
SOURCE
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleExecutable</key><string>YTRemote</string>
  <key>CFBundleIdentifier</key><string>net.ytremote.desktop</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>YT Remote 3.0</string>
  <key>CFBundleDisplayName</key><string>YT Remote 3.0</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>3.0.0</string>
  <key>CFBundleVersion</key><string>3</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSScreenCaptureUsageDescription</key><string>Share your screen with the teacher only after you explicitly approve this session. Stop sharing at any time.</string>
  <key>NSHumanReadableCopyright</key><string>YT Remote development build</string>
</dict>
</plist>
PLIST
codesign --force --sign - --timestamp=none "$APP"
codesign --verify --deep --verbose=2 "$APP"
ditto -c -k --sequesterRsrc --keepParent "$APP" "dist/YTRemote-${LABEL}.zip"
hdiutil create -volname "YT Remote 3.0" -srcfolder "$APP" -ov -format UDZO "dist/YTRemote-${LABEL}.dmg"
echo "macOS development .app and .dmg generated. NOT notarized or end-to-end tested."
