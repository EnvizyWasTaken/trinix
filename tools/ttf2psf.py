#!/usr/bin/env python3
"""
Convert a TTF/OTF font to PSF2 bitmap format for VGA text mode (8×16).

Renders each glyph at 4× resolution using antialiasing, then downsamples
to 8×16 and thresholds to 1-bit. This gives much cleaner results than
rendering directly at the target size.

Usage:
    python3 tools/ttf2psf.py <font.ttf> assets/fonts/<name>.psf [--size N] [--threshold N]
    python3 tools/ttf2psf.py --batch    # convert all fonts listed in BATCH
"""

import sys, struct, os, argparse
from PIL import Image, ImageFont, ImageDraw

GLYPH_W   = 8
GLYPH_H   = 16
SCALE     = 4          # render at this multiple, then downsample
NUM_GLYPHS = 256

BATCH = [
    ("/usr/share/fonts/TTF/JetBrainsMonoNLNerdFont-Regular.ttf", "assets/fonts/jetbrains-mono.psf"),
    ("/usr/share/fonts/TTF/JetBrainsMonoNLNerdFont-Bold.ttf",    "assets/fonts/jetbrains-mono-bold.psf"),
    ("/usr/share/fonts/TTF/Hack-Regular.ttf",                    "assets/fonts/hack.psf"),
    ("/usr/share/fonts/Adwaita/AdwaitaMono-Regular.ttf",         "assets/fonts/adwaita-mono.psf"),
    ("/usr/share/fonts/noto/NotoSansMono-Regular.ttf",           "assets/fonts/noto-mono.psf"),
]

# ─── size detection ───────────────────────────────────────────────────────────

def find_size(font_path: str) -> int:
    """Find the largest integer size where the cap-width fits in GLYPH_W pixels."""
    best = 8
    for size in range(5, 30):
        f = ImageFont.truetype(font_path, size)
        # getlength returns the advance width; for monospace all chars are equal
        w = f.getlength("M")
        if w <= GLYPH_W:
            best = size
        else:
            break
    return best

# ─── glyph rendering ─────────────────────────────────────────────────────────

def render_glyph(font: ImageFont.FreeTypeFont, ch: str, threshold: int) -> bytes:
    sw, sh = GLYPH_W * SCALE, GLYPH_H * SCALE

    canvas = Image.new("L", (sw, sh), 0)
    draw   = ImageDraw.Draw(canvas)

    try:
        bbox = font.getbbox(ch)
        if bbox is None or (bbox[2] - bbox[0]) == 0:
            return bytes(GLYPH_H)  # empty glyph

        gw = bbox[2] - bbox[0]
        gh = bbox[3] - bbox[1]

        # Centre horizontally; align baseline to ~75 % of cell height
        ascent, _descent = font.getmetrics()
        x = (sw - gw) // 2 - bbox[0]
        baseline = int(sh * 0.78)
        y = baseline - ascent

        draw.text((x, y), ch, font=font, fill=255)
    except Exception:
        return bytes(GLYPH_H)

    # Downsample to target cell size
    small  = canvas.resize((GLYPH_W, GLYPH_H), Image.LANCZOS)
    pixels = small.load()

    rows = []
    for row in range(GLYPH_H):
        byte = 0
        for col in range(GLYPH_W):
            if pixels[col, row] > threshold:
                byte |= 0x80 >> col
        rows.append(byte)
    return bytes(rows)

# ─── PSF2 writer ─────────────────────────────────────────────────────────────

PSF2_MAGIC = 0x864AB572

def write_psf2(path: str, glyphs: list[bytes]) -> None:
    header = struct.pack("<IIIIIIII",
        PSF2_MAGIC,
        0,           # version
        32,          # headersize
        0,           # flags (no unicode table)
        len(glyphs), # num glyphs
        GLYPH_H,     # charsize  (bytes per glyph = H rows × 1 byte/row for 8-wide)
        GLYPH_H,     # height
        GLYPH_W,     # width
    )
    with open(path, "wb") as f:
        f.write(header)
        for g in glyphs:
            f.write(g)

# ─── main ─────────────────────────────────────────────────────────────────────

def convert(ttf: str, out: str, size: int | None, threshold: int) -> None:
    if not os.path.exists(ttf):
        print(f"  skip (not found): {ttf}")
        return

    if size is None:
        size = find_size(ttf)

    font = ImageFont.truetype(ttf, size)
    print(f"  {os.path.basename(ttf)}  size={size}  threshold={threshold}", flush=True)

    glyphs = [render_glyph(font, chr(i), threshold) for i in range(NUM_GLYPHS)]
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    write_psf2(out, glyphs)
    print(f"  → {out}  ({len(glyphs)} glyphs, {GLYPH_W}×{GLYPH_H})")

def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("ttf",       nargs="?")
    ap.add_argument("out",       nargs="?")
    ap.add_argument("--size",      type=int, default=None)
    ap.add_argument("--threshold", type=int, default=88)
    ap.add_argument("--batch",     action="store_true")
    args = ap.parse_args()

    if args.batch:
        print("Batch converting fonts...")
        for ttf, out in BATCH:
            convert(ttf, out, args.size, args.threshold)
        print("Done.")
    elif args.ttf and args.out:
        convert(args.ttf, args.out, args.size, args.threshold)
    else:
        ap.print_help()
        sys.exit(1)

if __name__ == "__main__":
    main()
