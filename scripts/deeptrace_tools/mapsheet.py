"""mapsheet.py OUT.png ROW_DIR... --frames 100000,200000,300000 [--scale 3] -- deeptrace maps as one picture (deep trace lane, 2026-10-06)

One row per run directory, one column per frame, each `map_fNNNNNN.txt` drawn a cell to a scale x scale block, with a
6-pixel gap between maps. No libraries: the PNG is written with zlib. Colours: air and dug space dark, ground brown,
loose spoil tan, water blue, ants red, brood cream, provisions and other food green, crumbs yellow, corpses purple; a
missing map is left grey. Under a picture, say which run is which row: the image carries no text.
"""
import sys, zlib, struct, os

COL = {
    ".": (26, 26, 34), "#": (107, 79, 50), "s": (200, 180, 138), "=": (150, 120, 90), "r": (80, 80, 80),
    "~": (106, 160, 200), "a": (210, 75, 60), "b": (239, 227, 194), "e": (239, 227, 194), "l": (239, 227, 194),
    "p": (239, 227, 194), "f": (95, 158, 87), "c": (224, 192, 64), "x": (150, 70, 160), "o": (26, 26, 34),
}
GAP, GREY, BG = 6, (60, 60, 60), (12, 12, 16)


def load(p):
    L = open(p).read().split("\n")
    x0, y0, w, h = map(int, L[0].split())
    return w, h, [r.ljust(w, "#")[:w] for r in L[1:1 + h]]


def main():
    a = sys.argv[1:]
    scale, frames = 3, [100_000, 200_000, 300_000]
    if "--scale" in a:
        i = a.index("--scale"); scale = int(a[i + 1]); del a[i:i + 2]
    if "--frames" in a:
        i = a.index("--frames"); frames = [int(f) for f in a[i + 1].split(",")]; del a[i:i + 2]
    out, runs = a[0], a[1:]
    grids = [[load(p) if os.path.exists(p := f"{r}/map_f{f:06d}.txt") else None for f in frames] for r in runs]
    w, h = next((g[0], g[1]) for row in grids for g in row if g)
    W = len(frames) * (w * scale + GAP) + GAP
    H = len(runs) * (h * scale + GAP) + GAP
    img = [[BG] * W for _ in range(H)]
    for ri, row in enumerate(grids):
        for ci, g in enumerate(row):
            ox, oy = GAP + ci * (w * scale + GAP), GAP + ri * (h * scale + GAP)
            for y in range(h):
                for x in range(w):
                    c = COL.get(g[2][y][x], (255, 0, 255)) if g else GREY
                    for dy in range(scale):
                        line = img[oy + y * scale + dy]
                        for dx in range(scale):
                            line[ox + x * scale + dx] = c
    raw = b"".join(b"\x00" + bytes(v for px in line for v in px) for line in img)
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    open(out, "wb").write(png)
    print(f"{out}: {W}x{H}, rows {', '.join(os.path.basename(r.rstrip('/')) for r in runs)}, columns {', '.join(f'{f // 1000}k' for f in frames)}")


if __name__ == "__main__":
    main()
