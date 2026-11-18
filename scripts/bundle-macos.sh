#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/pomotroid}"
bundle="$(dirname "$binary")/Pomotroid.app"
version=$(cargo metadata --locked --no-deps --format-version 1 | python3 -c \
  'import json, sys; print(json.load(sys.stdin)["packages"][0]["version"])')

install -d "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
install -m 755 "$binary" "$bundle/Contents/MacOS/pomotroid"
install -m 644 build/icons/icon.icns "$bundle/Contents/Resources/Pomotroid.icns"

cat > "$bundle/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>com.splode.pomotroid</string>
  <key>CFBundleName</key>
  <string>Pomotroid</string>
  <key>CFBundleExecutable</key>
  <string>pomotroid</string>
  <key>CFBundleIconFile</key>
  <string>Pomotroid.icns</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$version</string>
  <key>CFBundleVersion</key>
  <string>$version</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.productivity</string>
</dict>
</plist>
EOF

plutil -lint "$bundle/Contents/Info.plist"
codesign --force --sign - "$bundle"
codesign --verify --strict "$bundle"
