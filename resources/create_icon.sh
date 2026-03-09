#!/bin/bash
# Generate a simple code editor icon using sips and iconutil

ICONSET_DIR="/tmp/AppIcon.iconset"
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

# Create a simple icon using Python (available on macOS)
python3 << 'PYEOF'
import struct, zlib, os

def create_png(width, height, filename):
    """Create a simple code editor icon as PNG."""
    pixels = []

    for y in range(height):
        row = []
        for x in range(width):
            # Normalized coordinates
            nx = x / width
            ny = y / height

            # Background: dark blue-purple gradient (Tokyo Night inspired)
            bg_r = int(26 + ny * 10)
            bg_g = int(27 + ny * 8)
            bg_b = int(46 + ny * 15)

            r, g, b, a = bg_r, bg_g, bg_b, 255

            # Rounded rectangle border
            margin = width * 0.08
            corner_r = width * 0.18

            # Check if inside rounded rect
            in_rect = True
            dx = 0
            dy = 0

            if x < margin or x >= width - margin or y < margin or y >= height - margin:
                in_rect = False
            else:
                # Check corners
                cx = x
                cy = y
                if cx < margin + corner_r and cy < margin + corner_r:
                    dx = margin + corner_r - cx
                    dy = margin + corner_r - cy
                    if dx*dx + dy*dy > corner_r*corner_r:
                        in_rect = False
                elif cx >= width - margin - corner_r and cy < margin + corner_r:
                    dx = cx - (width - margin - corner_r)
                    dy = margin + corner_r - cy
                    if dx*dx + dy*dy > corner_r*corner_r:
                        in_rect = False
                elif cx < margin + corner_r and cy >= height - margin - corner_r:
                    dx = margin + corner_r - cx
                    dy = cy - (height - margin - corner_r)
                    if dx*dx + dy*dy > corner_r*corner_r:
                        in_rect = False
                elif cx >= width - margin - corner_r and cy >= height - margin - corner_r:
                    dx = cx - (width - margin - corner_r)
                    dy = cy - (height - margin - corner_r)
                    if dx*dx + dy*dy > corner_r*corner_r:
                        in_rect = False

            if not in_rect:
                r, g, b, a = 0, 0, 0, 0
            else:
                # Title bar area
                title_h = height * 0.12
                if ny < margin/height + title_h/height:
                    r, g, b = 36, 37, 56

                    # Traffic lights
                    light_y = margin + title_h * 0.5
                    light_r_val = width * 0.02
                    light_spacing = width * 0.05

                    for i, color in enumerate([(255, 95, 86), (255, 189, 46), (39, 201, 63)]):
                        lx = margin + width * 0.06 + i * light_spacing
                        dist = ((x - lx)**2 + (y - light_y)**2) ** 0.5
                        if dist < light_r_val:
                            r, g, b = color

                # Sidebar area
                elif nx < margin/width + 0.22:
                    r, g, b = 30, 31, 50

                    # File tree lines
                    line_h = height * 0.04
                    content_start = margin + title_h + height * 0.03

                    for i in range(8):
                        line_y = content_start + i * (line_h + height * 0.02)
                        indent = width * 0.03 * (1 if i in [2, 3, 4, 6, 7] else 0)
                        line_x_start = margin + width * 0.04 + indent
                        line_width = width * (0.08 + (i % 3) * 0.03)

                        if abs(y - line_y) < line_h * 0.4 and line_x_start < x < line_x_start + line_width:
                            if i in [0, 5]:  # Folders
                                r, g, b = 224, 175, 104
                            else:  # Files
                                r, g, b = 120, 124, 156

                # Editor area
                else:
                    r, g, b = 26, 27, 46

                    # Code lines
                    line_h = height * 0.035
                    content_start = margin + title_h + height * 0.03
                    editor_x_start = margin + width * 0.26

                    # Line numbers
                    num_x = editor_x_start + width * 0.01

                    for i in range(10):
                        line_y = content_start + i * (line_h + height * 0.015)

                        # Line number
                        if abs(y - line_y) < line_h * 0.35 and num_x < x < num_x + width * 0.03:
                            r, g, b = 65, 68, 95

                        # Code content with different colors
                        code_x = num_x + width * 0.05
                        indent = width * 0.03 * (1 if i in [1, 2, 3, 5, 6, 7, 8] else 0)
                        colors = [
                            (187, 154, 247),  # purple - keyword
                            (125, 207, 255),  # blue - function
                            (192, 202, 245),  # white - text
                            (158, 206, 106),  # green - string
                            (255, 158, 100),  # orange - number
                            (187, 154, 247),  # purple
                            (125, 207, 255),  # blue
                            (192, 202, 245),  # white
                            (158, 206, 106),  # green
                            (255, 158, 100),  # orange
                        ]

                        code_width = width * (0.06 + (i * 7 % 5) * 0.025)

                        if abs(y - line_y) < line_h * 0.35:
                            cx = code_x + indent
                            if cx < x < cx + code_width:
                                r, g, b = colors[i]
                            # Second segment
                            cx2 = cx + code_width + width * 0.01
                            seg2_w = width * (0.04 + (i * 3 % 4) * 0.02)
                            if cx2 < x < cx2 + seg2_w:
                                r, g, b = colors[(i + 2) % len(colors)]
                            # Third segment
                            cx3 = cx2 + seg2_w + width * 0.01
                            seg3_w = width * (0.03 + (i * 5 % 3) * 0.015)
                            if i < 8 and cx3 < x < cx3 + seg3_w:
                                r, g, b = colors[(i + 4) % len(colors)]

            row.extend([r, g, b, a])
        pixels.append(bytes([0] + row))  # Filter byte + row data

    # Create PNG
    raw = b''.join(pixels)

    def make_chunk(chunk_type, data):
        c = chunk_type + data
        crc = struct.pack('>I', zlib.crc32(c) & 0xffffffff)
        return struct.pack('>I', len(data)) + c + crc

    png = b'\x89PNG\r\n\x1a\n'
    png += make_chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
    png += make_chunk(b'IDAT', zlib.compress(raw, 9))
    png += make_chunk(b'IEND', b'')

    with open(filename, 'wb') as f:
        f.write(png)

sizes = [16, 32, 64, 128, 256, 512, 1024]
iconset = "/tmp/AppIcon.iconset"

for s in sizes:
    create_png(s, s, f"{iconset}/icon_{s}x{s}.png")
    if s <= 512:
        create_png(s*2, s*2, f"{iconset}/icon_{s}x{s}@2x.png")
    print(f"Created {s}x{s}")

print("All icons created!")
PYEOF

# Convert iconset to icns
iconutil -c icns "$ICONSET_DIR" -o "$(dirname "$0")/AppIcon.icns"
echo "AppIcon.icns created!"
rm -rf "$ICONSET_DIR"
