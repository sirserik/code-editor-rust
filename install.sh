#!/bin/bash
set -e

echo "=== Code Editor - macOS Installer ==="
echo ""

# Kill ALL running instances
echo "[0/4] Closing running instances..."
pkill -x "code-editor-rust" 2>/dev/null || true
sleep 0.5

echo "[1/4] Building release binary..."
cargo build --release 2>&1 | tail -1

BINARY="target/release/code-editor-rust"
if [ ! -f "$BINARY" ]; then
    echo "ERROR: Build failed!"
    exit 1
fi

APP_NAME="Code Editor.app"
APP_DIR="/Applications/$APP_NAME"
CONTENTS="$APP_DIR/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"

echo "[2/4] Creating app bundle at $APP_DIR..."

# Remove old version if exists
rm -rf "$APP_DIR"

# Create bundle structure
mkdir -p "$MACOS"
mkdir -p "$RESOURCES"

# Copy files
cp resources/Info.plist "$CONTENTS/"
cp "$BINARY" "$MACOS/code-editor-rust"
cp resources/AppIcon.icns "$RESOURCES/"

# Make executable
chmod +x "$MACOS/code-editor-rust"

echo "[3/4] Installing CLI command..."

# Also install CLI command
cp "$BINARY" /usr/local/bin/code-editor
chmod +x /usr/local/bin/code-editor

echo "[4/4] Registering with Launch Services..."

# Register the app with macOS
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$APP_DIR" 2>/dev/null || true

# Touch to update Spotlight index
touch "$APP_DIR"

echo ""
echo "=== Installation Complete ==="
echo ""
echo "  App:  /Applications/Code Editor.app"
echo "  CLI:  code-editor [path]"
echo ""
echo "  Launch from:"
echo "    - Spotlight (Cmd+Space → 'Code Editor')"
echo "    - Applications folder"
echo "    - Terminal: code-editor ."
echo "    - Right-click file → Open With → Code Editor"
echo ""

# Launch one instance
open "$APP_DIR"
