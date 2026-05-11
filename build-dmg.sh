#!/bin/bash
# Build a distributable macOS .dmg with a drag-to-Applications install layout.
# Usage: ./build-dmg.sh
# Output: dist/CodeEditor-<version>.dmg
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$HERE"

# Pull version from Cargo.toml so we don't keep two sources of truth.
VERSION="$(awk -F '"' '/^version = / {print $2; exit}' Cargo.toml)"
APP_NAME="Code Editor"
DMG_BASE="CodeEditor-${VERSION}"
DIST_DIR="dist"
STAGE="$(mktemp -d -t codeeditor-dmg-XXXX)"
APP_DIR="$STAGE/$APP_NAME.app"
DMG_VOLNAME="$APP_NAME ${VERSION}"

cleanup() { rm -rf "$STAGE" /tmp/codeeditor-dmg-rw.dmg 2>/dev/null || true; }
trap cleanup EXIT

echo "[1/6] Building release binary (this is the slow step)…"
cargo build --release --quiet

echo "[2/6] Assembling .app bundle in staging…"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"
cp resources/Info.plist "$APP_DIR/Contents/"
cp target/release/code-editor-rust "$APP_DIR/Contents/MacOS/"
chmod +x "$APP_DIR/Contents/MacOS/code-editor-rust"
cp resources/AppIcon.icns "$APP_DIR/Contents/Resources/"

# Symlink so users can drag the app right next to /Applications without searching.
ln -s /Applications "$STAGE/Applications"

# Optional README that ships inside the DMG window.
cat > "$STAGE/README.txt" <<EOF
Code Editor ${VERSION}

To install, drag "$APP_NAME.app" onto the Applications shortcut.

Then optionally install the CLI launcher (lets you run \`code-editor .\` from a
terminal). Open Terminal and run:

    sudo cp /Applications/${APP_NAME}.app/Contents/MacOS/code-editor-rust /usr/local/bin/code-editor

For source, builds and updates: https://github.com/sirserik/code-editor-rust
EOF

echo "[3/6] Removing macOS quarantine attributes from staged bundle…"
xattr -cr "$APP_DIR" 2>/dev/null || true

echo "[4/6] Creating writable intermediate DMG…"
RW_DMG="/tmp/codeeditor-dmg-rw.dmg"
hdiutil create -srcfolder "$STAGE" \
    -volname "$DMG_VOLNAME" \
    -fs HFS+ -fsargs "-c c=64,a=16,e=16" \
    -format UDRW -size 200m \
    "$RW_DMG" >/dev/null

echo "[5/6] Arranging Finder window via AppleScript (skipped if it hangs)…"
MOUNT_POINT="$(hdiutil attach -readwrite -noverify -noautoopen "$RW_DMG" | awk '/Volumes/ {print $NF; exit}')"

# AppleScript-driven window layout is purely cosmetic — it gives the DMG that "drag-to-
# the-pretty-Applications-icon" look. It needs an active Aqua session (Finder running)
# which often isn't available in CI or headless shells. So: best-effort + 10s timeout,
# and if it fails we still produce a perfectly usable DMG with a basic list view.
LAYOUT_SCRIPT="tell application \"Finder\"
    tell disk \"$DMG_VOLNAME\"
        open
        set current view of container window to icon view
        set toolbar visible of container window to false
        set statusbar visible of container window to false
        set the bounds of container window to {200, 200, 800, 540}
        set theViewOptions to the icon view options of container window
        set arrangement of theViewOptions to not arranged
        set icon size of theViewOptions to 128
        set position of item \"$APP_NAME.app\" of container window to {160, 170}
        set position of item \"Applications\" of container window to {440, 170}
        set position of item \"README.txt\" of container window to {580, 320}
        update without registering applications
        close
    end tell
end tell"

# Use `perl alarm` for portable timeout (BSD `timeout` isn't on macOS by default).
echo "$LAYOUT_SCRIPT" | (perl -e 'alarm 10; exec @ARGV' osascript - >/dev/null 2>&1 || true)

sync
# Force-detach in case Finder is still holding the volume open.
hdiutil detach "$MOUNT_POINT" -force -quiet 2>/dev/null || \
    hdiutil detach "/Volumes/$DMG_VOLNAME" -force -quiet 2>/dev/null || true

echo "[6/6] Compressing into final read-only DMG…"
mkdir -p "$DIST_DIR"
FINAL_DMG="$DIST_DIR/${DMG_BASE}.dmg"
rm -f "$FINAL_DMG"
hdiutil convert "$RW_DMG" -format UDZO -imagekey zlib-level=9 \
    -o "$FINAL_DMG" >/dev/null

# Drop a hint about codesigning. macOS Gatekeeper will warn users on first open
# because the bundle isn't signed; running `xattr -dr com.apple.quarantine`
# clears that for the user, but the cleanest fix is a Developer ID signature.
SIZE_KB=$(du -k "$FINAL_DMG" | awk '{print $1}')
echo
echo "✅ Built: $FINAL_DMG ($((SIZE_KB / 1024)) MB)"
echo
echo "Note: this DMG is NOT codesigned. First-time users will see a Gatekeeper warning."
echo "      They can bypass it via System Settings → Privacy → 'Open Anyway',"
echo "      or you can sign with: codesign --deep --force --options runtime \\"
echo "         --sign 'Developer ID Application: Your Name (TEAMID)' '$APP_DIR'"
