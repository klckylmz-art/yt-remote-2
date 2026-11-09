#!/usr/bin/env bash
set -euo pipefail
# Unsigned distribution (ad-hoc code signature only). No Apple Developer account required.
# This does not bypass Gatekeeper; a real device owner may need to approve the app.
LABEL="${1:?Expected platform label}"
APP="dist/YTRemote.app"
mkdir -p "$APP/Contents/MacOS" "dist"
cp target/release/yt-remote-desktop "$APP/Contents/MacOS/YTRemote"
chmod +x "$APP/Contents/MacOS/YTRemote"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleExecutable</key><string>YTRemote</string>
  <key>CFBundleIdentifier</key><string>net.ytremote.desktop</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>YT Remote</string>
  <key>CFBundleDisplayName</key><string>YT Remote</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>YT Remote development build</string>
</dict>
</plist>
PLIST
codesign --force --sign - --timestamp=none "$APP"
codesign --verify --deep --verbose=2 "$APP"
ditto -c -k --sequesterRsrc --keepParent "$APP" "dist/YTRemote-${LABEL}.zip"
hdiutil create -volname "YT Remote" -srcfolder "$APP" -ov -format UDZO "dist/YTRemote-${LABEL}.dmg"
echo "macOS development .app and .dmg generated. NOT notarized or end-to-end tested."
