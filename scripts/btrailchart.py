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
picture of nothing. If the row count is not `keys x frames`, something is being
overwritten and the number says so.

**Stage 0 of `Reports/food-trail-plan-2026-09-29.md` widened the rows**, and
this script widened with them. A run now writes one row per (channel, kind,
rule, y) per sample frame -- live, replayed, counterfactual (`cf`) and
design-level (`onetrail mode=stream`) planes, over the band of walking rows,
from west of the door to past the pile -- plus `BSHAFT` column maxima down the
shaft. So every readout first SELECTS, and says what it selected:

    --kind live|replay|cf|design|any   (default live)
    --ch A|B                           (default B)
    --rule R                           (cf/design rules: odo32, gate, brain, ...)
    --y ROW                            (default: the surface row, below)
    --seed/--arm/--gap/--gate/--ft/--piles/--layfrom

The default `y` is **the surface row**, trailfollow's `surface`: the row
`b_profile` reads and the old logs' only row, found from the band's shape
(`surface-3..=surface+1`), never from the B in it, so every arm of a seed is
read on the same row (see `surface_y` for the case that forced this). **It is
not the row the ants walk**: measured on the 2026-09-29 dwide trace, 65% of
empty heads away from the door stand at surface+1 and 42% of B is laid there
(10% at the surface), so `--y <surface+1>` reads the trail as it is laid. The
pick is printed; pass `--y` to read another. Old logs (2026-09-21, before the widening) have no `gate`, `ft`,
`piles`, `nest`, `target`, `ch`, `kind`, `rule` or `y` keys, and their `arm=`
carried the GATE name: they parse as `gate=<that> arm=? ch=B kind=live rule=-
y=None nest=x0 target=x1`, so every option below still runs on them.

Readouts, each per key and era (hand-laid while the ramp is refreshed,
ant-laid after):

    --stats   continuity at --reach (below), and the three shape numbers the
              plan registers predictions on: `slope`, the mean over frames of
              the OLS slope of ln(1+B) against cells toward the food, nest to
              pile; `rising`, the share of adjacent lit pairs (both >= --thresh,
              default 256 raw = TRAIL_HALF/4) that increase toward the food,
              pooled over frames; `pile/door`, the time-mean B within 3 cells of
              the pile over the same at the door. Then a per-group line over
              seeds (rising < 0.5 on k/n, > 0.9 on k/n, slope > 0 on k/n) --
              P0.4 is counted there, not by eye.
    --door    the E-W gradient at the door over time: g = (bF - bA)/(bF + bA +
              1024), bF the food side (max of nest+1, nest+2 toward the food),
              bA the other; and at reach 6 (nest+6 against nest-6, the sensor
              point). Medians per era; --series prints every frame.
    --expect SPEC   live | expected | live - expected on ONE fixed log scale
              (the plan's `ln(1+v/256)/ln(1+4*DEPOSIT/256)`), plus rising and
              slope against time for both. SPEC is a file (e.g. `onetrail
              mode=stream` output, `kind=design` rows) or a selector on this
              log: `cf:odo32`, `design`, `replay`, `same`; --expect-kind and
              --expect-rule override. cf/replay rows pair on the same run and
              row; design rows pair on gap, in distance from the nest.
    --shaft   BSHAFT rows: time-mean column maximum by depth, and a picture.

**--reach defaults to 6 for compatibility** with the 2026-09-21 readings (the
old sensor offset). **The chooser's own reach is 2**: `trail_presence` reads
the cells one and two along a heading, so for the trail as today's ant meets
it, pass `--reach 2`.

The three shape numbers are defined exactly as `onetrail mode=stream`'s
`STREAM` line computes them (route u = 0..=span, OLS over every route cell,
7-cell windows, the 1,024 floor in the gradient), so the design model's rows
read back through `--stats --late` must reproduce its own summary: two
instruments, one definition, and a disagreement is a bug in one of them.

Controls (`--selftest`, exits 1 on any failure): a rising ramp reads rising >
0.9 and slope > 0; a nest-peaked profile rising < 0.5 and slope < 0; iid noise
rising ~0.5; the same ramp mirrored (food toward -x) reads identically; a
brighter food side reads a positive door gradient and an old row, which starts
at the door, reads none rather than zero; old-format rows still parse; a wide
row's dark cells west of the door do not break continuity on the route.
"""

import argparse
import math
import random
import re
import statistics as st
import struct
import sys
import zlib
from collections import Counter, defaultdict
from pathlib import Path

KV = re.compile(r"(\w+)=([^\s]+)")

# `pheromone::DEPOSIT` and the chooser's half-saturation, raw. The script reads
# the plane in the units the ant does -- `CLAUDE.md`: measure the number the
# consumer computes, never the stored value.
DEPOSIT = 40 * 256
TRAIL_HALF = DEPOSIT // 10
# The fixed log scale of the plan's `gifoverlay`: the same brightness means the
# same B in every panel of every run, so two panels can be compared by eye.
LOG_TOP = math.log1p(4 * DEPOSIT / 256)

INTS = ("seed", "frame", "stop", "gap", "x0", "x1", "cells", "peak", "hand", "nest", "target", "y", "xlo", "xhi", "y0", "surface")
# trailfollow's band of BTRAIL rows, `surface-3..=surface+1`: the rows the
# hand-laid trail paints. How `surface_y` finds the surface without reading B.
BAND_ABOVE, BAND_BELOW = 3, 1
# Every dimension a run can sweep. `layfrom` is not in the plan's list and is
# kept because the 2026-09-21 logs swept it: dropping a swept dimension from
# the key is precisely the failure the cardinality line exists to catch.
KEYDIMS = ("seed", "gap", "arm", "gate", "ft", "piles", "layfrom", "kind", "rule", "ch", "y")
RUNDIMS = ("seed", "gap", "arm", "gate", "ft", "piles", "layfrom")
NAN = float("nan")


def _ints(d):
    for k in INTS:
        if k in d:
            try:
                d[k] = int(d[k])
            except (TypeError, ValueError):
                pass


def normalise(d, ch0="B"):
    """Fill the keys an old or partial row lacks, so every row answers `key()`.

    **An old row's `arm=` is the gate**, not the arm (the 2026-09-21 bug the
    plan's 0b.1 fixes), so it moves to `gate` and the arm is written `?`
    rather than guessed: a wrong arm label is worse than an unknown one.
    """
    old = "gate" not in d
    if old:
        d["gate"] = d.get("arm", "?")
        d["arm"] = "?"
    d["old"] = old
    for k, v in (("ft", "-"), ("piles", "-"), ("layfrom", "-"), ("ch", ch0), ("kind", "live"), ("rule", "-"), ("hand", 0)):
        d.setdefault(k, v)
    d.setdefault("y", None)
    if "prof" in d:
        d.setdefault("x0", 0)
        d.setdefault("x1", d["x0"] + len(d["prof"]) - 1)
        d.setdefault("nest", d["x0"])
        d.setdefault("target", d["x1"])
    return d


def parse(lines):
    """`BTRAIL` rows as dicts. Everything else in the log is ignored.

    An `ATRAIL` row is read as a `BTRAIL` row whose channel defaults to A, in
    case the harness spells channel A that way rather than `ch=A`.
    """
    rows = []
    for line in lines:
        if line.startswith("BTRAIL "):
            ch0 = "B"
        elif line.startswith("ATRAIL "):
            ch0 = "A"
        else:
            continue
        d = dict(KV.findall(line))
        if "prof" not in d:
            continue
        d["prof"] = [int(v) for v in d["prof"].split(",") if v != ""]
        _ints(d)
        rows.append(normalise(d, ch0))
    return rows


def parse_shaft(lines):
    """`BSHAFT` rows: column maxima over xlo..=xhi, from y0 downward.

    **A BSHAFT row does not carry `gap`** (nor `piles`, `layfrom`, `stop`), so
    a log sweeping two gaps put both gaps' shafts under one key -- the
    cardinality line caught it on the 2026-09-29 shadow log, 16 rows sharing a
    (key, frame). The harness prints a sample's BTRAIL rows just before its
    BSHAFT row, so a row missing those keys takes them from the last BTRAIL row
    of the same (seed, arm, gate, ft, frame); the cardinality line still says
    if that did not separate them.
    """
    rows = []
    last = {}
    for line in lines:
        if line.startswith("BTRAIL "):
            d = dict(KV.findall(line))
            last[tuple(d.get(k) for k in ("seed", "arm", "gate", "ft", "frame"))] = \
                {k: d[k] for k in ("gap", "piles", "layfrom", "stop") if k in d}
            continue
        if not line.startswith("BSHAFT "):
            continue
        d = dict(KV.findall(line))
        if "prof" not in d:
            continue
        for k, v in last.get(tuple(d.get(k) for k in ("seed", "arm", "gate", "ft", "frame")), {}).items():
            d.setdefault(k, v)
        d["prof"] = [int(v) for v in d["prof"].split(",") if v != ""]
        _ints(d)
        normalise(d)
        rows.append(d)
    return rows


def key(r):
    return tuple(r.get(k) for k in KEYDIMS)


def runkey(r):
    return tuple(r.get(k) for k in RUNDIMS)


def varying(keys):
    """The dimensions that take more than one value -- the ones a label needs."""
    return {d for i, d in enumerate(KEYDIMS) if len({k[i] for k in keys}) > 1}


def label(k, vary):
    s = " ".join(f"{d}={v}" for d, v in zip(KEYDIMS, k) if d in vary)
    return s or "(one key)"


def ramp_t(t):
    """Dark -> bright on t in [0, 1]; see `ramp`."""
    t = min(1.0, max(0.0, t))
    stops = [(0.0, (20, 30, 80)), (0.35, (40, 110, 190)), (0.7, (240, 150, 40)), (1.0, (255, 250, 235))]
    for i in range(len(stops) - 1):
        t0, c0 = stops[i]
        t1, c1 = stops[i + 1]
        if t <= t1:
            f = 0.0 if t1 == t0 else (t - t0) / (t1 - t0)
            return tuple(int(c0[j] + (c1[j] - c0[j]) * f) for j in range(3))
    return stops[-1][1]


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
    return ramp_t(t)


def logramp(v):
    """The FIXED scale: one brightness is one B in every panel and every run."""
    if v is None:
        return (40, 40, 40)
    if v <= 0:
        return (14, 14, 18)
    return ramp_t(math.log1p(v / 256) / LOG_TOP)


def difframp(d):
    """Signed, on the same log scale: orange where live is above expected,
    blue where it is below, dark where they agree."""
    if d is None:
        return (40, 40, 40)
    t = min(1.0, math.log1p(abs(d) / 256) / LOG_TOP)
    if t <= 0:
        return (14, 14, 18)
    hi = (255, 170, 40) if d > 0 else (60, 150, 255)
    return tuple(int(14 + (hi[j] - 14) * t ** 0.6) for j in range(3))


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


# --- the shape of one row, in distance toward the food -----------------------
#
# Every number below is taken in u = (x - nest) * toward, toward = the sign of
# (target - nest), so a mirrored bed (food toward -x, `onetrail reverse=on`)
# reads the same. A cell outside the row's x0..x1 is None -- NOT zero: an old
# row starts at the door, and "the west side is dark" is a claim it cannot make.


def toward(r):
    return 1 if r["target"] >= r["nest"] else -1


def cell(r, x):
    i = x - r["x0"]
    return r["prof"][i] if 0 <= i < len(r["prof"]) else None


def at_u(r, u):
    return cell(r, r["nest"] + u * toward(r))


def route(r):
    """B at u = 0..=span, the door to the pile inclusive."""
    return [at_u(r, u) for u in range(abs(r["target"] - r["nest"]) + 1)]


def slope(vals):
    """OLS slope of ln(1+B) on u, cells toward the food; every route cell counts,
    dark ones included, as in `onetrail`'s `STREAM` line."""
    pts = [(u, math.log1p(v)) for u, v in enumerate(vals) if v is not None]
    if len(pts) < 3:
        return NAN
    mu = sum(p[0] for p in pts) / len(pts)
    ml = sum(p[1] for p in pts) / len(pts)
    sxx = sum((p[0] - mu) ** 2 for p in pts)
    sxy = sum((p[0] - mu) * (p[1] - ml) for p in pts)
    return sxy / sxx if sxx > 0 else NAN


def rising(vals, thresh):
    """(increasing, lit) over adjacent pairs both >= thresh. A tie is not a rise.

    Below `thresh` (TRAIL_HALF/4 by default) the chooser's presence reads under
    0.2 and a step is rounding, not something an ant could climb."""
    up = lit = 0
    for a, b in zip(vals, vals[1:]):
        if a is None or b is None or a < thresh or b < thresh:
            continue
        lit += 1
        up += b > a
    return up, lit


def window(r, u):
    """Mean B over u-3..=u+3 (both sides: the door and the pile are places),
    and how many of the 7 cells the row actually has."""
    vs = [at_u(r, u + d) for d in range(-3, 4)]
    vs = [v for v in vs if v is not None]
    return (sum(vs) / len(vs) if vs else NAN), len(vs)


def door(r):
    """(g2, g6): the E-W read an empty ant leaving the door makes, food side
    against the other, at the chooser's reach and at the sensor point. None
    where the row does not reach far enough west to say."""
    f1, f2, a1, a2 = at_u(r, 1), at_u(r, 2), at_u(r, -1), at_u(r, -2)
    g2 = None
    if None not in (f1, f2, a1, a2):
        bf, ba = max(f1, f2), max(a1, a2)
        g2 = (bf - ba) / (bf + ba + TRAIL_HALF)
    f6, a6 = at_u(r, 6), at_u(r, -6)
    g6 = None if None in (f6, a6) else (f6 - a6) / (f6 + a6 + TRAIL_HALF)
    return g2, g6


def shape(rows, thresh):
    """The era summary: mean slope, pooled rising, pile/door ratio of time means."""
    sl = [slope(route(r)) for r in rows]
    sl = [s for s in sl if not math.isnan(s)]
    up = lit = 0
    for r in rows:
        a, b = rising(route(r), thresh)
        up += a
        lit += b
    span = abs(rows[0]["target"] - rows[0]["nest"])
    pw = [window(r, span) for r in rows]
    dw = [window(r, 0) for r in rows]
    pm = sum(p[0] for p in pw) / len(pw)
    dm = sum(p[0] for p in dw) / len(dw)
    return dict(slope=(sum(sl) / len(sl)) if sl else NAN, up=up, lit=lit,
                rising=(up / lit) if lit else NAN, pile=pm, door=dm,
                pile_door=(pm / dm) if dm > 0 else (math.inf if pm > 0 else NAN),
                cells=min(min(p[1] for p in pw), min(p[1] for p in dw)))


def eras(rows):
    """(name, rows) for each era present: hand-laid while the ramp is being
    refreshed, ant-laid after (and throughout, on an arm with no ramp)."""
    out = []
    for era, want in (("hand-laid", True), ("ant-laid", False)):
        sel = sorted((r for r in rows if bool(r.get("hand")) == want), key=lambda r: r["frame"])
        if sel:
            out.append((era, sel))
    return out


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

    A widened row (west of the door to past the pile) gets a thin strip on top
    marking the door (green) and the pile (yellow), since the ends of the row
    are no longer where they are.
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
    r0 = rows[0]
    wide = r0["nest"] != r0["x0"] or r0["target"] != r0["x1"]
    top = 4 if wide else 0
    w = prof_len * cell_w + gutter
    h = len(rows) * cell_h + top
    px = [bytearray((14, 14, 18) * w) for _ in range(h)]
    if wide:
        for x, c in ((r0["nest"], (60, 200, 90)), (r0["target"], (240, 220, 60))):
            xi = x - r0["x0"]
            for y in range(top - 1):
                for xx in range(gutter + xi * cell_w, gutter + (xi + 1) * cell_w):
                    if 0 <= xx < w:
                        px[y][xx * 3 : xx * 3 + 3] = bytes(c)

    for ri, r in enumerate(rows):
        hand = r.get("hand", 0)
        for y in range(top + ri * cell_h, top + (ri + 1) * cell_h):
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
    vary = varying(keys)
    print(f"btrailchart: {out}  {w}x{h}  {len(panels)} panels, top to bottom:")
    for k, rows in zip(keys, panels):
        hand = [r for r in rows if r.get("hand")]
        ant = [r for r in rows if not r.get("hand")]
        hp = max((r["peak"] for r in hand), default=0)
        ap_ = max((r["peak"] for r in ant), default=0)
        # An arm with no hand-laid era has nothing to be a share OF: `n/a`,
        # not the 2,462,600% that dividing by max(1, 0) printed for `self`.
        share = f"{100 * ap_ / hp:4.1f}%" if hp else " n/a"
        print(f"             {label(k, vary):<32}  hand-laid peak {hp:6d}  ant-laid peak {ap_:6d}"
              f"  ({share} of it)   ** each panel on its OWN scale")
    return 0


# --- selection and the cardinality line --------------------------------------


def cardinality(rows, what, keyf=key, dims=KEYDIMS):
    """Print the key's cardinality, per dimension, and whether the sweep is square."""
    by_key = defaultdict(list)
    for r in rows:
        by_key[keyf(r)].append(r)
    frames = sorted({r["frame"] for r in rows})
    print(f"btrailchart: {len(rows)} {what} rows over {len(by_key)} keys on ({','.join(dims)})")
    parts = []
    for i, d in enumerate(dims):
        vals = sorted({k[i] for k in by_key}, key=str)
        shown = vals if len(vals) <= 6 else f"[{vals[0]}..{vals[-1]}]"
        parts.append(f"{d} {len(vals)} {shown}")
    print("             " + "  ".join(parts))
    print(f"             {len(frames)} sample frames")
    dup = sum(n - 1 for n in Counter((keyf(r), r["frame"]) for r in rows).values() if n > 1)
    expect = len(by_key) * len(frames)
    if dup:
        print(f"             ** {dup} rows share a (key, frame) with another: the key is missing a dimension "
              f"the run swept, and anything averaged over it pools two experiments. Last write would win.")
    if expect != len(rows):
        print(f"             ** {len(rows)} rows against {expect} = keys x frames: the sweep is NOT "
              f"square, so some key is short or duplicated. Do not read a picture off this "
              f"until you know which.")
    return by_key


def surface_y(rows, why=None):
    """The surface row per run: trailfollow's `surface`, the row `b_profile`
    reads and the old logs' single row.

    **Structural, never picked by content.** It was first "most rows, ties on
    most B", and trailfollow writes every row of its band equally often, so the
    tie-break decided it: on the lead's 2026-09-29 shadow log that read `hand`
    at y=93, `self` at y=95 and the dark `mute` at y=91 -- three arms of one
    seed on three rows, so every paired comparison across arms compared rows,
    not arms. The band is `surface-3..=surface+1` (examples/trailfollow.rs,
    the BTRAIL block), so a run whose rows are exactly that band has its
    surface one above the lowest. A `surface=` key, if a row carries one, wins;
    anything else falls back to the most rows, ties on most B, and says so.

    Chosen from the run's live B rows where it has any, so a counterfactual or
    replay plane is always read on the SAME row as the live one it is compared
    with.
    """
    runs = defaultdict(list)
    for r in rows:
        runs[runkey(r)].append(r)
    pick = {}
    for rk, rs in runs.items():
        base = [r for r in rs if r["kind"] == "live" and r["ch"] == "B"] or rs
        given = {r["surface"] for r in base if isinstance(r.get("surface"), int)}
        ys = sorted({r["y"] for r in base if r["y"] is not None})
        if len(given) == 1:
            pick[rk] = given.pop()
            rule = "surface="
        elif ys and ys == list(range(ys[-1] - BAND_BELOW - BAND_ABOVE, ys[-1] + 1)):
            pick[rk] = ys[-1] - BAND_BELOW
            rule = "band"
        else:
            n = Counter(r["y"] for r in base)
            mass = Counter()
            for r in base:
                mass[r["y"]] += sum(r["prof"])
            pick[rk] = max(n, key=lambda y: (n[y], mass[y]))
            rule = "most rows" if len(n) == 1 else "most rows, ties on most B -- NOT the trailfollow band"
        if why is not None:
            why[rule] += 1
    return pick


def matches(r, a):
    if a.kind != "any" and r["kind"] != a.kind:
        return False
    if a.ch and r["ch"] != a.ch:
        return False
    if a.rule is not None and r["rule"] != a.rule:
        return False
    for k in ("seed", "gap"):
        v = getattr(a, k)
        if v is not None and r.get(k) != v:
            return False
    for k in ("arm", "gate", "ft", "piles", "layfrom"):
        v = getattr(a, k)
        if v is not None and str(r.get(k)) != v:
            return False
    return True


def select(rows, a):
    sel = [r for r in rows if matches(r, a)]
    if not sel:
        return sel
    if a.y is not None:
        return [r for r in sel if r["y"] == a.y]
    ys = sorted({r["y"] for r in sel}, key=str)
    if len(ys) == 1:
        return sel
    why = Counter()
    pick = surface_y(rows, why)
    picked = Counter(pick[runkey(r)] for r in sel)
    print(f"btrailchart: rows at y in {ys}; reading the surface row per run {dict(picked)} "
          f"(by {', '.join(f'{k} on {v}' for k, v in why.items())} runs) -- pass --y to read another.")
    print(f"             The surface is `b_profile`'s row; the ants' heads walk one BELOW it (surface+1),"
          f" where most B is laid.")
    return [r for r in sel if r["y"] == pick[runkey(r)]]


def fmt(v, spec):
    if v is None or (isinstance(v, float) and math.isnan(v)):
        return "n/a"
    if isinstance(v, float) and math.isinf(v):
        return "inf"
    return format(v, spec)


def median(v):
    v = [x for x in v if x is not None and not (isinstance(x, float) and math.isnan(x))]
    return st.median(v) if v else NAN


# --- readouts ----------------------------------------------------------------


def stats_mode(by_key, a):
    keys = sorted(by_key, key=str)
    vary = varying(keys)
    const = {d: keys[0][i] for i, d in enumerate(KEYDIMS) if d not in vary}
    print(f"             constant: {' '.join(f'{d}={v}' for d, v in const.items())}")
    w = max(24, max(len(label(k, vary)) for k in keys) + 1)
    print(f"\n{'key':<{w}} {'era':<10} {'lit/route':>10} {'worst gap':>9} {'bridged@' + str(a.reach):>10}"
          f" {'slope/cell':>11} {'rising':>7} {'(pairs)':>8} {'pile/door':>10}")
    print("-" * (w + 83))
    groups = defaultdict(list)
    for k in keys:
        for era, sel in eras(by_key[k]):
            stats = [continuity([v or 0 for v in route(r)], a.reach) for r in sel]
            n = len(stats)
            lit = sorted(x[0] for x in stats)[n // 2]
            gap = sorted(x[1] for x in stats)[n // 2]
            br = 100.0 * sum(1 for x in stats if x[2]) / n
            s = shape(sel, a.thresh)
            trunc = "" if s["cells"] == 7 else f" [{s['cells']}/7]"
            print(f"{label(k, vary):<{w}} {era:<10} {lit:>4}/{len(route(sel[0])):<5} {gap:>9} {br:>9.0f}%"
                  f" {fmt(s['slope'], '+.5f'):>11} {fmt(s['rising'], '.3f'):>7} {s['lit']:>8} {fmt(s['pile_door'], '.3f'):>10}{trunc}")
            gk = tuple(v for d, v in zip(KEYDIMS, k) if d != "seed")
            groups[(gk, era)].append(s)
    print(f"\n  medians over sampled frames for continuity; `bridged` = the share of frames with NO dark run")
    print(f"  at least {a.reach} cells long on the route (door to pile). --reach 2 is the chooser's own reach.")
    print(f"  slope: mean over frames of the OLS slope of ln(1+B) per cell toward the food, door to pile.")
    print(f"  rising: pooled share of adjacent pairs both >= {a.thresh} raw that increase toward the food.")
    print(f"  pile/door: time-mean B within 3 cells of the pile over the same at the door; [n/7] = the row")
    print(f"  holds only n of a window's 7 cells (an old row ends at the door and the pile).")
    if len(keys) > 1:
        gvary = {d for d in vary if d != "seed"}
        print(f"\n{'group (over seeds)':<{w}} {'era':<10} {'n':>3} {'rising<0.5':>11} {'rising>0.9':>11} {'slope>0':>8}"
              f" {'med rising':>11} {'med slope':>10} {'med pile/door':>14}")
        for (gk, era), ss in sorted(groups.items(), key=str):
            full = dict(zip([d for d in KEYDIMS if d != "seed"], gk))
            lab = " ".join(f"{d}={full[d]}" for d in KEYDIMS if d in gvary) or "(all)"
            n = len(ss)
            lo = sum(1 for s in ss if not math.isnan(s["rising"]) and s["rising"] < 0.5)
            hi = sum(1 for s in ss if not math.isnan(s["rising"]) and s["rising"] > 0.9)
            up = sum(1 for s in ss if not math.isnan(s["slope"]) and s["slope"] > 0)
            print(f"{lab:<{w}} {era:<10} {n:>3} {f'{lo}/{n}':>11} {f'{hi}/{n}':>11} {f'{up}/{n}':>8}"
                  f" {fmt(median([s['rising'] for s in ss]), '.3f'):>11} {fmt(median([s['slope'] for s in ss]), '+.5f'):>10}"
                  f" {fmt(median([s['pile_door'] for s in ss if not math.isinf(s['pile_door'])]), '.3f'):>14}")
    return 0


def door_mode(by_key, a):
    keys = sorted(by_key, key=str)
    vary = varying(keys)
    w = max(24, max(len(label(k, vary)) for k in keys) + 1)
    print(f"\nthe E-W read at the door: g = (bF - bA)/(bF + bA + {TRAIL_HALF}), bF on the food side")
    print(f"{'key':<{w}} {'era':<10} {'frames':>6} {'med g2':>8} {'mean g2':>8} {'g2>0.1':>7} {'med g6':>8} {'mean g6':>8} {'g6>0.1':>7}")
    pooled = defaultdict(lambda: ([], []))
    for k in keys:
        for era, sel in eras(by_key[k]):
            g = [door(r) for r in sel]
            g2 = [x[0] for x in g if x[0] is not None]
            g6 = [x[1] for x in g if x[1] is not None]
            s2 = f"{sum(v > 0.1 for v in g2) / len(g2):.0%}" if g2 else "n/a"
            s6 = f"{sum(v > 0.1 for v in g6) / len(g6):.0%}" if g6 else "n/a"
            # The mean beside the median because `onetrail`'s `STREAM` line
            # reports means, and the two instruments must agree on its rows.
            m2 = sum(g2) / len(g2) if g2 else NAN
            m6 = sum(g6) / len(g6) if g6 else NAN
            print(f"{label(k, vary):<{w}} {era:<10} {len(sel):>6} {fmt(median(g2), '+.3f'):>8} {fmt(m2, '+.4f'):>8} {s2:>7}"
                  f" {fmt(median(g6), '+.3f'):>8} {fmt(m6, '+.4f'):>8} {s6:>7}")
            gk = tuple(v for d, v in zip(KEYDIMS, k) if d != "seed")
            pooled[(gk, era)][0].extend(g2)
            pooled[(gk, era)][1].extend(g6)
            if a.series:
                for r, (x2, x6) in zip(sel, g):
                    print(f"    frame {r['frame']:>6}  g2 {fmt(x2, '+.3f'):>7}  g6 {fmt(x6, '+.3f'):>7}")
    if any(r["old"] for rs in by_key.values() for r in rs):
        print("  n/a: an old-format row starts AT the door, so it holds nothing west of it -- the read is")
        print("  unanswerable from it, which is not the same as a zero gradient.")
    if len(keys) > 1:
        gvary = {d for d in vary if d != "seed"}
        print("\npooled over seeds (median of every frame):")
        for (gk, era), (g2, g6) in sorted(pooled.items(), key=str):
            full = dict(zip([d for d in KEYDIMS if d != "seed"], gk))
            lab = " ".join(f"{d}={full[d]}" for d in KEYDIMS if d in gvary) or "(all)"
            print(f"  {lab:<{w - 2}} {era:<10} med g2 {fmt(median(g2), '+.3f')}  med g6 {fmt(median(g6), '+.3f')}")
    return 0


def load_expect(a, rows):
    """The expected rows, and where they came from. See the module doc."""
    spec = a.expect
    kind = rule = None
    if Path(spec).is_file():
        src, origin = parse(Path(spec).read_text().splitlines()), spec
    else:
        src, origin = rows, "this log"
        if ":" in spec:
            kind, rule = spec.split(":", 1)
        elif spec != "same":
            kind = spec
    kind = a.expect_kind or kind or ("cf" if any(r["kind"] == "cf" for r in src) else "design")
    rule = a.expect_rule or rule
    cand = [r for r in src if r["kind"] == kind and r["ch"] == (a.ch or "B")]
    rules = sorted({r["rule"] for r in cand})
    if not cand:
        print(f"btrailchart: --expect: no kind={kind} ch={a.ch or 'B'} rows in {origin}")
        return None
    if rule is None:
        if len(rules) != 1:
            print(f"btrailchart: --expect: {len(rules)} rules of kind={kind} in {origin}: {rules} -- name one "
                  f"(--expect-rule, or SPEC kind:rule)")
            return None
        rule = rules[0]
    cand = [r for r in cand if r["rule"] == rule]
    if not cand:
        print(f"btrailchart: --expect: no kind={kind} rule={rule} rows in {origin} (rules there: {rules})")
        return None
    return cand, kind, rule, origin


def pair_expect(by_key, cand, kind):
    """{live key: {frame: expected row}} and the worst frame mismatch used."""
    out = {}
    worst = 0
    if kind == "design":
        pick = surface_y(cand)
        cand = [r for r in cand if r["y"] == pick[runkey(r)]]
        groups = defaultdict(list)
        for r in cand:
            groups[r.get("gap")].append(r)
    else:
        groups = defaultdict(list)
        for r in cand:
            groups[(runkey(r), r["y"])].append(r)
    for k, rs in by_key.items():
        r0 = rs[0]
        if kind == "design":
            g = groups.get(r0.get("gap")) or (next(iter(groups.values())) if len(groups) == 1 else None)
        else:
            g = groups.get((runkey(r0), r0["y"]))
        if not g:
            continue
        fr = {r["frame"]: r for r in g}
        fs = sorted(fr)
        m = {}
        for r in rs:
            f = r["frame"]
            best = fr.get(f) or fr[min(fs, key=lambda x: abs(x - f))]
            worst = max(worst, abs(best["frame"] - f))
            m[f] = best
        out[k] = m
    return out, worst


def expect_mode(by_key, rows, a):
    got = load_expect(a, rows)
    if got is None:
        return 1
    cand, kind, rule, origin = got
    pairs, worst = pair_expect(by_key, cand, kind)
    keys = sorted((k for k in by_key if k in pairs), key=str)
    missing = [k for k in by_key if k not in pairs]
    print(f"btrailchart: expected = kind={kind} rule={rule} from {origin}; {len(keys)} of {len(by_key)} live keys paired"
          f"{'' if not missing else f' -- {len(missing)} have no expected rows and are left out'}")
    if worst:
        print(f"             ** frames do not coincide: paired on the nearest expected frame, up to {worst} frames apart")
    if not keys:
        return 1
    vary = varying(keys)
    w = max(24, max(len(label(k, vary)) for k in keys) + 1)
    print(f"\n{'key':<{w}} {'era':<10} {'slope live':>11} {'expected':>9} {'rising live':>12} {'expected':>9}"
          f" {'pile/door live':>15} {'expected':>9} {'live>=exp':>10}")
    series = {}
    for k in keys:
        for era, sel in eras(by_key[k]):
            ex = [pairs[k][r["frame"]] for r in sel]
            sl, se = shape(sel, a.thresh), shape(ex, a.thresh)
            per = []
            for r, e in zip(sel, ex):
                ul, ll = rising(route(r), a.thresh)
                ue, le = rising(route(e), a.thresh)
                per.append((r["frame"], ul / ll if ll else NAN, ue / le if le else NAN, slope(route(r)), slope(route(e))))
            ok = [p for p in per if not math.isnan(p[1]) and not math.isnan(p[2])]
            ge = f"{sum(p[1] >= p[2] for p in ok)}/{len(ok)}"
            print(f"{label(k, vary):<{w}} {era:<10} {fmt(sl['slope'], '+.5f'):>11} {fmt(se['slope'], '+.5f'):>9}"
                  f" {fmt(sl['rising'], '.3f'):>12} {fmt(se['rising'], '.3f'):>9}"
                  f" {fmt(sl['pile_door'], '.3f'):>15} {fmt(se['pile_door'], '.3f'):>9} {ge:>10}")
            series.setdefault(k, []).extend(per)
            if a.series:
                for f, rl, re_, sl_, se_ in per:
                    print(f"    frame {f:>6}  rising {fmt(rl, '.3f')} vs {fmt(re_, '.3f')}   slope {fmt(sl_, '+.5f')} vs {fmt(se_, '+.5f')}")
    draw_expect(by_key, pairs, keys, vary, series, a.out)
    return 0


def draw_expect(by_key, pairs, keys, vary, series, out, cell_w=4, cell_h=4, gutter=22, sep=10, tw=80, slope_top=0.05):
    """Per key, one band: live | expected | live - expected | rising(t) | slope(t).

    Every heatmap is on the FIXED log scale and in distance from the door, so a
    colony and a design model with different x ranges line up cell for cell,
    and brightness means the same B everywhere. The two time-series panels share
    the heatmaps' time axis: white is live, orange expected; rising spans 0..1,
    slope +/-`slope_top` per cell with the grey rule at zero.
    """
    us = set()
    for k in keys:
        for r in by_key[k] + list(pairs[k].values()):
            t = toward(r)
            us.update(((r["x0"] - r["nest"]) * t, (r["x1"] - r["nest"]) * t))
    umin, umax = min(us), max(us)
    ncol = umax - umin + 1
    pw = ncol * cell_w
    top = 4
    bands = []
    for k in keys:
        rs = sorted(by_key[k], key=lambda r: r["frame"])
        bands.append((k, rs))
    W = gutter + 3 * (pw + 6) + 2 * (tw + 6)
    H = sum(top + len(rs) * cell_h + sep for _, rs in bands)
    px = [bytearray((14, 14, 18) * W) for _ in range(H)]

    def put(x, y, c):
        if 0 <= x < W and 0 <= y < H:
            px[y][x * 3 : x * 3 + 3] = bytes(c)

    y0 = 0
    for k, rs in bands:
        span = abs(rs[0]["target"] - rs[0]["nest"])
        for p in range(3):
            base = gutter + p * (pw + 6)
            for u, c in ((0, (60, 200, 90)), (span, (240, 220, 60))):
                for x in range(base + (u - umin) * cell_w, base + (u - umin + 1) * cell_w):
                    for y in range(y0, y0 + top - 1):
                        put(x, y, c)
        ser = {f: (rl, re_, sl, se) for f, rl, re_, sl, se in series.get(k, [])}
        for ri, r in enumerate(rs):
            e = pairs[k][r["frame"]]
            yy = y0 + top + ri * cell_h
            band = (200, 60, 60) if r.get("hand") else (45, 45, 55)
            for y in range(yy, yy + cell_h):
                for x in range(gutter - 6):
                    put(x, y, band)
            for u in range(umin, umax + 1):
                lv, ev = at_u(r, u), at_u(e, u)
                cols = (logramp(lv), logramp(ev), difframp(None if lv is None or ev is None else lv - ev))
                for p, c in enumerate(cols):
                    x0 = gutter + p * (pw + 6) + (u - umin) * cell_w
                    for y in range(yy, yy + cell_h):
                        for x in range(x0, x0 + cell_w):
                            put(x, y, c)
            rl, re_, sl, se = ser.get(r["frame"], (NAN,) * 4)
            for p, (vals, lo, hi) in enumerate((((rl, re_), 0.0, 1.0), ((sl, se), -slope_top, slope_top))):
                base = gutter + 3 * (pw + 6) + p * (tw + 6)
                zero = base + int((0 - lo) / (hi - lo) * (tw - 1))
                for y in range(yy, yy + cell_h):
                    put(base, y, (45, 45, 55))
                    put(base + tw - 1, y, (45, 45, 55))
                    if lo < 0:
                        put(zero, y, (70, 70, 80))
                for v, c in zip(vals, ((250, 250, 250), (255, 150, 40))):
                    if math.isnan(v):
                        continue
                    xv = base + int((min(hi, max(lo, v)) - lo) / (hi - lo) * (tw - 1))
                    for y in range(yy + 1, yy + cell_h - 1):
                        for x in range(xv - 1, xv + 2):
                            put(x, y, c)
        y0 += top + len(rs) * cell_h + sep
    write_png(out, px, W, H)
    print(f"\nbtrailchart: {out}  {W}x{H}  {len(bands)} bands, top to bottom: {', '.join(label(k, vary) for k, _ in bands)}")
    print(f"             panels: live | expected | live - expected (orange above, blue below), u = {umin}..{umax}"
          f" cells from the door; green = door, yellow = pile")
    print(f"             then rising 0..1 and slope +/-{slope_top}/cell against time (white live, orange expected)")
    print(f"             ONE fixed scale: brightness = ln(1+v/256)/ln(1+4*DEPOSIT/256), full at {4 * DEPOSIT} raw")


def shaft_mode(shaft, a):
    sel = [r for r in shaft if matches(r, a)]
    if not sel:
        print("btrailchart: no BSHAFT rows matched -- was the run given the widened `btrail`?")
        return 1
    dims = tuple(d for d in KEYDIMS if d != "y")
    kf = lambda r: tuple(r.get(d) for d in dims)
    by_key = cardinality(sel, "BSHAFT", kf, dims)
    keys = sorted(by_key, key=str)
    vary = {d for i, d in enumerate(dims) if len({k[i] for k in keys}) > 1}
    print("\ntime-mean column maximum of B by depth below y0 (every 2nd row), per key and era:")
    for k in keys:
        lab = " ".join(f"{d}={v}" for d, v in zip(dims, k) if d in vary) or "(one key)"
        for era, rs in eras(by_key[k]):
            depth = max(len(r["prof"]) for r in rs)
            means = [sum(r["prof"][i] for r in rs if i < len(r["prof"])) / len(rs) for i in range(depth)]
            print(f"  {lab:<28} {era:<10} y0={rs[0].get('y0')} x={rs[0].get('xlo')}..{rs[0].get('xhi')}  "
                  + " ".join(f"{m:.0f}" for m in means[::2]))
    rs = sorted(by_key[keys[0]], key=lambda r: r["frame"])
    depth = max(len(r["prof"]) for r in rs)
    cw, chh, gut = 6, 4, 22
    W, H = gut + depth * cw, len(rs) * chh
    px = [bytearray((14, 14, 18) * W) for _ in range(H)]
    for ri, r in enumerate(rs):
        for y in range(ri * chh, (ri + 1) * chh):
            band = (200, 60, 60) if r.get("hand") else (45, 45, 55)
            for x in range(gut - 6):
                px[y][x * 3 : x * 3 + 3] = bytes(band)
            for i, v in enumerate(r["prof"]):
                for x in range(gut + i * cw, gut + (i + 1) * cw):
                    px[y][x * 3 : x * 3 + 3] = bytes(logramp(v))
    write_png(a.out, px, W, H)
    print(f"\nbtrailchart: {a.out}  {W}x{H}  the first key only, time down, depth across (left = y0), fixed log scale")
    return 0


def main():
    ap = argparse.ArgumentParser(description="Read trailfollow's BTRAIL/BSHAFT rows; see the module doc.")
    ap.add_argument("log", nargs="?")
    ap.add_argument("--out", default="/tmp/btrail.png")
    ap.add_argument("--seed", type=int)
    ap.add_argument("--gap", type=int)
    ap.add_argument("--arm")
    ap.add_argument("--gate")
    ap.add_argument("--ft")
    ap.add_argument("--piles")
    ap.add_argument("--layfrom")
    ap.add_argument("--kind", default="live", help="live|replay|cf|design|any (default live)")
    ap.add_argument("--ch", default="B", help="A or B (default B)")
    ap.add_argument("--rule", help="cf/design rule (brain, gate, odo32, step32, sq..); default any")
    ap.add_argument("--y", type=int, help="the row to read (default: the surface row -- see the module doc)")
    ap.add_argument("--since", type=int, default=0, help="only frames > SINCE")
    ap.add_argument("--late", action="store_true",
                    help="only the second half of each key's frames -- the standing state, as onetrail's STREAM line")
    ap.add_argument("--stats", action="store_true",
                    help="per key and era: continuity, slope, rising share, pile/door; then counts over seeds")
    ap.add_argument("--thresh", type=int, default=TRAIL_HALF // 4,
                    help=f"a pair counts toward `rising` only if both cells hold at least this (default {TRAIL_HALF // 4} raw = TRAIL_HALF/4)")
    ap.add_argument("--reach", type=int, default=6,
                    help="a dark run shorter than this is not a break (default 6, the old sensor offset; "
                         "the chooser's own reach is 2 -- trail_presence reads head+d and head+2d)")
    ap.add_argument("--door", action="store_true", help="the E-W gradient at the door over time, reach 2 and 6")
    ap.add_argument("--expect", metavar="SPEC",
                    help="a file of expected rows, or a selector on this log: cf:<rule>, design, replay, same")
    ap.add_argument("--expect-kind")
    ap.add_argument("--expect-rule")
    ap.add_argument("--series", action="store_true", help="with --door/--expect: print every frame")
    ap.add_argument("--shaft", action="store_true", help="BSHAFT rows: B by depth down the shaft over time")
    ap.add_argument("--grid", action="store_true",
                    help="every key as a panel in one image -- the distribution, not one draw")
    ap.add_argument("--after", action="store_true",
                    help="rescale to the brightest cell AFTER the hand-laid ramp stops -- see draw()")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()

    if a.selftest:
        return selftest()
    if not a.log:
        ap.error("a log is required (or --selftest)")

    lines = Path(a.log).read_text().splitlines()
    if a.shaft:
        return shaft_mode(parse_shaft(lines), a)
    rows = parse(lines)
    if not rows:
        print(f"btrailchart: no BTRAIL rows in {a.log} -- was the run given `btrail`?")
        return 1

    # **The cardinality check, before anything is drawn.** See the module doc.
    # Once over the whole log, so what was NOT selected is on the screen too.
    cardinality(rows, "BTRAIL (whole log)")
    nold = sum(r["old"] for r in rows)
    if nold:
        print(f"             {nold} old-format rows: arm unknown (their arm= carried the gate), ch=B kind=live, "
              f"the row starts at the door and ends at the pile")
    sel = select(rows, a)
    if sel and (a.since or a.late):
        last = defaultdict(int)
        for r in sel:
            last[key(r)] = max(last[key(r)], r["frame"])
        sel = [r for r in sel if r["frame"] > a.since and (not a.late or 2 * r["frame"] > last[key(r)])]
    if not sel:
        kinds = sorted({(r["kind"], r["ch"], r["rule"]) for r in rows})
        print(f"btrailchart: nothing matched the selection; (kind, ch, rule) in the log: {kinds}")
        return 1
    by_key = cardinality(sel, "selected")

    if a.expect:
        return expect_mode(by_key, rows, a)
    if a.door:
        return door_mode(by_key, a)
    if a.stats:
        return stats_mode(by_key, a)

    if a.grid:
        # **Every seed in one image, because one seed is a draw from a wide
        # distribution.** Measured 2026-09-21: over four seeds of one arm the
        # ants' own peak ran 18%, 27%, 32% and 78% of the hand-laid ramp, and
        # the route was bare in the last 6,000 frames on one seed and lit on 70
        # of 91 cells on another. A single panel would have been read as the
        # result. `CLAUDE.md`: an outcome is a distribution, not a binary.
        keys = sorted(by_key, key=str)
        panels = [by_key[k] for k in keys]
        return grid(panels, keys, a.out)

    if len(by_key) != 1:
        vary = varying(list(by_key))
        print(f"btrailchart: {len(by_key)} keys selected -- narrow with --seed/--arm/--gap/--rule/..., or "
              f"they will be drawn on top of each other:\n  " + "\n  ".join(label(x, vary) for x in sorted(by_key, key=str)))
        return 1

    stop = sel[0].get("stop", 0)
    w, h, vmax, n = draw(sel, a.out, after_only=a.after)
    print(f"btrailchart: {a.out}  {w}x{h}  row {n} cells  peak channel {sel[0]['ch']} {vmax}"
          f"{' (AFTER the handover only)' if a.after else ''}  stop={stop}")
    print(f"             left band RED while the hand-laid ramp is refreshed, GREY after.")
    return 0


def _row(seed, frame, prof, nest, target, x0, **kw):
    """A new-format BTRAIL line, for the controls."""
    d = dict(seed=seed, arm="self", gate="shipped", ft="off", piles="east", stop=0, gap=abs(target - nest),
             layfrom="nest", nest=nest, target=target, ch="B", kind="live", rule="-", y=95, x0=x0,
             x1=x0 + len(prof) - 1, frame=frame, hand=0)
    d.update(kw)
    return ("BTRAIL " + " ".join(f"{k}={v}" for k, v in d.items())
            + f" cells={sum(v > 0 for v in prof)} peak={max(prof)} prof=" + ",".join(str(v) for v in prof))


def selftest():
    """The positive control: a bed whose answer is known, drawn and read back.

    `CLAUDE.md` -- a guard that cannot go red is blind, not strong. This builds
    synthetic runs whose answers are known -- a trail and none, a ramp that
    rises toward the food and one that falls, noise, a mirrored bed, an old row
    -- and asserts every readout separates them. It is the check that would
    have caught a chart script reporting a clean empty plane because its regex
    missed `prof=`.
    """
    try:
        _selftest()
    except AssertionError as e:
        print(f"btrailchart selftest: FAIL -- {e}")
        return 1
    return 0


def _selftest():
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
    assert logramp(300) != logramp(0) and logramp(DEPOSIT) != logramp(4 * DEPOSIT), "the fixed log scale saturates too early"
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

    # Old-format rows default as the plan says: the arm= was the gate.
    o = rows[0]
    assert (o["gate"], o["arm"], o["ch"], o["kind"], o["rule"], o["y"], o["nest"], o["target"]) == \
        ("shipped", "?", "B", "live", "-", None, 48, 52), f"an old row is not defaulted as the plan says: {o}"
    assert door(o) == (None, None), "an old row starts AT the door: its gradient is unanswerable, not zero"

    # New rows, every key, and a switch echo carrying its own `=`.
    nest, target, x0 = 50, 140, 30
    span = target - nest
    width = span + 41
    ramp_p = [0] * 20 + [int(300 * math.exp(u * math.log(60) / span)) for u in range(span + 1)] + [0] * 20
    peak_p = [0] * 20 + [int(18000 * math.exp(-u * math.log(60) / span)) for u in range(span + 1)] + [0] * 20
    rng = random.Random(7)
    noise_p = [0] * 20 + [rng.randint(300, 20000) for _ in range(span + 1)] + [0] * 20
    assert len(ramp_p) == width
    r = parse([_row(1, 500, ramp_p, nest, target, x0, ft="lay,t=32")])[0]
    assert r["ft"] == "lay,t=32" and r["kind"] == "live" and r["y"] == 95 and r["old"] is False, r
    assert route(r)[0] == ramp_p[20] and route(r)[-1] == ramp_p[-21], "the route is not the door-to-pile slice"
    up, nl = rising(route(r), TRAIL_HALF // 4)
    assert nl > 80 and up / nl > 0.9 and slope(route(r)) > 0, f"a ramp toward the food must rise: {up}/{nl} {slope(route(r))}"
    assert continuity([v or 0 for v in route(r)], 6)[2], "dark cells WEST of the door are breaking the route's continuity"
    p = parse([_row(1, 500, peak_p, nest, target, x0)])[0]
    up, nl = rising(route(p), TRAIL_HALF // 4)
    assert up / nl < 0.5 and slope(route(p)) < 0, f"a nest-peaked trail must fall: {up}/{nl} {slope(route(p))}"
    nrows = parse([_row(1, f, [0] * 20 + [rng.randint(300, 20000) for _ in range(span + 1)] + [0] * 20, nest, target, x0)
                   for f in range(500, 5000, 500)] + [_row(1, 500, noise_p, nest, target, x0)])
    s = shape(nrows, TRAIL_HALF // 4)
    assert 0.4 < s["rising"] < 0.6, f"iid noise must read rising ~0.5, got {s['rising']:.3f}"
    assert shape([r], 256)["pile_door"] > 1 > shape([p], 256)["pile_door"], "pile/door does not separate a ramp from a peak"
    # The threshold is doing work: a ramp entirely under it has no lit pairs.
    dim = parse([_row(1, 500, [v // 100 for v in ramp_p], nest, target, x0)])[0]
    assert rising(route(dim), 256)[1] == 0, "pairs under the threshold are being counted"
    # A flat lit trail has pairs and no rise: a tie is not a climb.
    flat = parse([_row(1, 500, [0] * 20 + [2000] * (span + 1) + [0] * 20, nest, target, x0)])[0]
    assert rising(route(flat), 256) == (0, span), f"a flat trail must read 0 of {span} rising: {rising(route(flat), 256)}"

    # A mirrored bed (food toward -x) reads identically: every number is in u.
    m = parse([_row(1, 500, ramp_p[::-1], x0 + width - 1 - 20, x0 + 20, x0)])[0]
    assert toward(m) == -1 and route(m) == route(r), "the mirrored route is not the same route"
    assert abs(slope(route(m)) - slope(route(r))) < 1e-12 and rising(route(m), 256) == rising(route(r), 256)

    # The door read: food side brighter reads positive, and reach 6 is its own cell pair.
    dp = [0] * width
    for u in range(1, 7):
        dp[20 + u] = 6000
    for u in range(-6, 0):
        dp[20 + u] = 1000
    g2, g6 = door(parse([_row(1, 500, dp, nest, target, x0)])[0])
    assert g2 is not None and g2 > 0.5 and g6 > 0.5, f"a brighter food side must read positive: {g2} {g6}"
    g2m, _ = door(parse([_row(1, 500, dp[::-1], x0 + width - 1 - 20, x0 + 20, x0)])[0])
    assert abs(g2m - g2) < 1e-12, "the door read is not mirror-invariant"

    # The surface row, from the band's shape and never from its B: three arms
    # whose B peaks on three different rows (and one with none) are all read
    # on trailfollow's `surface`, one above the band's lowest row -- the
    # 2026-09-29 shadow log read them on 93, 95 and 91 -- and a cf plane is
    # read on the LIVE plane's row even where its own B would pick another.
    band = []
    for arm, hot in (("hand", 93), ("self", 95), ("mute", None)):
        for y in range(91, 96):
            v = ramp_p if hot is not None else [0] * width
            band.append(_row(1, 500, [c * 5 for c in v] if y == hot else v, nest, target, x0, y=y, arm=arm))
    cf = [_row(1, 500, [v * 9 if y == 91 else v for v in ramp_p], nest, target, x0, y=y, kind="cf", rule="odo32", arm="self")
          for y in range(91, 96)]
    pick = surface_y(parse(band + cf))
    assert len(pick) == 3 and set(pick.values()) == {94}, f"every arm must be read on the band's surface row, 94: {pick}"
    # ...a row that names its surface wins, and a set of rows that is not the
    # band falls back to the most B, as before.
    named = parse([_row(1, 500, ramp_p, nest, target, x0, y=y, surface=92) for y in range(91, 96)])
    assert set(surface_y(named).values()) == {92}, "a row's own surface= must win over the band's shape"
    odd = [_row(1, 500, ramp_p, nest, target, x0, y=y) for y in (93, 94, 95, 96)]
    odd[1] = _row(1, 500, [v * 3 for v in ramp_p], nest, target, x0, y=94)
    assert set(surface_y(parse(odd)).values()) == {94}, "off the band, the fallback is the row with the most live B"

    # --expect pairing: a cf plane identical to live pairs frame for frame and differs by nothing.
    live = parse([_row(2, f, ramp_p, nest, target, x0) for f in (500, 1000)])
    cfr = parse([_row(2, f, ramp_p, nest, target, x0, kind="cf", rule="odo32") for f in (500, 1000)])
    bk = defaultdict(list)
    for x in live:
        bk[key(x)].append(x)
    pairs, worst = pair_expect(bk, cfr, "cf")
    assert len(pairs) == 1 and worst == 0, f"a cf plane on the same run is not paired: {len(pairs)} {worst}"
    k = next(iter(pairs))
    assert all(route(pairs[k][x["frame"]]) == route(x) for x in live)
    # ...and a design row from another world pairs on gap, in distance from its own door.
    des = parse([_row(0, 500, ramp_p[5:-5], nest - 25, target - 25, x0 - 20, arm="design", gate="-", kind="design", rule="odo32", y=32)])
    pairs, _ = pair_expect(bk, des, "design")
    assert route(pairs[k][500]) == route(live[0]), "a design row is not aligned on distance from the door"

    # BSHAFT rows parse, with the same defaults.
    sh = parse_shaft(["BSHAFT seed=1 arm=self gate=shipped ft=off ch=B kind=live frame=600 xlo=48 xhi=50 y0=96 prof=900,500,100,0"])
    assert len(sh) == 1 and sh[0]["prof"] == [900, 500, 100, 0] and sh[0]["y0"] == 96 and sh[0]["kind"] == "live"
    # ...and two gaps of one seed and arm, which BSHAFT does not name, stay apart.
    lines = []
    for g in (90, 140):
        lines.append(_row(1, 600, ramp_p, nest, nest + g, x0, ft="off"))
        lines.append(f"BSHAFT seed=1 arm=self gate=shipped ft=off ch=B kind=live frame=600 xlo=48 xhi=50 y0=96 prof={g},1")
    sh = parse_shaft(lines)
    assert sorted(r["gap"] for r in sh) == [90, 140], f"a BSHAFT row must take its gap from its run's BTRAIL rows: {sh}"

    print("btrailchart selftest: ok -- parse, handover flag, ramp separation, faint-cell visibility, png writer,")
    print("                      continuity (a spike WINS on peak and correctly fails as a trail),")
    print("                      old rows (gate from arm=, no door read), new keys, route slice,")
    print("                      ramp rising > 0.9 / slope > 0, nest peak < 0.5 / < 0, noise ~0.5, the threshold,")
    print("                      mirror invariance, the door gradient, the surface row, cf and design pairing, BSHAFT")


if __name__ == "__main__":
    sys.exit(main())
