#!/bin/bash
# USBfly macOS packaging script
# This script will build a macOS app bundle for USBfly

set -e  # Exit on any error

# Configuration
APP_NAME="USBfly"
APP_VERSION="1.0.0"
BUNDLE_ID="com.greatscottgadgets.usbfly"
MACOS_DEPLOYMENT_TARGET="10.15"  # Catalina or later
ICON_PATH="./generated-icon.png"
COPYRIGHT="© $(date +%Y) Great Scott Gadgets"

# Create output directories
echo "Creating output directories..."
mkdir -p "./target/release/bundle/macos/$APP_NAME.app/Contents/"{MacOS,Resources,Frameworks}

# Build release binary
echo "Building release binary..."
cargo build --release

# Create Info.plist
echo "Creating Info.plist..."
cat > "./target/release/bundle/macos/$APP_NAME.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${APP_VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${APP_VERSION}</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>LSMinimumSystemVersion</key>
    <string>${MACOS_DEPLOYMENT_TARGET}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>${COPYRIGHT}</string>
</dict>
</plist>
EOF

# Copy the binary
echo "Copying binary..."
cp "./target/release/usbfly" "./target/release/bundle/macos/$APP_NAME.app/Contents/MacOS/$APP_NAME"

# Generate and convert icon if needed 
if [ -f "$ICON_PATH" ]; then
    echo "Converting icon..."
    mkdir -p "./target/release/bundle/macos/$APP_NAME.app/Contents/Resources/AppIcon.iconset"
    
    # Check if we have sips/iconutil (macOS tools) or use alternative conversion
    if command -v sips >/dev/null && command -v iconutil >/dev/null; then
        # macOS icon generation
        for size in 16 32 64 128 256 512; do
            sips -z $size $size "$ICON_PATH" --out "./target/release/bundle/macos/$APP_NAME.app/Contents/Resources/AppIcon.iconset/icon_${size}x${size}.png"
            sips -z $((size*2)) $((size*2)) "$ICON_PATH" --out "./target/release/bundle/macos/$APP_NAME.app/Contents/Resources/AppIcon.iconset/icon_${size}x${size}@2x.png"
        done
        iconutil -c icns "./target/release/bundle/macos/$APP_NAME.app/Contents/Resources/AppIcon.iconset" -o "./target/release/bundle/macos/$APP_NAME.app/Contents/Resources/AppIcon.icns"
        rm -rf "./target/release/bundle/macos/$APP_NAME.app/Contents/Resources/AppIcon.iconset"
    else
        # Fallback if not on macOS
        echo "Warning: sips/iconutil not available. Using icon file directly."
        cp "$ICON_PATH" "./target/release/bundle/macos/$APP_NAME.app/Contents/Resources/AppIcon.icns"
    fi
else
    echo "Warning: Icon file not found at $ICON_PATH. App will have no custom icon."
fi

# Handle dylib dependencies if needed (this could be expanded)
echo "Checking for dynamic library dependencies..."
# Use otool on macOS to check dylib dependencies
if command -v otool >/dev/null; then
    DEPS=$(otool -L "./target/release/usbfly" | grep -v '/System/\|/usr/lib/\|@executable_path/' | awk '{print $1}')
    if [ -n "$DEPS" ]; then
        echo "Found dependencies to bundle:"
        echo "$DEPS"
        for lib in $DEPS; do
            echo "Copying $lib..."
            cp "$lib" "./target/release/bundle/macos/$APP_NAME.app/Contents/Frameworks/"
            LIBNAME=$(basename "$lib")
            install_name_tool -change "$lib" "@executable_path/../Frameworks/$LIBNAME" "./target/release/bundle/macos/$APP_NAME.app/Contents/MacOS/$APP_NAME"
        done
    else
        echo "No external dependencies found."
    fi
else
    echo "otool not available. Skipping dylib dependency check."
fi

# Create a DMG for distribution (if hdiutil is available)
if command -v hdiutil >/dev/null; then
    echo "Creating DMG..."
    DMG_PATH="./target/release/$APP_NAME-$APP_VERSION.dmg"
    hdiutil create -volname "$APP_NAME" -srcfolder "./target/release/bundle/macos/$APP_NAME.app" -ov -format UDZO "$DMG_PATH"
    echo "DMG created: $DMG_PATH"
else
    echo "hdiutil not found. Skipping DMG creation."
    echo "App bundle is located at: ./target/release/bundle/macos/$APP_NAME.app"
fi

echo "Packaging complete!"