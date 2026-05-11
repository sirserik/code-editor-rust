#!/bin/bash
# Regenerate AppIcon.icns from the modern Python generator.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"

echo "[1/3] Rendering iconset via PIL…"
python3 "$HERE/create_icon.py"

echo "[2/3] Packing iconset into AppIcon.icns…"
iconutil -c icns /tmp/AppIcon.iconset -o "$HERE/AppIcon.icns"

echo "[3/3] Cleaning up…"
rm -rf /tmp/AppIcon.iconset
echo "Done: $HERE/AppIcon.icns"
