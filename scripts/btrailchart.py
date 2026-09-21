#!/usr/bin/env python3
"""Turn `trailfollow btrail` rows into a picture of the food trail over time.

The owner's question, 2026-09-21: *"how the hand laid trail looks compared to
the ant laid trail after 6000 ticks, and in general how that trail changes over
time."* That is a shape question across two axes -- where along the route, and
when -- so the answer is a heatmap and not a table: **x is the route from the
ramp's foot to the larder, y is time running downward, brightness is channel B.**
The frame `stop` withdrew the hand-laid ramp at is drawn as a rule across the
image, so the handover is a thing you look at rather than infer.

Why this is not the `b_profile` column already in the harness: that column is a
MEAN over the whole run, so with `stop=` set it is dominated by the hand-laid
era and prints healthy numbers for a run whose plane is empty for most of its
length. See `examples/trailfollow.rs`'s doc on the field.

    python3 scripts/btrailchart.py run.log --out /tmp/btrail.png
    python3 scripts/btrailchart.py run.log --out /tmp/btrail.png --seed 3
    python3 scripts/btrailchart.py --selftest

**It prints the key's cardinality before it draws anything**, and that is not
decoration: `CLAUDE.md`'s *a parse is a measurement, and it inherits every
dimension the run swept*. A log keyed on fewer dimensions than the run varied
pools them silently, last write wins, and the result is a complete plausible
picture of nothing. If the row count is not `seeds x gaps x layfrom x frames`,
something is being overwritten and the number says so.
"""

import argparse
import re
import struct
import sys
import zlib
from collections import defaultdict
from pathlib import Path

KV = re.compile(r"(\w+)=([^\s]+)")


def parse(lines):
    """`BTRAIL` rows as dicts. Everything else in the log is ignored."""
    rows = []
    for line in lines:
        if not line.startswith("BTRAIL "):
            continue
        d = dict(KV.findall(line))
        if "prof" not in d:
            continue
        d["prof"] = [int(v) for v in d["prof"].split(",") if v != ""]
        for k in ("seed", "frame", "stop", "gap", "x0", "x1", "cells", "peak", "hand"):
            if k in d:
                d[k] = int(d[k])
        rows.append(d)
    return rows


def ramp(v, vmax):
    """Dark -> bright, a FULL REPLACE rather than a blend.

    `CLAUDE.md`'s overlay rule, which was learned on a canopy sheet that read as
    blank because a magnitude-scaled blend moved one colour byte from 139 to
    155. Black -> deep blue -> orange -> white, so both ends of the range are
    separable and a faint trail is visible rather than nearly-black.
    """
    if v <= 0:
        return (14, 14, 18)
    t = (v / vmax) ** 0.45 if vmax > 0 else 0.0
    t = min(1.0, max(0.0, t))
    stops = [(0.0, (20, 30, 80)), (0.35, (40, 110, 190)), (0.7, (240, 150, 40)), (1.0, (255, 250, 235))]
    for i in range(len(stops) - 1):
        t0, c0 = stops[i]
        t1, c1 = stops[i + 1]
        if t <= t1:
            f = 0.0 if t1 == t0 else (t - t0) / (t1 - t0)
            return tuple(int(c0[j] + (c1[j] - c0[j]) * f) for j in range(3))
    return stops[-1][1]


def continuity(prof, reach):
    """Is this route profile a TRAIL, or bright cells that happen to be on it?

    **A peak is the wrong metric and this function exists because it was used.**
    Owner, 2026-09-21, reading a panel whose ant-laid peak was 78% of the
    hand-laid ramp: *"it is caused by a single bright spot not a good trail from
    nest to food."* Exactly right -- `max()` over cells cannot see a hole, and a
    trail with a hole in it is not a trail.

    So this reports what an animal walking the route would meet:

    * `lit` -- cells holding anything at all;
    * `gap` -- the longest unbroken DARK run, which is the thing that strands an
      ant, and the number a peak is blind to;
    * `bridged` -- whether every dark run is shorter than `reach`, the ant's own
      `sensor_offset`. **This is the one to quote.** A gap the animal can see
      across is not a break in the trail; one it cannot is, however bright the
      cells either side of it are.

    `reach` is the consumer's number rather than a constant of this script --
    `CLAUDE.md`: measure the number the consumer computes, never the stored
    value.
    """
    lit = sum(1 for v in prof if v > 0)
    longest = run = 0
    for v in prof:
        run = run + 1 if v == 0 else 0
        longest = max(longest, run)
    return lit, longest, longest < reach


def write_png(path, pixels, w, h):
    raw = b"".join(b"\x00" + bytes(pixels[y]) for y in range(h))

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    Path(path).write_bytes(png)
    return path


def draw(rows, out, cell_w=6, cell_h=6, gutter=28, after_only=False):
    """One panel: time down, route across.

    `after_only` rescales the ramp to the brightest cell seen AFTER the
    hand-laid trail was withdrawn. **Both panels are needed and neither is
    sufficient.** Scaled to the whole run, the ants' own trail is honest and
    nearly invisible, because the hand-laid ramp is up to two orders brighter --
    which is the headline, and also hides the shape. Scaled to the ant era the
    shape is legible and the magnitude is a lie unless the other panel is beside
    it. Quoting only the second is how a faint trail gets reported as a working
    one.
    """
    rows = sorted(rows, key=lambda r: r["frame"])
    prof_len = max(len(r["prof"]) for r in rows)
    pool = [r for r in rows if not r.get("hand")] if after_only else rows
    if not pool:
        pool = rows
    vmax = max((max(r["prof"]) if r["prof"] else 0) for r in pool)
    if vmax == 0:
        print("btrailchart: every sample is zero -- nothing to draw. That is a "
              "result, but check the arm laid a trail at all before believing it.")
    w = prof_len * cell_w + gutter
    h = len(rows) * cell_h
    px = [bytearray((14, 14, 18) * w) for _ in range(h)]

    for ri, r in enumerate(rows):
        hand = r.get("hand", 0)
        for y in range(ri * cell_h, (ri + 1) * cell_h):
            # The gutter carries one mark: lit while the hand-laid ramp is
            # still being refreshed, dark once the colony is on its own. It is
            # the only thing in the image that says WHICH TRAIL you are
            # looking at, and without it the handover is a guess.
            band = (200, 60, 60) if hand else (45, 45, 55)
            for x in range(gutter - 6):
                px[y][x * 3 : x * 3 + 3] = bytes(band)
            for xi, v in enumerate(r["prof"]):
                c = ramp(v, vmax)
                x0 = gutter + xi * cell_w
                for x in range(x0, x0 + cell_w):
                    px[y][x * 3 : x * 3 + 3] = bytes(c)
    write_png(out, px, w, h)
    return w, h, vmax, prof_len


def grid(panels, keys, out, cell_w=5, cell_h=5, gutter=22, sep=8):
    """Stack one panel per key, each on its OWN scale, with a label gutter.

    Per-panel scaling is deliberate and is the thing to be careful about: it
    makes each colony's *shape* readable and makes the panels non-comparable in
    brightness. The per-seed peaks belong in the caption for that reason, and
    the printout below emits them.
    """
    prof_len = max(len(r["prof"]) for p in panels for r in p)
    ph = max(len(p) for p in panels) * cell_h
    w = prof_len * cell_w + gutter
    h = len(panels) * (ph + sep)
    px = [bytearray((14, 14, 18) * w) for _ in range(h)]
    for pi, rows in enumerate(panels):
        rows = sorted(rows, key=lambda r: r["frame"])
        pool = [r for r in rows if not r.get("hand")] or rows
        vmax = max((max(r["prof"]) if r["prof"] else 0) for r in pool)
        top = pi * (ph + sep)
        for ri, r in enumerate(rows):
            for y in range(top + ri * cell_h, top + (ri + 1) * cell_h):
                if y >= h:
                    break
                band = (200, 60, 60) if r.get("hand") else (45, 45, 55)
                for x in range(gutter - 6):
                    px[y][x * 3 : x * 3 + 3] = bytes(band)
                for xi, v in enumerate(r["prof"]):
                    c = ramp(v, vmax)
                    x0 = gutter + xi * cell_w
                    for x in range(x0, min(x0 + cell_w, w)):
                        px[y][x * 3 : x * 3 + 3] = bytes(c)
    write_png(out, px, w, h)
    print(f"btrailchart: {out}  {w}x{h}  {len(panels)} panels, top to bottom:")
    for k, rows in zip(keys, panels):
        hand = [r for r in rows if r.get("hand")]
        ant = [r for r in rows if not r.get("hand")]
        hp = max((r["peak"] for r in hand), default=0)
        ap_ = max((r["peak"] for r in ant), default=0)
        print(f"             {k}  hand-laid peak {hp:6d}  ant-laid peak {ap_:6d}"
              f"  ({100 * ap_ / max(1, hp):4.1f}% of it)   ** each panel on its OWN scale")
    return 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("log", nargs="?")
    ap.add_argument("--out", default="/tmp/btrail.png")
    ap.add_argument("--seed", type=int)
    ap.add_argument("--layfrom")
    ap.add_argument("--stats", action="store_true",
                    help="continuity per key and era -- lit cells, worst dark gap, and whether an ant could bridge it")
    ap.add_argument("--reach", type=int, default=6,
                    help="the ant's sensor_offset; a dark run shorter than this is not a break (default 6)")
    ap.add_argument("--grid", action="store_true",
                    help="every key as a panel in one image -- the distribution, not one draw")
    ap.add_argument("--after", action="store_true",
                    help="rescale to the brightest cell AFTER the hand-laid ramp stops -- see draw()")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()

    if a.selftest:
        return selftest()

    rows = parse(Path(a.log).read_text().splitlines())
    if not rows:
        print(f"btrailchart: no BTRAIL rows in {a.log} -- was the run given `btrail`?")
        return 1

    # **The cardinality check, before anything is drawn.** See the module doc.
    key = lambda r: (r["seed"], r.get("gap"), r.get("layfrom"), r.get("arm"))
    by_key = defaultdict(list)
    for r in rows:
        by_key[key(r)].append(r)
    seeds = sorted({r["seed"] for r in rows})
    gaps = sorted({r.get("gap") for r in rows})
    lf = sorted({r.get("layfrom") for r in rows})
    frames = sorted({r["frame"] for r in rows})
    print(f"btrailchart: {len(rows)} rows over {len(by_key)} (seed,gap,layfrom,arm) keys")
    print(f"             seeds {seeds}  gaps {gaps}  layfrom {lf}  {len(frames)} sample frames")
    expect = len(by_key) * len(frames)
    if expect != len(rows):
        print(f"             ** {len(rows)} rows against {expect} = keys x frames: the sweep is NOT "
              f"square, so some key is short or duplicated. Do not read a picture off this "
              f"until you know which.")

    if a.stats:
        print(f"\n{'key':<40} {'lit/route':>12} {'worst dark gap':>15} {'bridged':>10}")
        print("-" * 82)
        for k in sorted({key(r) for r in rows}, key=str):
            for era, want_hand in (("hand-laid", True), ("ant-laid", False)):
                sel = [r for r in rows if key(r) == k and bool(r.get("hand")) == want_hand]
                if not sel:
                    continue
                stats = [continuity(r["prof"], a.reach) for r in sel]
                n = len(stats)
                lit = sorted(x[0] for x in stats)[n // 2]
                gap = sorted(x[1] for x in stats)[n // 2]
                br = 100.0 * sum(1 for x in stats if x[2]) / n
                route = len(sel[0]["prof"])
                print(f"{str(k) + ' ' + era:<40} {lit:>5}/{route:<6} {gap:>15} {br:>9.0f}%")
        print(f"\n  medians over sampled frames. `bridged` = the share of frames with NO dark run")
        print(f"  at least {a.reach} cells long -- the ant's own sensor reach, so a gap it can see")
        print(f"  across does not count as a break. A peak cannot see a hole; this is what replaces it.")
        return 0

    if a.grid:
        # **Every seed in one image, because one seed is a draw from a wide
        # distribution.** Measured 2026-09-21: over four seeds of one arm the
        # ants' own peak ran 18%, 27%, 32% and 78% of the hand-laid ramp, and
        # the route was bare in the last 6,000 frames on one seed and lit on 70
        # of 91 cells on another. A single panel would have been read as the
        # result. `CLAUDE.md`: an outcome is a distribution, not a binary.
        keys = sorted({key(r) for r in rows}, key=str)
        panels = [[r for r in rows if key(r) == k] for k in keys]
        return grid(panels, keys, a.out)

    sel = [r for r in rows if (a.seed is None or r["seed"] == a.seed) and (a.layfrom is None or r.get("layfrom") == a.layfrom)]
    if not sel:
        print("btrailchart: nothing matched --seed/--layfrom")
        return 1
    k = {key(r) for r in sel}
    if len(k) != 1:
        print(f"btrailchart: {len(k)} keys selected -- narrow with --seed/--layfrom, or "
              f"they will be drawn on top of each other:\n  " + "\n  ".join(str(x) for x in sorted(k, key=str)))
        return 1

    stop = sel[0].get("stop", 0)
    w, h, vmax, n = draw(sel, a.out, after_only=a.after)
    print(f"btrailchart: {a.out}  {w}x{h}  route {n} cells  peak channel B {vmax}{' (AFTER the handover only)' if a.after else ''}  stop={stop}")
    print(f"             left band RED while the hand-laid ramp is refreshed, GREY after.")
    return 0


def selftest():
    """The positive control: a bed whose answer is known, drawn and read back.

    `CLAUDE.md` -- a guard that cannot go red is blind, not strong. This builds
    two synthetic runs, one with a trail and one without, and asserts the
    parser separates them and the ramp maps a non-zero cell to a non-background
    colour. It is the check that would have caught a chart script reporting a
    clean empty plane because its regex missed `prof=`.
    """
    good = ["BTRAIL seed=1 arm=shipped stop=600 gap=90 layfrom=nest x0=48 x1=52 "
            f"frame={f} hand={1 if f <= 600 else 0} cells=5 peak=255 prof=0,64,128,192,255"
            for f in (200, 400, 600, 800)]
    rows = parse(good)
    assert len(rows) == 4, f"parsed {len(rows)} of 4"
    assert rows[0]["prof"] == [0, 64, 128, 192, 255], rows[0]["prof"]
    assert [r["hand"] for r in rows] == [1, 1, 1, 0], "the hand/ant handover is not being read"
    assert parse(["nonsense", "BTRAIL seed=1 frame=1"]) == [], "a row with no prof= must be dropped, not half-read"
    bg, lit = ramp(0, 255), ramp(255, 255)
    assert bg != lit and sum(lit) > sum(bg), f"the ramp does not separate empty from full: {bg} {lit}"
    assert ramp(20, 255) != bg, "a FAINT cell is being drawn as background -- the exact failure this ramp exists to avoid"
    out = write_png("/tmp/btrailchart_selftest.png", [bytearray((1, 2, 3) * 4) for _ in range(4)], 4, 4)
    assert Path(out).stat().st_size > 0
    spike = [0] * 40 + [60000] + [0] * 40
    even = [800] * 81
    holed = [800] * 38 + [0] * 8 + [800] * 35
    assert max(spike) > max(even), "the control is not constructed: the spike must WIN on peak"
    assert not continuity(spike, 6)[2], "a single bright spot is being called a connected trail"
    assert continuity(even, 6)[2], "an unbroken dim trail is not being called connected"
    assert not continuity(holed, 6)[2], f"an 8-cell hole at reach 6 must break it: {continuity(holed, 6)}"
    assert continuity(holed, 9)[2], "...and must NOT break it once the animal can see that far"
    assert continuity(spike, 6)[1] == 40, f"worst-gap is wrong: {continuity(spike, 6)}"
    print("btrailchart selftest: ok -- parse, handover flag, ramp separation, faint-cell visibility, png writer,")
    print("                      continuity (a spike WINS on peak and correctly fails as a trail)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
