#!/usr/bin/env python3
"""
Generate a modern macOS-style app icon for Code Editor.

Design: squircle background with indigo→purple gradient + centered "</>" in white,
rendered via the bundled JetBrainsMono font. All required iconset sizes are emitted
and packaged into an .icns via `iconutil` (called by the surrounding shell script).
"""
import os
import sys
from PIL import Image, ImageDraw, ImageFont, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(HERE)
FONT_PATH = os.path.join(PROJECT_ROOT, "assets", "JetBrainsMono-Regular.ttf")
ICONSET_DIR = "/tmp/AppIcon.iconset"

# Modern macOS-style palette (indigo → purple, similar to JetBrains Toolbox / Linear)
GRAD_TOP_LEFT = (79, 70, 229)   # #4F46E5 indigo-600
GRAD_BOT_RIGHT = (124, 58, 237) # #7C3AED purple-600
SHADOW_COLOR = (0, 0, 0, 80)
TEXT_COLOR = (255, 255, 255, 255)
SYMBOL = "</>"

# macOS Big Sur HIG: corner radius is ~22.37% of icon size for the "squircle" look.
CORNER_RATIO = 0.2237
# Leave a small transparent margin so the icon doesn't touch the canvas edge —
# matches how macOS apps render against the Dock background.
MARGIN_RATIO = 0.06


def squircle_mask(size: int) -> Image.Image:
    """Black/white mask: white inside the rounded square, black outside."""
    margin = int(size * MARGIN_RATIO)
    inner = size - 2 * margin
    radius = int(inner * CORNER_RATIO)
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle(
        [(margin, margin), (size - margin, size - margin)],
        radius=radius,
        fill=255,
    )
    return mask


def diagonal_gradient(size: int) -> Image.Image:
    """Indigo→purple gradient running top-left to bottom-right."""
    grad = Image.new("RGB", (size, size))
    px = grad.load()
    tr, tg, tb = GRAD_TOP_LEFT
    br, bg, bb = GRAD_BOT_RIGHT
    inv = 1.0 / (2.0 * (size - 1))
    for y in range(size):
        for x in range(size):
            # Diagonal interpolation: (x + y) / 2*(size-1) goes from 0 → 1
            t = (x + y) * inv
            r = int(tr + (br - tr) * t)
            g = int(tg + (bg - tg) * t)
            b = int(tb + (bb - tb) * t)
            px[x, y] = (r, g, b)
    return grad


def draw_symbol(canvas: Image.Image) -> None:
    """Draw the </> symbol centered, with a subtle drop shadow."""
    size = canvas.width
    # Use ~50% of icon width for the glyph; rendered via FreeType so it scales cleanly.
    # JetBrainsMono is monospace so "</>" renders as 3 cells of equal width.
    font_size = int(size * 0.45)
    try:
        font = ImageFont.truetype(FONT_PATH, font_size)
    except OSError as e:
        print(f"Failed to load font {FONT_PATH}: {e}", file=sys.stderr)
        sys.exit(1)

    # Drop shadow — render glyph onto a transparent layer, blur, paste underneath.
    shadow_layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow_layer)
    sd.text((size // 2, size // 2 + int(size * 0.01)), SYMBOL, font=font,
            fill=SHADOW_COLOR, anchor="mm")
    shadow_layer = shadow_layer.filter(ImageFilter.GaussianBlur(radius=size * 0.012))
    canvas.alpha_composite(shadow_layer)

    # Main glyph
    draw = ImageDraw.Draw(canvas)
    draw.text((size // 2, size // 2), SYMBOL, font=font, fill=TEXT_COLOR, anchor="mm")


def render_icon(size: int) -> Image.Image:
    """Render the full icon at the given size."""
    grad = diagonal_gradient(size).convert("RGBA")
    mask = squircle_mask(size)
    # Combine: gradient where mask is white, transparent elsewhere.
    icon = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    icon.paste(grad, (0, 0), mask=mask)

    # Drop shadow under the squircle itself — gives the icon depth in the Finder grid.
    shadow_under = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow_under)
    margin = int(size * MARGIN_RATIO)
    radius = int((size - 2 * margin) * CORNER_RATIO)
    sd.rounded_rectangle(
        [(margin, margin + int(size * 0.012)), (size - margin, size - margin + int(size * 0.012))],
        radius=radius,
        fill=(0, 0, 0, 60),
    )
    shadow_under = shadow_under.filter(ImageFilter.GaussianBlur(radius=size * 0.02))

    composite = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    composite.alpha_composite(shadow_under)
    composite.alpha_composite(icon)

    draw_symbol(composite)
    return composite


def main() -> int:
    if os.path.exists(ICONSET_DIR):
        import shutil
        shutil.rmtree(ICONSET_DIR)
    os.makedirs(ICONSET_DIR, exist_ok=True)

    # Apple's required sizes: each generated at @1x and @2x where applicable.
    spec = [
        (16, "icon_16x16.png"),
        (32, "icon_16x16@2x.png"),
        (32, "icon_32x32.png"),
        (64, "icon_32x32@2x.png"),
        (128, "icon_128x128.png"),
        (256, "icon_128x128@2x.png"),
        (256, "icon_256x256.png"),
        (512, "icon_256x256@2x.png"),
        (512, "icon_512x512.png"),
        (1024, "icon_512x512@2x.png"),
    ]
    for size, name in spec:
        img = render_icon(size)
        img.save(os.path.join(ICONSET_DIR, name), "PNG")
        print(f"  rendered {name} ({size}x{size})")

    # Also drop a 256×256 raw RGBA blob next to the font: eframe/winit needs raw pixels
    # at runtime to set the Dock icon when the binary is launched outside a .app bundle
    # (e.g. via `cargo run` or directly from the terminal).
    assets_dir = os.path.join(PROJECT_ROOT, "assets")
    os.makedirs(assets_dir, exist_ok=True)
    rgba_path = os.path.join(assets_dir, "icon-256.rgba")
    rgba_img = render_icon(256).convert("RGBA")
    with open(rgba_path, "wb") as f:
        f.write(rgba_img.tobytes())
    print(f"  wrote runtime icon → {rgba_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
