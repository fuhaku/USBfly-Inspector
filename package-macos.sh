#!/bin/bash
set -e

# Configuration
APP_NAME="USBfly"
APP_IDENTIFIER="com.usbfly.app"
APP_VERSION="0.1.0"
ICON_PATH="assets/icon.svg"
OUTPUT_DIR="target/release"

# Check if the binary exists
if [ ! -f "$OUTPUT_DIR/usbfly" ]; then
    echo "Error: Release binary not found at $OUTPUT_DIR/usbfly"
    echo "Please build the project with 'cargo build --release' first."
    exit 1
fi

# Create the app bundle structure
BUNDLE_DIR="$OUTPUT_DIR/$APP_NAME.app"
CONTENTS_DIR="$BUNDLE_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# Copy the binary
cp "$OUTPUT_DIR/usbfly" "$MACOS_DIR/$APP_NAME"
chmod +x "$MACOS_DIR/$APP_NAME"

# Create Info.plist
cat > "$CONTENTS_DIR/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>$APP_NAME</string>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>$APP_IDENTIFIER</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$APP_VERSION</string>
    <key>CFBundleVersion</key>
    <string>$APP_VERSION</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2023. All rights reserved.</string>
</dict>
</plist>
EOF

# Check if the icon file exists and convert it to .icns
if [ -f "$ICON_PATH" ]; then
    echo "Converting SVG icon to icns..."
    # For this step, you'd typically need to use ImageMagick or a similar tool
    # to convert the SVG to PNG files at various sizes, then use iconutil to create an .icns
    # For simplicity, we'll just mention that this step would be done here
    echo "Note: In a real implementation, the SVG would be converted to an .icns file here."
    echo "For now, we'll create an empty icon file as a placeholder."
    touch "$RESOURCES_DIR/AppIcon.icns"
else
    echo "Warning: Icon file not found at $ICON_PATH"
    echo "Creating a placeholder icon file."
    touch "$RESOURCES_DIR/AppIcon.icns"
fi

# Create a DMG
DMG_FILE="$OUTPUT_DIR/$APP_NAME-$APP_VERSION.dmg"
DMG_TMP_FILE="$OUTPUT_DIR/$APP_NAME-$APP_VERSION-tmp.dmg"

echo "Creating DMG..."
# For simplicity, we're using a basic approach here.
# In a real implementation, you might want to use create-dmg or similar tools
# to create a more polished DMG with custom background, etc.

# Create a temporary DMG
hdiutil create -volname "$APP_NAME" -srcfolder "$BUNDLE_DIR" -ov -format UDRW "$DMG_TMP_FILE"

# Convert the temporary DMG to the final compressed DMG
hdiutil convert "$DMG_TMP_FILE" -format UDZO -o "$DMG_FILE"
rm -f "$DMG_TMP_FILE"

echo "Done! Created $APP_NAME.app and $APP_NAME-$APP_VERSION.dmg in the $OUTPUT_DIR directory."
