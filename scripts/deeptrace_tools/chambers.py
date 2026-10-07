"""chambers.py RUN_DIR... [--frames 100000,200000,300000] [--png OUT.png] | --selftest -- separate chambers in the goal box's dug nest,
by the owner's chamber rule (deep trace lane, 2026-10-06)

The rule is `examples/digbox.rs`'s `chambers_of`, ported line for line so the goal box and the dig box count chambers the
same way. It is the owner's ruling of 2026-09-20: "the chambers have to be readable. Chambers that are just two cells
tall are not going to look like a chamber in this game ... at some point the answer is just digging one giant hole ...
and then it's not chambers and tunnels." The spec is a passage of about 4 cells, chambers 8-16 tall and wider than tall,
and a chamber bore 2-4x the passage's.

How it counts:
- The room is every cell below the old ground line that is not ground (`s`, `#`, `=`). Ants, brood, food, crumbs,
  corpses and water stand in dug space, so they count as room. A chamber does not stop being one when something is in it.
- Every room cell gets its Chebyshev distance to the nearest non-room cell: the radius of the largest square of room
  centred on it. A 4-cell bore reads 2 whichever way it runs, so a shaft and a tunnel are one feature rotated.
- Cells at radius 4 or more (a bore of 7-8 cells, the owner's 2x-the-passage floor) are chamber core. The rest is passage.
- A chamber is an 8-connected piece of core. Two chambers joined by a tunnel narrower than the core never touch. A hole
  with no narrow waist is one chamber, and scratches reach the threshold nowhere and count as none.
- Each core cell claims the square it is the centre of, which gives the chamber's box (tall x wide) and its cells.
- The passage bore is the median of 2 x radius over the non-core room. The contrast is the median chamber height over
  that bore.
- A chamber is spec-shaped when its box is 8-16 tall and wider than tall. Separate rooms in the goal's sense are 2+ of those.

Why not the scorecard's old room rule: it keeps a cell with 7 of its 3x3 open and joins 4-connected cells, so anything
3+ cells wide is room. It calls the door shaft, the gallery and the pit one room on all 12 baseline maps, and it would
join two real chambers through any 3-wide tunnel.

The map reaches 70 rows under the ground line; a nest dug deeper is cut off there.
"""
import sys, os, re, glob, zlib, struct

GROUND = set("s#=r")
CHAMBER_R = 4


def founded(run):
    for L in open(f"{run}/events.txt"):
        if " FOUNDED " in L:
            kv = dict(t.split("=", 1) for t in L.split() if "=" in t)
            return int(kv["nest_x"]), int(kv["ground_y"])
    raise SystemExit(f"{run}: no FOUNDED line")


def load(path):
    L = open(path).read().split("\n")
    x0, y0, w, h = map(int, L[0].split())
    return x0, y0, w, h, L[1:1 + h]


def room_mask(path, gy):
    """The room below the old ground line: (mask, w, h, x0, top_y, rows), row 0 = ground_y + 1."""
    x0, y0, w, h, rows = load(path)
    top = gy + 1
    hh = y0 + h - top
    mask = [[rows[top - y0 + j][i] not in GROUND for i in range(w)] for j in range(hh)]
    return mask, w, hh, x0, top, rows


def chebyshev(mask, w, h):
    FAR = 1 << 20
    d = [[FAR if mask[y][x] else 0 for x in range(w)] for y in range(h)]
    for y in range(h):
        for x in range(w):
            if d[y][x] == 0:
                continue
            best = FAR
            if x > 0:
                best = min(best, d[y][x - 1])
            if y > 0:
                best = min(best, d[y - 1][x])
                if x > 0:
                    best = min(best, d[y - 1][x - 1])
                if x + 1 < w:
                    best = min(best, d[y - 1][x + 1])
            d[y][x] = min(d[y][x], best + 1)
    for y in range(h - 1, -1, -1):
        for x in range(w - 1, -1, -1):
            if d[y][x] == 0:
                continue
            best = FAR
            if x + 1 < w:
                best = min(best, d[y][x + 1])
            if y + 1 < h:
                best = min(best, d[y + 1][x])
                if x + 1 < w:
                    best = min(best, d[y + 1][x + 1])
                if x > 0:
                    best = min(best, d[y + 1][x - 1])
            d[y][x] = min(d[y][x], best + 1)
    return d


def chambers(mask, w, h, r=CHAMBER_R):
    """digbox's chambers_of: (chambers, passage, contrast, d, claim). Each chamber is a dict with h, w, cells (claimed,
    after earlier chambers' claims), top and left (rows below the ground line and map column)."""
    d = chebyshev(mask, w, h)
    seen = [[False] * w for _ in range(h)]
    claim = [[-1] * w for _ in range(h)]
    found = []
    for sy in range(h):
        for sx in range(w):
            if d[sy][sx] < r or seen[sy][sx]:
                continue
            x0 = y0 = 1 << 30
            x1 = y1 = -(1 << 30)
            k = len(found)
            seen[sy][sx] = True
            stack = [(sx, sy)]
            core = []
            while stack:
                x, y = stack.pop()
                core.append((x, y))
                q = d[y][x] - 1
                x0, x1, y0, y1 = min(x0, x - q), max(x1, x + q), min(y0, y - q), max(y1, y + q)
                for dy in (-1, 0, 1):
                    for dx in (-1, 0, 1):
                        nx, ny = x + dx, y + dy
                        if 0 <= nx < w and 0 <= ny < h and d[ny][nx] >= r and not seen[ny][nx]:
                            seen[ny][nx] = True
                            stack.append((nx, ny))
            for x, y in core:
                q = d[y][x] - 1
                for yy in range(max(0, y - q), min(h, y + q + 1)):
                    for xx in range(max(0, x - q), min(w, x + q + 1)):
                        if mask[yy][xx] and claim[yy][xx] < 0:
                            claim[yy][xx] = k
            found.append({"h": y1 - y0 + 1, "w": x1 - x0 + 1, "top": y0, "left": x0, "core": len(core)})
    for c in found:
        c["cells"] = 0
    for y in range(h):
        for x in range(w):
            if claim[y][x] >= 0:
                found[claim[y][x]]["cells"] += 1
    bores = sorted(2 * d[y][x] for y in range(h) for x in range(w) if mask[y][x] and d[y][x] < r)
    passage = bores[len(bores) // 2] if bores else 0
    hs = sorted(c["h"] for c in found)
    med_h = hs[len(hs) // 2] if hs else 0
    contrast = med_h / passage if passage else 0.0
    return found, passage, contrast, d, claim


def measure(path, gy):
    mask, w, h, x0, top, rows = room_mask(path, gy)
    found, passage, contrast, d, claim = chambers(mask, w, h)
    room = sum(map(sum, mask))
    return {"chambers": found, "passage": passage, "contrast": contrast, "room": room,
            "in_chambers": sum(c["cells"] for c in found)}


def spec_shaped(c):
    """The owner's chamber: 8-16 tall and wider than tall."""
    return 8 <= c["h"] <= 16 and c["w"] > c["h"]


def describe(m):
    """One line: the count, each chamber tall x wide (biggest first), how many are spec-shaped, the passage bore and
    the contrast."""
    ch = sorted(m["chambers"], key=lambda c: -c["cells"])
    shapes = " ".join(f"{c['h']}x{c['w']}" for c in ch) or "-"
    return f"{len(ch)} ({shapes}); spec {sum(map(spec_shaped, ch))}; passage {m['passage']}, {m['contrast']:.1f}x"


# --- the picture: ground tan, passage dark, chamber cells by chamber, core brighter; ants red, brood cream on top ---
PAL = [(222, 90, 200), (70, 170, 230), (240, 200, 60), (120, 220, 120), (240, 130, 60), (180, 140, 250)]


def png(out, runs, frames, scale=3):
    tiles = []
    for run in runs:
        nx, gy = founded(run)
        row = []
        for f in frames:
            p = f"{run}/map_f{f:06d}.txt"
            if not os.path.exists(p):
                row.append(None)
                continue
            mask, w, h, x0, top, rows = room_mask(p, gy)
            found, passage, contrast, d, claim = chambers(mask, w, h)
            row.append((mask, w, h, d, claim, rows, top, x0))
        tiles.append(row)
    W = max(t[1] for r in tiles for t in r if t) * scale
    H = max(t[2] for r in tiles for t in r if t) * scale
    gap = 6
    TW, TH = len(frames) * (W + gap) - gap, len(tiles) * (H + gap) - gap
    canvas = [[(10, 10, 14)] * TW for _ in range(TH)]
    for ri, r in enumerate(tiles):
        for ci, t in enumerate(r):
            if not t:
                continue
            mask, w, h, d, claim, rows, top, x0 = t
            for y in range(h):
                for x in range(w):
                    if not mask[y][x]:
                        c = (198, 178, 140)
                    elif claim[y][x] >= 0:
                        base = PAL[claim[y][x] % len(PAL)]
                        c = base if d[y][x] >= CHAMBER_R else tuple(int(v * 0.55) for v in base)
                    else:
                        c = (34, 34, 44)
                    for yy in range(scale):
                        for xx in range(scale):
                            canvas[ri * (H + gap) + y * scale + yy][ci * (W + gap) + x * scale + xx] = c
    raw = b"".join(b"\x00" + bytes(v for px in line for v in px) for line in canvas)
    chunk = lambda t, dd: struct.pack(">I", len(dd)) + t + dd + struct.pack(">I", zlib.crc32(t + dd) & 0xFFFFFFFF)
    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", TW, TH, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    open(out, "wb").write(data)
    print(f"{out}: {TW}x{TH}, rows {', '.join(os.path.basename(r.rstrip('/')) for r in runs)}, columns {', '.join(f'{f // 1000}k' for f in frames)}")


def selftest():
    """Positive and negative controls on carved maps with known answers (ground row 160, map from row 100 like deeptrace's)."""
    import tempfile
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import scorecard

    def carve(*boxes):
        w, h, x0, y0 = 151, 131, 176, 100
        g = [["." if y < 60 else "s" for x in range(w)] for y in range(h)]
        for (bx, by, bw, bh) in boxes:  # map columns, rows below the ground line (1 = first row under it)
            for y in range(by, by + bh):
                for x in range(bx, bx + bw):
                    g[60 + y][x] = "."
        fd, path = tempfile.mkstemp(suffix=".txt")
        with os.fdopen(fd, "w") as fo:
            fo.write(f"{x0} {y0} {w} {h}\n" + "\n".join("".join(r) for r in g) + "\n")
        return path

    cases = [
        ("two 12x20 rooms joined by a 4-wide tunnel", [(20, 10, 20, 12), (40, 15, 30, 4), (70, 10, 20, 12)], 2, 2, 1),
        ("one hole 24 tall x 40 wide", [(50, 5, 40, 24)], 1, 0, 1),
        ("a 3-wide shaft 30 deep", [(75, 1, 3, 30)], 0, 0, 0),
        ("a gallery 5 tall x 60 wide", [(40, 6, 60, 5)], 0, 0, 1),
        ("one room 12 tall x 24 wide", [(60, 8, 24, 12)], 1, 1, 1),
    ]
    bad = 0
    for name, boxes, want, want_spec, old_want in cases:
        path = carve(*boxes)
        m = measure(path, 160)
        got, spec = len(m["chambers"]), sum(map(spec_shaped, m["chambers"]))
        old = len(scorecard.rooms(path, 160)[0])
        ok = got == want and spec == want_spec and old == old_want
        bad += not ok
        print(f"{'ok ' if ok else 'BAD'} {name}: chambers {got} (want {want}), spec-shaped {spec} (want {want_spec}); old room rule {old} (want {old_want})")
        os.remove(path)
    print("selftest", "passed" if not bad else f"FAILED {bad}")
    return bad


def main():
    a = sys.argv[1:]
    if a == ["--selftest"]:
        sys.exit(selftest())
    frames = [100_000, 200_000, 300_000]
    out = None
    if "--frames" in a:
        i = a.index("--frames"); frames = [int(f) for f in a[i + 1].split(",")]; del a[i:i + 2]
    if "--png" in a:
        i = a.index("--png"); out = a[i + 1]; del a[i:i + 2]
    for run in a:
        nx, gy = founded(run)
        for f in frames:
            p = f"{run}/map_f{f:06d}.txt"
            if os.path.exists(p):
                print(f"{os.path.basename(run.rstrip('/'))} {f // 1000}k: chambers {describe(measure(p, gy))}")
    if out:
        png(out, a, frames)


if __name__ == "__main__":
    main()
