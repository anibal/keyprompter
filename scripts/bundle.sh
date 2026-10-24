#!/bin/sh
# Builds target/release/KeyPrompter.app with an ad-hoc signature.
set -eu
cd "$(dirname "$0")/.."
cargo build --release --offline
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
app=target/release/KeyPrompter.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
cp target/release/keyprompter "$app/Contents/MacOS/KeyPrompter"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>KeyPrompter</string>
  <key>CFBundleDisplayName</key><string>KeyPrompter</string>
  <key>CFBundleIdentifier</key><string>com.anibalrojas.KeyPrompter</string>
  <key>CFBundleExecutable</key><string>KeyPrompter</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>Markdown Rundown</string>
      <key>CFBundleTypeRole</key><string>Viewer</string>
      <key>LSHandlerRank</key><string>Alternate</string>
      <key>LSItemContentTypes</key>
      <array><string>net.daringfireball.markdown</string></array>
    </dict>
  </array>
</dict>
</plist>
PLIST
codesign --force --sign - "$app"
echo "$app"
