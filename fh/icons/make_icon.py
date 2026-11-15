#!/usr/bin/env python3
"""Draws Fabric n Horizon's icon (fh/icons/fh.svg) and writes fh.ico.

No image library is involved: the SVG's shapes (the black tile and the F and H of its mark) are
rasterised here with 4x4 supersampling, and the frames are written as the .ico container Windows
reads: 32-bit BMP frames up to 64 pixels, PNG frames above (what Explorer and the task bar take for
the large sizes). The tile and the lettering follow JLac's own mark: white on black, no colour.

    python3 fh/icons/make_icon.py            # writes fh/icons/fh.ico
    python3 fh/icons/make_icon.py --png 256  # also writes fh-256.png, to look at it
"""

import argparse
import pathlib
import struct
import zlib

# The icon in the SVG's 64-unit square: the tile's rounding, and the F and H drawn from bars.
TILE = (0.0, 0.0, 64.0, 64.0)
RADIUS = 14.0
# F: stem, top arm, middle arm. H: left stem, right stem, crossbar.
BARS = [
    (12.0, 18.0, 18.0, 46.0),
    (12.0, 18.0, 32.0, 24.0),
    (12.0, 29.0, 28.0, 35.0),
    (38.0, 18.0, 44.0, 46.0),
    (50.0, 18.0, 56.0, 46.0),
    (38.0, 29.0, 56.0, 35.0),
]

NIGHT = (0x0B, 0x0B, 0x0B)
MARK = (0xFF, 0xFF, 0xFF)

SIZES = [16, 20, 24, 32, 40, 48, 64, 128, 256]
SUPERSAMPLE = 4


def inside_tile(x, y):
    left, top, right, bottom = TILE
    if not (left <= x <= right and top <= y <= bottom):
        return False
    # The rounded corners: a circle of RADIUS at each corner of the tile.
    cx = min(max(x, left + RADIUS), right - RADIUS)
    cy = min(max(y, top + RADIUS), bottom - RADIUS)
    return (x - cx) ** 2 + (y - cy) ** 2 <= RADIUS * RADIUS


def inside_mark(x, y):
    """Whether (x, y) is in the lettering: inside one of its bars."""
    return any(left <= x <= right and top <= y <= bottom for left, top, right, bottom in BARS)


def pixel(px, py, size):
    """One pixel's RGBA, from the 64-unit shapes scaled to `size`."""
    unit = 64.0 / size
    red = green = blue = alpha = 0.0
    for sy in range(SUPERSAMPLE):
        for sx in range(SUPERSAMPLE):
            x = (px + (sx + 0.5) / SUPERSAMPLE) * unit
            y = (py + (sy + 0.5) / SUPERSAMPLE) * unit
            if not inside_tile(x, y):
                continue
            colour = MARK if inside_mark(x, y) else NIGHT
            red += colour[0]
            green += colour[1]
            blue += colour[2]
            alpha += 1.0
    samples = SUPERSAMPLE * SUPERSAMPLE
    if alpha == 0.0:
        return (0, 0, 0, 0)
    return (
        round(red / alpha),
        round(green / alpha),
        round(blue / alpha),
        round(255 * alpha / samples),
    )


def frame(size):
    """The RGBA pixels of `size` square, top row first."""
    return [[pixel(x, y, size) for x in range(size)] for y in range(size)]


def bmp_frame(pixels):
    """A 32-bit BMP frame: the size and the doubled height, rows bottom-up, then the AND mask."""
    height = len(pixels)
    width = len(pixels[0])
    header = struct.pack(
        "<IiiHHIIiiII",
        40,
        width,
        height * 2,
        1,
        32,
        0,
        width * height * 4,
        0,
        0,
        0,
        0,
    )
    body = bytearray()
    for row in reversed(pixels):
        for (r, g, b, a) in row:
            body += bytes((b, g, r, a))
    # The AND mask: every pixel the alpha already makes transparent, one bit each, padded to 32 bits.
    stride = ((width + 31) // 32) * 4
    mask = bytearray()
    for row in reversed(pixels):
        bits = bytearray(stride)
        for x, (_, _, _, a) in enumerate(row):
            if a == 0:
                bits[x // 8] |= 1 << (7 - x % 8)
        mask += bits
    return header + bytes(body) + bytes(mask)


def png_frame(pixels):
    """The same frame as a PNG, which is what the large sizes hold."""
    height = len(pixels)
    width = len(pixels[0])

    def chunk(kind, data):
        return (
            struct.pack(">I", len(data))
            + kind
            + data
            + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
        )

    raw = bytearray()
    for row in pixels:
        raw.append(0)
        for pixel in row:
            raw += bytes(pixel)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def ico(frames):
    """The .ico container: a header, one entry per frame, then the frames' bytes."""
    header = struct.pack("<HHH", 0, 1, len(frames))
    offset = len(header) + 16 * len(frames)
    entries = bytearray()
    body = bytearray()
    for size, data in frames:
        entries += struct.pack(
            "<BBBBHHII",
            0 if size >= 256 else size,
            0 if size >= 256 else size,
            0,
            0,
            1,
            32,
            len(data),
            offset,
        )
        body += data
        offset += len(data)
    return header + bytes(entries) + bytes(body)


def write_png(path, pixels):
    path.write_bytes(png_frame(pixels))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--png",
        type=int,
        action="append",
        default=[],
        help="also write fh-<size>.png beside the .ico, to look at",
    )
    default = pathlib.Path(__file__).parent / "fh.ico"
    parser.add_argument("--out", default=str(default))
    args = parser.parse_args()

    frames = []
    for size in SIZES:
        pixels = frame(size)
        data = bmp_frame(pixels) if size <= 64 else png_frame(pixels)
        frames.append((size, data))
    out = pathlib.Path(args.out)
    out.write_bytes(ico(frames))
    print(f"{out}: {len(SIZES)} frames, {out.stat().st_size} bytes")
    for size in args.png:
        path = out.with_name(f"fh-{size}.png")
        rows = frame(size)
        write_png(path, rows)
        print(f"{path}: {path.stat().st_size} bytes")


if __name__ == "__main__":
    main()
