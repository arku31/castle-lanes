#!/usr/bin/env python3
"""Slice a generated 5-frame unit atlas into game-ready sprite frames.

Pipeline (plan.md Phase 2 item 5):
- split the single-row atlas into 5 equal cells
- convert the flat #ff00ff chroma-key background to alpha
- trim empty bounds per frame, then normalize onto a shared square canvas so
  the character stays registered across frames (no jitter when animated)
- write <out_prefix>_frame0..4.png plus a contact sheet

Usage:
  .venv/bin/python tools/slice_frame_atlas.py \
      assets/art/units/atlases_raw/vanguard_guard_frames.png \
      assets/art/units/frames/vanguard_guard
"""

import sys
from pathlib import Path

from PIL import Image

FRAME_COUNT = 5
CANVAS = 192
KEY = (255, 0, 255)


def chroma_to_alpha(image: Image.Image) -> Image.Image:
    image = image.convert("RGBA")
    pixels = image.load()
    width, height = image.size
    for y in range(height):
        for x in range(width):
            r, g, b, a = pixels[x, y]
            # key distance with a small tolerance; despill magenta fringe
            if abs(r - KEY[0]) < 90 and g < 140 and b > 140:
                pixels[x, y] = (r, g, b, 0)
            elif r > 150 and b > 150 and g < 90:
                pixels[x, y] = (r, g, b, 0)
    return image


def trim(image: Image.Image) -> Image.Image:
    bounds = image.getbbox()
    return image.crop(bounds) if bounds else image


def slice_atlas(atlas_path: Path, out_prefix: Path) -> None:
    atlas = Image.open(atlas_path)
    width, height = atlas.size
    cell_width = width // FRAME_COUNT
    out_prefix.parent.mkdir(parents=True, exist_ok=True)

    frames = []
    for index in range(FRAME_COUNT):
        cell = atlas.crop((index * cell_width, 0, (index + 1) * cell_width, height))
        cell = trim(chroma_to_alpha(cell))
        frames.append(cell)

    # Shared normalization: scale each frame to fit the canvas, then center on
    # the ground line so multi-frame playback does not jitter.
    normalized = []
    for cell in frames:
        scale = min(CANVAS / cell.width, CANVAS / cell.height, 1.0 if cell.width >= CANVAS else 1.0)
        scale = min(CANVAS / cell.width, CANVAS / cell.height)
        if 0 < scale < 1.0:
            cell = cell.resize(
                (max(1, int(cell.width * scale)), max(1, int(cell.height * scale))),
                Image.LANCZOS,
            )
        elif cell.width > CANVAS or cell.height > CANVAS:
            cell.thumbnail((CANVAS, CANVAS), Image.LANCZOS)
        normalized.append(cell)

    for index, cell in enumerate(normalized):
        canvas = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
        canvas.paste(cell, ((CANVAS - cell.width) // 2, CANVAS - cell.height), cell)
        out = out_prefix.parent / f"{out_prefix.name}_frame{index}.png"
        canvas.save(out)
        print(f"wrote {out}")

    sheet = Image.new("RGBA", (CANVAS * FRAME_COUNT, CANVAS), (24, 24, 28, 255))
    for index, cell in enumerate(normalized):
        sheet.paste(cell, (index * CANVAS, CANVAS - cell.height), cell)
    sheet_path = out_prefix.parent / f"{out_prefix.name}_contact.png"
    sheet.save(sheet_path)
    print(f"wrote {sheet_path}")


def main() -> None:
    if len(sys.argv) != 3:
        print(__doc__)
        sys.exit(1)
    slice_atlas(Path(sys.argv[1]), Path(sys.argv[2]))


if __name__ == "__main__":
    main()
