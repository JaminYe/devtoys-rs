#!/usr/bin/env python3
"""Generate DevToys desktop icon assets (.ico and .png)"""
import os
import struct
from PIL import Image, ImageDraw

def generate_icons():
    canvas_size = 1024
    scale = canvas_size / 24.0
    img = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Background rounded rectangle: DevToys blue #2563eb
    pad = 0.5 * scale
    draw.rounded_rectangle(
        [pad, pad, canvas_size - pad, canvas_size - pad],
        radius=int(5.2 * scale),
        fill=(37, 99, 235, 255),
    )

    # Square terminal frame
    term_stroke = int(1.8 * scale)
    term_pad = 4.0 * scale
    draw.rounded_rectangle(
        [term_pad, term_pad, canvas_size - term_pad, canvas_size - term_pad],
        radius=int(2.2 * scale),
        outline=(255, 255, 255, 255),
        width=term_stroke,
    )

    # Chevron '>'
    p1 = (7.5 * scale, 8.5 * scale)
    p2 = (11.5 * scale, 12.0 * scale)
    p3 = (7.5 * scale, 15.5 * scale)
    draw.line([p1, p2, p3], fill=(255, 255, 255, 255), width=term_stroke, joint="round")

    # Cursor '_'
    c1 = (13.5 * scale, 15.5 * scale)
    c2 = (17.5 * scale, 15.5 * scale)
    draw.line([c1, c2], fill=(255, 255, 255, 255), width=term_stroke)

    img256 = img.resize((256, 256), Image.Resampling.LANCZOS)
    sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]

    assets_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "crates", "host", "assets")
    os.makedirs(assets_dir, exist_ok=True)
    ico_path = os.path.join(assets_dir, "icon.ico")
    png_path = os.path.join(assets_dir, "icon.png")

    img256.save(ico_path, format="ICO", sizes=sizes)
    img256.save(png_path, format="PNG")
    print(f"Generated {ico_path} and {png_path}")

if __name__ == "__main__":
    generate_icons()
