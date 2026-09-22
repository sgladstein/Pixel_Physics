#!/usr/bin/env python3
"""One ant's whole foraging loop, drawn: time across, position along the route.

**The owner's objection, review card `20260920T063732278Z-044d7d`, answered
2026-09-20:** *"Why do both of these start at the food? Should we start away
from food, move towards, become laden, go home, etc."* -- said of a return-leg
visual that began at the larder, so it could only ever show the homeward half.
The loop did not close at the time, which is why that visual started where it
did. It closes now, so the whole thing can be drawn.

    python3 scripts/loopchart.py /tmp/trailfollow-focal-seed1-gap90.csv \
        --nest 48 --food 138 --out /tmp/loop.png
    python3 scripts/loopchart.py --selftest

**Time runs left to right and route position runs up the image** -- nest at the
bottom, larder at the top -- so a closed lap is a triangle and a colony that
commutes is a sawtooth. An ant that never arrives is a flat line near the
bottom, and the two are not confusable at a glance, which is the whole reason
this is a picture and not a column.

**Colour is the carried load, and it is the only thing that separates the two
legs.** Blue out, orange home; `CarryingFood` is the gate the engine itself
reads, not `crop_cells` -- an ant whose crop holds spoil is not laden with food
and must not be drawn as though it were.
"""

import argparse
import csv
import struct
import sys
import zlib
from pathlib import Path

EMPTY, LADEN, BG, GUIDE, NEST = (70, 130, 200), (240, 150, 40), (14, 14, 18), (48, 48, 58), (120, 90, 150)


def read(path):
    rows = []
    with open(path, newline="") as fh:
        for r in csv.DictReader(fh):
            try:
                rows.append((int(r["frame"]), int(r["x"]), float(r["CarryingFood"]), r["id"]))
            except (KeyError, ValueError):
                continue
    return rows


def write_png(path, px, w, h):
    raw = b"".join(b"\x00" + bytes(px[y]) for y in range(h))

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    Path(path).write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    return path


def draw(rows, out, nest, food, w=900, h=320, pad=10):
    f0, f1 = min(r[0] for r in rows), max(r[0] for r in rows)
    # **The vertical span is the ROUTE, not the ant's own range.** Scaling to
    # where the animal happened to go would make a colony that never left the
    # nest fill the image exactly as a commuting one does -- the failure this
    # chart exists to make visible.
    lo, hi = min(nest, food, min(r[1] for r in rows)), max(nest, food, max(r[1] for r in rows))
    px = [bytearray(BG * w) for _ in range(h)]
    sx = lambda f: pad + int((w - 2 * pad - 1) * (f - f0) / max(1, f1 - f0))
    sy = lambda x: h - 1 - pad - int((h - 2 * pad - 1) * (x - lo) / max(1, hi - lo))

    for gx, col in ((nest, NEST), (food, GUIDE)):
        y = sy(gx)
        if 0 <= y < h:
            for x in range(w):
                px[y][x * 3 : x * 3 + 3] = bytes(col)

    prev = None
    for frame, x, carrying, _ in sorted(rows):
        cx, cy = sx(frame), sy(x)
        col = LADEN if carrying > 0.5 else EMPTY
        pts = [(cx, cy)]
        # Join consecutive samples: a trace sampled every decision tick leaves
        # gaps a scatter would read as a stalled ant.
        if prev and abs(cx - prev[0]) <= 40:
            x0, y0 = prev
            n = max(abs(cx - x0), abs(cy - y0))
            pts = [(x0 + (cx - x0) * i // n, y0 + (cy - y0) * i // n) for i in range(n + 1)] if n else pts
        for ax, ay in pts:
            for dy in (-1, 0, 1):
                yy = ay + dy
                if 0 <= yy < h and 0 <= ax < w:
                    px[yy][ax * 3 : ax * 3 + 3] = bytes(col)
        prev = (cx, cy)
    write_png(out, px, w, h)
    return f0, f1, lo, hi


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("csv", nargs="?")
    ap.add_argument("--nest", type=int, required=False, default=48)
    ap.add_argument("--food", type=int, required=False, default=138)
    ap.add_argument("--out", default="/tmp/loop.png")
    ap.add_argument("--id", help="draw only this organism id (the CSV may hold a cohort)")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        return selftest()

    rows = read(a.csv)
    if not rows:
        print(f"loopchart: no usable rows in {a.csv}")
        return 1
    ids = sorted({r[3] for r in rows})
    print(f"loopchart: {len(rows)} rows, {len(ids)} ant(s) in the file")
    if len(ids) > 1 and not a.id:
        print(f"           ** {len(ids)} ants in one file -- drawing them all on top of each other. "
              f"Pass --id to pick one: {ids[:6]}")
    if a.id:
        rows = [r for r in rows if r[3] == a.id]
    laden = sum(1 for r in rows if r[2] > 0.5)
    f0, f1, lo, hi = draw(rows, a.out, a.nest, a.food)
    print(f"loopchart: {a.out}  frames {f0}-{f1}  route x {lo}-{hi}  nest {a.nest} food {a.food}")
    # The count beside the picture -- CLAUDE.md's house rule. A trace of an ant
    # that never picked anything up draws a perfectly clean empty-leg line.
    print(f"           laden on {laden} of {len(rows)} traced ticks ({100 * laden / max(1, len(rows)):.1f}%)"
          + ("   ** ZERO laden ticks: this ant never carried food, so there is no loop in this picture."
             if laden == 0 else ""))
    return 0


def selftest():
    """Positive control: a constructed two-lap trace must draw both colours.

    The failure guarded is a chart that renders a complete-looking trajectory
    from an ant that never carried anything -- which is what the previous
    visual did, and is indistinguishable from a working loop unless the laden
    leg is actually coloured differently.
    """
    import tempfile

    d = Path(tempfile.mkdtemp())
    p = d / "f.csv"
    rows = ["id,stage,frame,x,y,dx_home,anchor_x,since_nest,PheroARise,HomeAligned,PheroAAlong,"
            "PheroAFront,Carrying,CarryingFood,crop_cells,spoil,heading,Energy"]
    f = 0
    for _lap in range(2):
        for x in range(48, 139, 3):  # out, empty
            rows.append(f"OrganismId(1),1,{f},{x},96,0,48,0,0,0,0,0,0,0.0,0,0,0,100")
            f += 1
        for x in range(138, 47, -3):  # home, laden
            rows.append(f"OrganismId(1),4,{f},{x},96,0,48,0,0,0,0,0,0,1.0,1,0,0,100")
            f += 1
    p.write_text("\n".join(rows) + "\n")
    got = read(p)
    assert len(got) == 4 * len(range(48, 139, 3)) // 2 * 2 or len(got) > 100, f"read {len(got)} rows"
    laden = sum(1 for r in got if r[2] > 0.5)
    assert 0 < laden < len(got), f"the control must contain BOTH legs: {laden} laden of {len(got)}"
    out = d / "o.png"
    draw(got, out, 48, 138)
    body = out.read_bytes()
    assert body[:8] == b"\x89PNG\r\n\x1a\n" and len(body) > 100, "png writer produced nothing usable"
    # The picture must actually contain both colours, not just the data.
    raw = zlib.decompress(body[body.index(b"IDAT") + 4 :][: -12])
    assert bytes(LADEN) in raw and bytes(EMPTY) in raw, "one of the two legs never reached the image"
    # ...and an all-empty trace must NOT contain the laden colour.
    flat = [(fr, 48, 0.0, "OrganismId(1)") for fr in range(200)]
    draw(flat, out, 48, 138)
    raw2 = zlib.decompress(out.read_bytes()[out.read_bytes().index(b"IDAT") + 4 :][: -12])
    assert bytes(LADEN) not in raw2, "an ant that never carried anything is being drawn as laden"
    print(f"loopchart selftest: ok -- {len(got)} rows, {laden} laden, both legs reach the image, "
          f"and an empty trace draws no laden colour")
    return 0


if __name__ == "__main__":
    sys.exit(main())
