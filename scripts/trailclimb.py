#!/usr/bin/env python3
"""Do the ants climb the food trail -- and is the trace we read that from the
draw the ant actually made?

Stage 0 of `Reports/food-trail-plan-2026-09-29.md`. `trailfollow decisioncsv
dwide` appends, per decision, what the chooser could read of trail B heading by
heading (`bn<d>`/`bf<d>` one and two cells along heading d, `b6<d>` at the
six-cell sensor point), the options it drew from (`opts`, a bitmask in `DIRS`
order: 0=E 1=NE 2=N 3=NW 4=W 5=SW 6=S 7=SE, y down), the score each was drawn
with (`sc<d>`), the looseness `k`, and the heading drawn (`chose`). This reads
those columns and answers three questions, in order:

  1. SELF-CONSISTENCY -- the positive control, and it comes first because
     everything after it is meaningless without it. `choose_weighted` draws
     option o with P(o) = (k + max(s_o, 0))^2 / sum over the options. Recompute
     P from the logged scores and check the realised draws against it: per leg
     class, the count of picks of the highest-P option against its expected
     count (sum of its P) and a 99% interval -- judged at 99% over the whole
     family of lines, since one false FAIL in many would read as the trace
     failing -- and a reliability table -- every
     (decision, option) pair binned by P(o), mean P against how often that
     option was picked. **Binned on each option's P, not the chosen one's**:
     binning on P(chosen) conditions on the outcome and has no expected value
     to compare with. If this FAILS, the scores in the trace are not the scores
     that were drawn, and no climbing number below may be quoted.
  2. CLIMB -- among the 'level' options (dx != 0: headings 0,1,3,4,5,7), with
     at least two present and b(d) = max(bn_d, bf_d), the cells
     `trail_presence` reads: the share of decisions taking the unique
     highest-b option (realised, expected from the logged P, and chance =
     1/options), and the uphill share -- picks whose b beats the mean of the
     other level options (realised, expected, chance). By leg class x zone x
     arm, then paired per seed across arms. **Chance is a uniform draw, not an
     ant without the trail**: the trace does not hold the scores minus the trail
     term, so the no-follow control is the paired `mute` or `self` arm, not
     that column.
  3. DOOR DEPARTURES -- every empty ant leaving the nest (at_nest >= 0.5 ->
     < 0.5 with leg empty): the E-W read at that row, grad2 = (b(E) - b(W)) /
     (b(E) + b(W) + 1024) from bn/bf and grad6 from b60/b64; the side it took
     (the sign of its x displacement once it is 30 cells from where it left,
     within 2,000 frames, else `none`); its energy class; and the age of the
     colony's last booked return (trip_load 1 -> 0 at the nest, any ant, strictly
     before the departure). Share east by gradient bin and by return-age bin
     (0-270, 270-774, 774-2000, >2000 frames), per arm: P0.8 and P0.9 of the
     plan are read here.

Then LAID: where trail B was put down, by zone (`emit_b_laid` at `dep_x`; on
old traces, a count of moved rows by a laden ant, the proxy
`/home/user/runs/trailq/departures.py` used).

Classes, of a decision the chooser made (chose != '-', opts != 0):
  fed-driven  reads_b, leg empty, energy_j >= 200, drive finite and > 0
  fed         reads_b, leg empty, energy_j >= 200, no drive (not in the plan's
              list; kept apart rather than folded into fed-driven)
  hungry      reads_b, leg empty, energy_j < 200
  lunch       reads_b, leg laden (a packed-lunch carrier)
  spoil       reads_b, hauling spoil (not in the plan's list; kept out of
              `laden`, which is the class that does NOT read B)
  laden       not reads_b (a true load: reads trail A)
  worker      the nest-worker caste (id % --caste == 0), whatever it carries
Zones, by the head before the move (x, y) against nest_x and gap: west (x <
nest-3), door (|x-nest| <= 3), road (nest+3 < x < nest+gap-10), pile (|x -
(nest+gap)| <= 10), far (anything else); and below ground (y > surface + 1),
shaft within 3 columns of the door and under elsewhere. The surface is --surface,
or per key the modal y of rows with |x - nest| > 10 and leg empty.

A decision whose options include a trunk crossing (`cross=1`) is left out of 1
and 2 and counted: the crossing is scored at the current heading's slot, so
when that heading is also a plain option one of the two scores is overwritten
and P cannot be rebuilt.

    python3 scripts/trailclimb.py /home/user/runs/csv/b518p-90
    python3 scripts/trailclimb.py 'runs/tf/*-gap90-*.csv.gz' --caste 4
    python3 scripts/trailclimb.py --selftest

**It prints the key's cardinality first** -- (seed, gap, arm, tag), from the
rows themselves rather than the file names -- because a parse keyed on fewer
dimensions than the run swept pools experiments silently (`CLAUDE.md`, *a
parse is a measurement*). On a trace without the `dwide` columns it says which
sections it cannot answer and answers the rest.

Controls (`--selftest`, exit 1 on failure): scores drawn exactly per P pass;
the same scores drawn by argmax FAIL, and drawn with linear rather than squared
weights FAIL (the check can tell the formula, not just gross bias); an ant that
always takes the highest-b option reads top-b and uphill 1.0; departures toward
the brighter side read 100% east and west in their bins; a return booked 200
frames before a departure lands in the 0-270 bin; a trace without `dwide` runs
and says what it cannot answer.
"""

import argparse
import bisect
import csv
import glob
import gzip
import math
import os
import random
import statistics as st
import sys
import tempfile
from collections import Counter, defaultdict
from statistics import NormalDist

LEVEL = (0, 1, 3, 4, 5, 7)
HALF = 1024            # TRAIL_HALF, raw: the floor in the gradient, as onetrail and btrailchart
FED_J = 200.0
CLASSES = ("fed-driven", "fed", "hungry", "lunch", "spoil", "laden", "worker")
READERS = ("fed-driven", "hungry", "lunch")
ZONES = ("west", "door", "road", "pile", "far", "shaft", "under")
AGE_BINS = ((0, 270), (270, 774), (774, 2000), (2000, math.inf))
Z99 = NormalDist().inv_cdf(0.995)
# Ten reliability bins per line, tested at once: the bin bar is Bonferroni
# over them as well as over the lines (`report`), so a trace drawn exactly per
# P fails by chance about one run in a hundred rather than one in five.
NBINS = 10
DWIDE = ["opts", "cross", "k", "chose", "reads_b"] + [f"{p}{d}" for p in ("bn", "bf", "b6", "sc") for d in range(8)]
BASE = ["seed", "gap", "arm", "tag", "frame", "id", "leg", "x", "y", "x2", "at_nest", "nest_x", "energy_j", "drive"]


def expand(specs):
    out = []
    for s in specs:
        if os.path.isdir(s):
            out += sorted(glob.glob(os.path.join(s, "*.csv")) + glob.glob(os.path.join(s, "*.csv.gz")))
        elif os.path.isfile(s):
            out.append(s)
        else:
            out += sorted(glob.glob(s))
    # A file named twice (a directory and a glob into it) would be read twice
    # and its bookings and departures counted twice.
    return list(dict.fromkeys(os.path.realpath(f) for f in out))


def rows(path):
    with (gzip.open(path, "rt") if path.endswith(".gz") else open(path, newline="")) as fh:
        rd = csv.reader(fh)
        h = next(rd)
        ix = {k: i for i, k in enumerate(h)}
        yield ix
        yield from rd


def fnum(s):
    try:
        return float(s)
    except ValueError:
        return math.nan


def leg_class(leg, reads_b, ej, drive, aid, caste):
    if caste and aid % caste == 0:
        return "worker"
    if not reads_b:
        return "laden"
    if leg == "laden":
        return "lunch"
    if leg == "empty":
        if ej < FED_J:
            return "hungry"
        return "fed-driven" if (not math.isnan(drive) and drive > 0) else "fed"
    # A spoil hauler reads B (`reads_b` is `reads_trail && !laden`, and spoil
    # is not food): 270 decisions on the 2026-09-29 dwide trace. Falling
    # through to `laden` filed readers in the non-reader class.
    return "spoil"


def zone(x, y, nx, gap, surf):
    dx = x - nx
    if surf is not None and y > surf + 1:
        return "shaft" if abs(dx) <= 3 else "under"
    if dx < -3:
        return "west"
    if abs(dx) <= 3:
        return "door"
    if abs(dx - gap) <= 10:
        return "pile"
    if 3 < dx < gap - 10:
        return "road"
    return "far"


def probs(opts, sc, k, linear=False):
    """P(o) as `choose_weighted` draws it; None if the weights are degenerate."""
    w = []
    for d in opts:
        s = sc[d]
        if math.isnan(s) or math.isnan(k):
            return None
        b = k + max(s, 0.0)
        w.append(b if linear else b * b)
    tot = sum(w)
    if tot <= 0:
        return None
    return {d: wi / tot for d, wi in zip(opts, w)}


class Acc:
    """Every accumulator, keyed so a later report can pool however it likes."""

    def __init__(self):
        self.keys = Counter()           # (seed, gap, arm, tag) -> rows
        self.files = defaultdict(set)   # key -> files
        self.file_keys = {}             # file -> set of keys
        self.dwide = {}                 # file -> bool
        self.surface = {}               # key -> y
        self.cons = defaultdict(lambda: [0, 0.0, 0.0, 0])            # (gap, (arm, tag), cls) -> n, E[top], var, hits
        self.bins = defaultdict(lambda: [[0, 0.0, 0.0, 0] for _ in range(NBINS)])
        self.skipped = Counter()        # (cls, why) -> n
        self.top = defaultdict(lambda: [0, 0, 0.0, 0.0])             # (key, cls, zone) -> n, hits, E, chance
        self.up = defaultdict(lambda: [0, 0, 0.0, 0.0])
        self.deps = []                  # dicts
        self.laid = defaultdict(Counter)  # key -> zone -> amount
        self.laid_proxy = {}            # key -> bool


def surface_of(path):
    """Modal y of empty rows away from the door, per key, from one pass."""
    it = rows(path)
    ix = next(it)
    ys = defaultdict(Counter)
    for r in it:
        if r[ix["leg"]] != "empty" or abs(int(r[ix["x"]]) - int(r[ix["nest_x"]])) <= 10:
            continue
        ys[(int(r[ix["seed"]]), int(r[ix["gap"]]), r[ix["arm"]], r[ix["tag"]])][int(r[ix["y"]])] += 1
    return {k: c.most_common(1)[0][0] for k, c in ys.items()}


def analyse(paths, caste=4, surface=None, linear=False):
    A = Acc()
    for path in paths:
        surf = {} if surface is not None else surface_of(path)
        it = rows(path)
        ix = next(it)
        missing = [c for c in BASE if c not in ix]
        if missing:
            print(f"trailclimb: {path}: missing {missing} -- skipped")
            continue
        wide = all(c in ix for c in DWIDE)
        A.dwide[path] = wide
        has_trip = "trip_load" in ix
        has_laid = wide and "emit_b_laid" in ix and "dep_x" in ix
        I = {c: ix[c] for c in BASE}
        W = {c: ix[c] for c in DWIDE} if wide else {}
        ants = defaultdict(list)
        fk = set()
        for r in it:
            key = (int(r[I["seed"]]), int(r[I["gap"]]), r[I["arm"]], r[I["tag"]])
            A.keys[key] += 1
            fk.add(key)
            sy = surface if surface is not None else surf.get(key)
            A.surface[key] = sy
            fr, aid = int(r[I["frame"]]), int(r[I["id"]])
            leg = r[I["leg"]]
            x, y, x2 = int(r[I["x"]]), int(r[I["y"]]), int(r[I["x2"]])
            nx, gap = int(r[I["nest_x"]]), key[1]
            ej, drv = fnum(r[I["energy_j"]]), fnum(r[I["drive"]])
            chose = r[W["chose"]] if wide else "-"
            bE = bW = b6E = b6W = None
            if wide and chose != "-":
                bn = [int(r[W[f"bn{d}"]]) for d in range(8)]
                bf = [int(r[W[f"bf{d}"]]) for d in range(8)]
                bE, bW = max(bn[0], bf[0]), max(bn[4], bf[4])
                b6E, b6W = int(r[W["b60"]]), int(r[W["b64"]])
            ants[aid].append((fr, x, x2, y, fnum(r[I["at_nest"]]), leg, ej, drv,
                              has_trip and r[ix["trip_load"]] == "1", bE, bW, b6E, b6W, nx, gap))
            if has_laid:
                amt = int(r[ix["emit_b_laid"]])
                if amt:
                    dx_ = r[ix["dep_x"]]
                    dx = int(dx_) if dx_ not in ("", "-") else x2
                    dy_ = r[ix["dep_y"]] if "dep_y" in ix else "-"
                    dy = int(dy_) if dy_ not in ("", "-") else y
                    A.laid[key][zone(dx, dy, nx, gap, sy)] += amt
            elif leg == "laden" and "moved" in ix and r[ix["moved"]] == "1":
                A.laid_proxy[key] = True
                A.laid[key][zone(x2, y, nx, gap, sy)] += 1
            if not wide or chose == "-":
                continue
            opts_m = int(r[W["opts"]])
            if not opts_m:
                continue
            cls = leg_class(leg, r[W["reads_b"]] == "1", ej, drv, aid, caste)
            if r[W["cross"]] == "1":
                A.skipped[(cls, "crossing")] += 1
                continue
            opts = [d for d in range(8) if opts_m >> d & 1]
            sc = [fnum(r[W[f"sc{d}"]]) for d in range(8)]
            P = probs(opts, sc, fnum(r[W["k"]]), linear)
            if P is None:
                A.skipped[(cls, "degenerate weights")] += 1
                continue
            c = int(chose)
            if c not in P:
                A.skipped[(cls, "chose outside opts")] += 1
                continue
            top = max(P.values())
            tops = [d for d in opts if P[d] >= top - 1e-12]
            pt = sum(P[d] for d in tops)
            # Per arm: a trace can be the draw on one arm and not another
            # (a rule that rescales a score after it is logged), and pooling
            # the arms would average a failure away.
            for cc in ((gap, col_of(key), cls), (gap, col_of(key), "all")):
                a = A.cons[cc]
                a[0] += 1
                a[1] += pt
                a[2] += pt * (1 - pt)
                a[3] += c in tops
                for d in opts:
                    b = A.bins[cc][min(NBINS - 1, int(P[d] * NBINS))]
                    b[0] += 1
                    b[1] += P[d]
                    b[2] += P[d] * (1 - P[d])
                    b[3] += c == d
            level = [d for d in opts if d in LEVEL]
            if len(level) < 2:
                continue
            bl = {d: max(bn[d], bf[d]) for d in level}
            bmax = max(bl.values())
            if bmax <= 0:
                continue
            z = zone(x, y, nx, gap, sy)
            best = [d for d in level if bl[d] == bmax]
            if len(best) == 1:
                t = A.top[(key, cls, z)]
                t[0] += 1
                t[1] += c == best[0]
                t[2] += P[best[0]]
                t[3] += 1.0 / len(opts)
            uphill = [d for d in level if bl[d] > sum(bl[e] for e in level if e != d) / (len(level) - 1)]
            # Level options all holding the same B have no uphill, and a
            # climber there reads as a failure -- as `top-b` skips a max that is
            # not unique. The selftest's climber hit it on 2 of 2,679.
            if c in level and uphill:
                plev = sum(P[d] for d in level)
                u = A.up[(key, cls, z)]
                u[0] += 1
                u[1] += c in uphill
                u[2] += sum(P[d] for d in uphill) / plev
                u[3] += len(uphill) / len(level)
        A.file_keys[path] = fk
        for k in fk:
            A.files[k].add(path)
        departures(A, ants, caste, has_trip, wide)
    return A


def departures(A, ants, caste, has_trip, wide):
    """Door departures of one file's ants; see the module doc."""
    FR, X, X2, Y, AN, LEG, EJ, DRV, TL, BE, BW, B6E, B6W, NX, GAP = range(15)
    # One file is one key (the cardinality line says so if not), so its
    # bookings are that key's colony's.
    books = []
    for aid, rs in ants.items():
        rs.sort()
        for p, r in zip(rs, rs[1:]):
            if has_trip and p[TL] and not r[TL] and r[AN] >= 0.5:
                books.append(r[FR])
    books.sort()
    for aid, rs in ants.items():
        if caste and aid % caste == 0:
            continue
        for i in range(1, len(rs)):
            p, r = rs[i - 1], rs[i]
            if not (p[AN] >= 0.5 and r[AN] < 0.5 and r[LEG] == "empty"):
                continue
            # The read at the departure row, or at the ant's next chooser row
            # within 3 rows: bn/bf are zeros on a row where the chooser did not
            # choose, and a zero there is "not read", not "dark".
            g2 = g6 = bd = None
            sub = 0
            if wide:
                for j in range(i, min(i + 3, len(rs))):
                    q = rs[j]
                    if q[BE] is not None:
                        g2 = (q[BE] - q[BW]) / (q[BE] + q[BW] + HALF)
                        g6 = (q[B6E] - q[B6W]) / (q[B6E] + q[B6W] + HALF)
                        bd = max(q[BE], q[BW])
                        sub = j - i
                        break
            x0, f0 = r[X], r[FR]
            side = "none"
            for q in rs[i:]:
                if q[FR] > f0 + 2000:
                    break
                if abs(q[X2] - x0) >= 30:
                    side = "E" if q[X2] > x0 else "W"
                    break
            if r[EJ] < FED_J:
                cls = "hungry"
            else:
                cls = "fed-driven" if (not math.isnan(r[DRV]) and r[DRV] > 0) else "fed"
            age = None
            if has_trip:
                j = bisect.bisect_left(books, f0)
                age = f0 - books[j - 1] if j else math.inf
            A.deps.append(dict(file=len(A.file_keys) - 1, frame=f0, cls=cls, side=side, g2=g2, g6=g6, bdoor=bd,
                               sub=sub, age=age, dx=x0 - r[NX]))


def col_of(key):
    return (key[2], key[3])


def pct(n, d):
    return f"{n / d:.0%}" if d else "n/a"


def report(A, caste, strict=False):
    keys = sorted(A.keys)
    print(f"trailclimb: {sum(A.keys.values())} rows over {len(keys)} (seed,gap,arm,tag) keys from {len(A.file_keys)} files")
    for i, d in enumerate(("seed", "gap", "arm", "tag")):
        vals = sorted({k[i] for k in keys})
        print(f"            {d:<5} {len(vals):>3}  {vals if len(vals) <= 12 else f'[{vals[0]}..{vals[-1]}]'}")
    multi = [f for f, ks in A.file_keys.items() if len(ks) > 1]
    dup = [k for k, fs in A.files.items() if len(fs) > 1]
    if multi or dup:
        print(f"            ** {len(multi)} files hold more than one key and {len(dup)} keys span more than one file:"
              f" bookings and departures are paired per file, so this pools runs. Split the input.")
    nw = sum(A.dwide.values())
    print(f"            dwide columns in {nw} of {len(A.dwide)} files")
    sv = Counter(A.surface.values())
    print(f"            surface row (modal y of empty rows > 10 from the door): {dict(sv)}")

    # 1. SELF-CONSISTENCY -------------------------------------------------
    print("\n1. SELF-CONSISTENCY: do the logged scores reproduce the draws? (the positive control)")
    verdict = None
    if not nw:
        print("   unavailable: needs the dwide columns (opts, sc0..sc7, k, chose, reads_b). Rerun with `dwide`.")
    else:
        verdict = True
        blocks = sorted({(g, col) for (g, col, _) in A.cons})
        # **The verdict is family-wise.** Every (gap, arm, class) line is a
        # test, and ANDing 18-40 of them each at 99% judged a trace drawn
        # exactly per P a FAIL 9 times in 40 on a three-arm fixture. So each
        # line is held to 99% over the family (Bonferroni over the lines that
        # can be tested, and over their bins); the 99% column is each line's
        # own interval, as the plan asks, and a line outside it but inside
        # the family bar is marked rather than failed.
        fam = [(g, col, cls) for g, col in blocks for cls in CLASSES + ("all",)
               if A.cons.get((g, col, cls)) and A.cons[(g, col, cls)][2] >= 1.0]
        m = max(1, len(fam))
        zbar = NormalDist().inv_cdf(1 - 0.01 / (2 * m))
        binbar = NormalDist().inv_cdf(1 - 0.01 / (2 * m * NBINS))
        for g, col in blocks:
            print(f"   gap {g} {'/'.join(col)}")
            print(f"   {'class':<11} {'n':>8} {'top-P picks':>12} {'expected':>10} {'99% +/-':>8} {'z':>6} {'worst bin z':>11}  verdict")
            for cls in CLASSES + ("all",):
                a = A.cons.get((g, col, cls))
                if not a or not a[0]:
                    continue
                n, e, var, hit = a
                sd = math.sqrt(var)
                z = (hit - e) / sd if sd > 0 else 0.0
                bz = 0.0
                for b in A.bins[(g, col, cls)]:
                    if b[0] and b[2] > 0:
                        bz = max(bz, abs(b[3] - b[1]) / math.sqrt(b[2]))
                ok = abs(z) <= zbar and bz <= binbar and sd >= 1.0
                note = "" if sd >= 1.0 else " (too few to test)"
                if sd >= 1.0:
                    verdict = verdict and ok
                    if ok and abs(z) > Z99:
                        note = " (outside its own 99%)"
                print(f"   {cls:<11} {n:>8} {hit:>12} {e:>10.1f} {Z99 * sd:>8.1f} {z:>+6.2f} {bz:>11.2f}  "
                      f"{('PASS' if ok else 'FAIL') if sd >= 1.0 else 'n/a'}{note}")
            print(f"   reliability, all classes: every (decision, option) pair binned by P(option)")
            print(f"   {'P bin':<9} {'pairs':>9} {'mean P':>8} {'picked':>8} {'z':>6}")
            for i, b in enumerate(A.bins[(g, col, "all")]):
                if not b[0]:
                    continue
                z = (b[3] - b[1]) / math.sqrt(b[2]) if b[2] > 0 else 0.0
                flag = "  **" if abs(z) > binbar else ""
                print(f"   {i / NBINS:.1f}-{(i + 1) / NBINS:.1f}  {b[0]:>9} {b[1] / b[0]:>8.3f} {b[3] / b[0]:>8.3f} {z:>+6.2f}{flag}")
        # A pick outside the options is not noise: the trace and the draw
        # disagree about what was on offer, so it fails outright.
        if any(why == "chose outside opts" and m for (_, why), m in A.skipped.items()):
            verdict = False
        sk = Counter()
        for (c, why), m in A.skipped.items():
            sk[why] += m
        if sk:
            print(f"   left out: " + ", ".join(f"{why} {m}" for why, m in sorted(sk.items())))
        print(f"   bars: 99% over the family of {m} tested lines (Bonferroni): |z| <= {zbar:.2f} on the top-P count,"
              f" <= {binbar:.2f} on every bin (x{NBINS} bins). One line alone at 99%: {Z99:.2f}.")
        print(f"   SELF-CONSISTENCY: {'PASS' if verdict else 'FAIL -- the trace is not the draw; do not quote section 2'}")

    # 2. CLIMB ------------------------------------------------------------
    print("\n2. CLIMB: level options (dx != 0), b = max(bn, bf); realised / expected (logged P) / chance (uniform)")
    if not nw:
        print("   unavailable: needs bn0..bn7, bf0..bf7, opts, sc0..sc7, k, chose. Rerun with `dwide`.")
    else:
        pool = defaultdict(lambda: [[0, 0, 0.0, 0.0], [0, 0, 0.0, 0.0]])
        for (key, cls, z), t in A.top.items():
            p = pool[(key[1], col_of(key), cls, z)][0]
            for i in range(4):
                p[i] += t[i]
        for (key, cls, z), u in A.up.items():
            p = pool[(key[1], col_of(key), cls, z)][1]
            for i in range(4):
                p[i] += u[i]
        print(f"   {'gap':>4} {'arm/tag':<16} {'class':<11} {'zone':<6} {'n top':>7} {'top-b':>6} {'exp':>6} {'chance':>6}"
              f" {'n up':>7} {'uphill':>6} {'exp':>6} {'chance':>6}")
        for g, col, cls, z in sorted(pool, key=lambda k: (k[0], k[1], CLASSES.index(k[2]), ZONES.index(k[3]))):
            t, u = pool[(g, col, cls, z)]
            if t[0] + u[0] < 20:
                continue
            f = lambda a: (f"{a[0]:>7} {a[1] / a[0]:>6.3f} {a[2] / a[0]:>6.3f} {a[3] / a[0]:>6.3f}" if a[0]
                           else f"{0:>7} {'':>6} {'':>6} {'':>6}")
            print(f"   {g:>4} {'/'.join(col):<16} {cls:<11} {z:<6} {f(t)} {f(u)}")
        print("   rows with fewer than 20 decisions are not printed. `top-b`: the unique highest-b option was taken;")
        print("   `uphill`: the pick's b beats the mean of the other level options (of level picks).")
        paired(A)

    # 3. DOOR DEPARTURES --------------------------------------------------
    print("\n3. DOOR DEPARTURES: empty ants leaving the nest band (at_nest >= 0.5 -> < 0.5), workers excluded"
          if caste else "\n3. DOOR DEPARTURES: empty ants leaving the nest band (at_nest >= 0.5 -> < 0.5)")
    if not A.deps:
        print("   none found")
    else:
        fk = list(A.file_keys.values())
        cols = defaultdict(list)
        for d in A.deps:
            k = next(iter(fk[d["file"]]))
            cols[(k[1], col_of(k))].append(d)
        has_g = any(d["g2"] is not None for d in A.deps)
        has_age = any(d["age"] is not None for d in A.deps)
        if not has_g:
            print("   the E-W read is unavailable: needs bn0/bf0/bn4/bf4 and b60/b64 (dwide). Sides and ages still read.")
        if not has_age:
            print("   return ages are unavailable: needs trip_load.")
        for (g, col), ds in sorted(cols.items()):
            n = len(ds)
            sides = Counter(d["side"] for d in ds)
            cl = Counter(d["cls"] for d in ds)
            print(f"   gap {g} {'/'.join(col)}: {n} departures ({', '.join(f'{c} {cl[c]}' for c in ('fed-driven', 'fed', 'hungry') if cl[c])});"
                  f" side E {pct(sides['E'], n)} W {pct(sides['W'], n)} none {pct(sides['none'], n)}")
            if has_g:
                fed = [d for d in ds if d["cls"] != "hungry" and d["g2"] is not None]
                print(f"     fed departures with a read: {len(fed)}; grad2 > 0.1 on {pct(sum(d['g2'] > 0.1 for d in fed), len(fed))},"
                      f" grad6 > 0.1 on {pct(sum(d['g6'] > 0.1 for d in fed), len(fed))}"
                      f"   (P0.8: >= 50% at reach 2, >= 65% at reach 6; today <= 35%)")
                sub = sum(1 for d in ds if d["g2"] is not None and d["sub"])
                if sub:
                    print(f"     ({sub} read at the next chooser row within 3: the departure row itself did not choose)")
                for name, gk in (("grad2", "g2"), ("grad6", "g6")):
                    cells = []
                    for lab in ("< -0.1", "-0.1..0.1", "> 0.1"):
                        sel = [d for d in ds if d[gk] is not None and _inbin(d[gk], lab)]
                        cells.append(f"{lab} n {len(sel):>4} E {pct(sum(d['side'] == 'E' for d in sel), len(sel)):>4}"
                                     f" W {pct(sum(d['side'] == 'W' for d in sel), len(sel)):>4}")
                    print(f"     {name}: " + " | ".join(cells))
            if has_age:
                print(f"     {'last return':<12} {'n':>5} {'E':>5} {'W':>5} {'none':>5} {'med door B':>11} {'grad2>0.1':>10}")
                for bi, (lo, hi) in enumerate(AGE_BINS):
                    sel = [d for d in ds if d["age"] is not None and age_bin(d["age"]) == bi]
                    lab = f"{lo}-{hi}" if hi < math.inf else f">{lo}"
                    bd = [d["bdoor"] for d in sel if d["bdoor"] is not None]
                    gg = [d for d in sel if d["g2"] is not None]
                    print(f"     {lab:<12} {len(sel):>5} {pct(sum(d['side'] == 'E' for d in sel), len(sel)):>5}"
                          f" {pct(sum(d['side'] == 'W' for d in sel), len(sel)):>5} {pct(sum(d['side'] == 'none' for d in sel), len(sel)):>5}"
                          f" {(f'{st.median(bd):.0f}' if bd else 'n/a'):>11} {pct(sum(d['g2'] > 0.1 for d in gg), len(gg)):>10}")
        print("   side: the sign of x - x_departure once it reaches 30 cells, within 2,000 frames. last return: frames")
        print("   since any ant booked a return (trip_load 1 -> 0 at the nest) before the departure; >2000 includes never.")
        print("   P0.9 reads `med door B` falling down the age bins.")

    # 4. LAID -------------------------------------------------------------
    print("\n4. LAID: where trail B went down, by zone")
    if A.laid:
        cols = defaultdict(Counter)
        proxy = set()
        for k, c in A.laid.items():
            cols[(k[1], col_of(k))].update(c)
            if A.laid_proxy.get(k):
                proxy.add((k[1], col_of(k)))
        for (g, col), c in sorted(cols.items()):
            tot = sum(c.values())
            what = "moved rows of a laden ant (proxy: no emit_b_laid)" if (g, col) in proxy else "raw B laid (emit_b_laid)"
            print(f"   gap {g} {'/'.join(col):<16} " + " ".join(f"{z} {c[z] / tot:.1%}" for z in ZONES if c[z])
                  + f"   of {tot} {what}")
    else:
        print("   unavailable: needs emit_b_laid and dep_x (dwide), or `moved` on old traces")
    if strict and verdict is False:
        return 3
    return 0


def age_bin(age):
    """The index into AGE_BINS; the last bin is open, and holds `inf` -- a
    departure before the colony's first return."""
    for i, (lo, hi) in enumerate(AGE_BINS):
        if lo <= age < hi:
            return i
    return len(AGE_BINS) - 1


def _inbin(g, lab):
    if lab == "< -0.1":
        return g < -0.1
    if lab == "> 0.1":
        return g > 0.1
    return -0.1 <= g <= 0.1


def paired(A):
    """Per seed, each arm's share, so arms are compared within a world."""
    cols = sorted({col_of(k) for k in A.keys})
    if len(cols) < 2:
        return
    print("\n   paired by seed (zones door+road pooled): uphill share per arm")
    for cls in READERS:
        for g in sorted({k[1] for k in A.keys}):
            seeds = sorted({k[0] for k in A.keys if k[1] == g})
            tab = {}
            for s in seeds:
                for c in cols:
                    n = h = 0
                    for z in ("door", "road"):
                        u = A.up.get(((s, g) + c, cls, z))
                        if u:
                            n += u[0]
                            h += u[1]
                    tab[(s, c)] = (h / n, n) if n >= 10 else None
            if not any(tab.values()):
                continue
            print(f"   {cls} gap {g}: " + "  ".join("/".join(c) for c in cols))
            for s in seeds:
                print(f"     seed {s:>3}  " + "  ".join(f"{tab[(s, c)][0]:.3f} ({tab[(s, c)][1]})" if tab[(s, c)] else "   n/a     "
                                                 for c in cols))
            for i, a in enumerate(cols):
                for b in cols[i + 1:]:
                    both = [(tab[(s, a)][0], tab[(s, b)][0]) for s in seeds if tab[(s, a)] and tab[(s, b)]]
                    hi = sum(x > y for x, y in both)
                    lo = sum(x < y for x, y in both)
                    print(f"     {'/'.join(a)} above {'/'.join(b)} on {hi} of {len(both)} seeds, below on {lo}")


# --- controls ------------------------------------------------------------


def _fixture(path, mode, seed, arm, rng, wide=True, n=2500, gz=False):
    """Synthetic decision rows. `mode`: fair (draw per P), argmax, linear (draw
    with k+s weights), climb (always the highest-b level option)."""
    head = BASE + ["y2", "fill", "trip_load", "moved"] + ((DWIDE + ["emit_b_laid", "dep_x", "dep_y", "since_trip"]) if wide else [])
    nx, gap, surf = 50, 90, 95
    out = []
    frame = 0
    for i in range(n):
        frame = 6 * (i // 5 + 1)
        aid = 1 + i % 5
        x = rng.randint(nx - 20, nx + gap + 15)
        y = surf if rng.random() < 0.9 else surf + rng.randint(2, 8)
        leg = rng.choice(["empty", "empty", "empty", "laden", "laden", "spoil"])
        ej = rng.choice([150.0, 400.0])
        drv = rng.choice(["0.8000", "NaN", "0.0000"])
        reads = leg == "empty" or rng.random() < 0.5
        opts = sorted(rng.sample(range(8), rng.randint(2, 6)))
        sc = [math.nan] * 8
        for d in opts:
            sc[d] = rng.uniform(-0.3, 1.5)
        k = 0.1
        bn = [max(0, int(2000 + 900 * [1, 1, 0, -1, -1, -1, 0, 1][d] + rng.randint(-300, 300))) for d in range(8)]
        bf = [max(0, v + rng.randint(-200, 200)) for v in bn]
        P = probs(opts, sc, k, linear=(mode == "linear"))
        if mode == "argmax":
            c = max(opts, key=lambda d: P[d])
        elif mode == "climb":
            lev = [d for d in opts if d in LEVEL]
            c = max(lev, key=lambda d: max(bn[d], bf[d])) if len(lev) >= 2 else opts[0]
        else:
            t, c = rng.random(), opts[-1]
            for d in opts:
                if t < P[d]:
                    c = d
                    break
                t -= P[d]
        row = dict(seed=seed, gap=gap, arm=arm, tag="t", frame=frame, id=aid, leg=leg, x=x, y=y, x2=x + DIRS_DX[c],
                   at_nest="0.0000", nest_x=nx, energy_j=ej, drive=drv, y2=y, fill="0.0000", trip_load=0,
                   moved=1, opts=sum(1 << d for d in opts), cross=0, k=k, chose=c, reads_b=int(reads),
                   emit_b_laid=7314 if leg == "laden" else 0, dep_x=x + DIRS_DX[c], dep_y=y, since_trip=0)
        for d in range(8):
            row[f"bn{d}"], row[f"bf{d}"], row[f"b6{d}"] = bn[d], bf[d], bn[d]
            row[f"sc{d}"] = "NaN" if math.isnan(sc[d]) else f"{sc[d]:.6f}"
        out.append(row)
    # Departures: ant 7 leaves toward a bright east three times, ant 8 toward
    # a bright west three times; ant 9 books returns 1,000 and 200 frames
    # before the first, so the LAST booking is the one that must be read.
    f0 = frame + 600
    ev = []

    def put(fr, aid, x, an, leg="empty", tl=0, bE=0, bW=0, chose="-", x2=None):
        r = dict(seed=seed, gap=gap, arm=arm, tag="t", frame=fr, id=aid, leg=leg, x=x, y=surf, x2=x if x2 is None else x2,
                 at_nest=an, nest_x=nx, energy_j=400.0, drive="0.8000", y2=surf, fill="0.0000", trip_load=tl, moved=0,
                 opts=0 if chose == "-" else 1, cross=0, k=0.1, chose=chose, reads_b=1, emit_b_laid=0, dep_x=x, dep_y=surf, since_trip=0)
        for d in range(8):
            r[f"bn{d}"] = r[f"bf{d}"] = r[f"b6{d}"] = bE if d == 0 else bW if d == 4 else 0
            r[f"sc{d}"] = "0.5" if (d == 0 and chose != "-") else "NaN"
        ev.append(r)

    for back in (1000, 200):
        put(f0 - back - 6, 9, nx, "1.0000", leg="laden", tl=1)
        put(f0 - back, 9, nx, "1.0000", leg="empty", tl=0)
    for rep in range(3):
        base = f0 + rep * 3000
        for aid, sgn, bE, bW in ((7, 1, 6000, 500), (8, -1, 500, 6000)):
            put(base - 6, aid, nx, "1.0000")
            for j in range(40):
                put(base + 6 * j, aid, nx + sgn * j, "0.0000", bE=bE, bW=bW, chose="0", x2=nx + sgn * (j + 1))
            put(base + 6 * 40 + 1200, aid, nx, "1.0000")
    out += ev
    op = gzip.open if gz else open
    with op(path, "wt", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(head)
        for r in sorted(out, key=lambda r: r["frame"]):
            w.writerow([r.get(c, "") for c in head])


DIRS_DX = [1, 1, 0, -1, -1, -1, 0, 1]


def selftest():
    try:
        _selftest()
    except AssertionError as e:
        print(f"trailclimb selftest: FAIL -- {e}")
        return 1
    return 0


def _selftest():
    tmp = tempfile.mkdtemp(prefix="trailclimb-")
    rng = random.Random(11)
    null = open(os.devnull, "w")

    def run(files, **kw):
        so = sys.stdout
        sys.stdout = null
        try:
            A = analyse(files, **kw)
            rc = report(A, kw.get("caste", 4), strict=True)
        finally:
            sys.stdout = so
        return A, rc

    fair = [os.path.join(tmp, f"fair-{s}.csv") for s in (1, 2)]
    for s, p in zip((1, 2), fair):
        _fixture(p, "fair", s, "self", rng, n=6000)
    gzp = os.path.join(tmp, "fair-3.csv.gz")
    _fixture(gzp, "fair", 3, "self", rng, n=3000, gz=True)
    A, rc = run(fair + [gzp], caste=0)
    assert len(A.keys) == 3 and rc == 0, f"a trace drawn exactly per P must PASS (rc {rc}, keys {len(A.keys)})"
    n, e, var, hit = A.cons[(90, ("self", "t"), "all")]
    assert n > 10000 and abs(hit - e) <= Z99 * math.sqrt(var), f"fair: {hit} against {e:.1f}"
    for cls in ("fed-driven", "fed", "hungry", "lunch", "spoil", "laden"):
        assert A.cons[(90, ("self", "t"), cls)][0] > 100, f"the fixture does not reach class {cls}"
    # A spoil hauler that reads B is not the non-reader control.
    assert leg_class("spoil", True, 400.0, math.nan, 1, 0) == "spoil" and leg_class("spoil", False, 400.0, math.nan, 1, 0) == "laden"
    zs = Counter(z for (_, _, z) in A.up)
    for z in ("west", "door", "road", "pile", "shaft"):
        assert zs[z], f"the fixture does not reach zone {z}"
    # Expected climb equals realised within noise on the fair trace.
    tn = sum(t[0] for t in A.top.values())
    th = sum(t[1] for t in A.top.values())
    te = sum(t[2] for t in A.top.values())
    assert abs(th - te) < 4 * math.sqrt(tn), f"fair top-b: realised {th} expected {te:.0f}"

    arg = os.path.join(tmp, "argmax.csv")
    _fixture(arg, "argmax", 1, "argmax", rng, n=4000)
    A, rc = run([arg], caste=0)
    assert rc == 3, "an argmax trace must FAIL self-consistency"
    lin = os.path.join(tmp, "linear.csv")
    _fixture(lin, "linear", 1, "linear", rng, n=12000)
    A, rc = run([lin], caste=0)
    assert rc == 3, "draws made with linear weights must FAIL against the squared rule -- the check cannot tell the formula"
    # ...and the same linear trace PASSES when checked against the linear rule: the failure is the formula, not noise.
    A, rc = run([lin], caste=0, linear=True)
    assert rc == 0, "the linear trace must pass against its own rule"

    cl = os.path.join(tmp, "climb.csv")
    _fixture(cl, "climb", 1, "climb", rng, n=3000)
    A, _ = run([cl], caste=0)
    tn = sum(t[0] for t in A.top.values())
    th = sum(t[1] for t in A.top.values())
    un = sum(u[0] for u in A.up.values())
    uh = sum(u[1] for u in A.up.values())
    assert tn > 500 and th == tn and uh == un, f"an ant that always climbs must read 1.0: top {th}/{tn} up {uh}/{un}"

    # Departures: 3 east toward bright east, 3 west toward bright west; the first 200 frames after a booking.
    A, _ = run(fair[:1], caste=0)
    deps = [d for d in A.deps if d["g2"] is not None and abs(d["g2"]) > 0.5]
    east = [d for d in deps if d["g2"] > 0.1]
    west = [d for d in deps if d["g2"] < -0.1]
    assert len(east) == 3 and all(d["side"] == "E" for d in east), f"east departures: {[(d['g2'], d['side']) for d in east]}"
    assert len(west) == 3 and all(d["side"] == "W" for d in west), f"west departures: {[(d['g2'], d['side']) for d in west]}"
    first = min(deps, key=lambda d: d["frame"])
    assert first["age"] == 200, f"the last return, booked 200 frames before, must read age 200, got {first['age']}"
    assert sorted(d["age"] for d in deps)[-1] == 6200, f"a departure 6,200 frames after the last return: {sorted(d['age'] for d in deps)}"
    assert [age_bin(a) for a in (0, 269, 270, 774, 1999, 2000, math.inf)] == [0, 0, 1, 2, 2, 3, 3], \
        "a departure before the first return (age inf) must land in the open bin, not vanish from the table"
    # The caste: ids 7 and 8 are not workers at caste 4 -- but 8 is, so it drops out.
    A, _ = run(fair[:1], caste=4)
    assert not any(d["g2"] is not None and d["g2"] < -0.5 for d in A.deps), "a worker-caste ant is counted as a forager"

    old = os.path.join(tmp, "old.csv")
    _fixture(old, "fair", 1, "self", rng, wide=False, n=500)
    so = sys.stdout
    buf = open(os.path.join(tmp, "old.txt"), "w")
    sys.stdout = buf
    try:
        A = analyse([old], caste=0)
        rc = report(A, 0)
    finally:
        sys.stdout = so
        buf.close()
    txt = open(os.path.join(tmp, "old.txt")).read()
    assert rc == 0 and "unavailable: needs the dwide" in txt and A.deps, "a trace without dwide must run and say what it cannot answer"
    assert all(d["g2"] is None for d in A.deps), "a trace without dwide must not invent a gradient"
    print("trailclimb selftest: ok -- per-P draws PASS (csv and csv.gz, all classes and zones, spoil readers apart); argmax FAILS; linear")
    print("                     weights FAIL against the squared rule and PASS against their own; a climber reads 1.0;")
    print("                     departures go with the brighter side; a return 200 frames back reads 200; the caste")
    print("                     drops out; a trace without dwide runs and says what it cannot answer")


def main():
    ap = argparse.ArgumentParser(description="Do ants climb trail B? Reads trailfollow decisioncsv dwide; see the module doc.")
    ap.add_argument("paths", nargs="*", help="directories, globs or files (.csv / .csv.gz)")
    ap.add_argument("--caste", type=int, default=4, help="nest-worker caste: id %% caste == 0 (Storeroom::SHIPPED is 4; 0 = none)")
    ap.add_argument("--surface", type=int, help="the walking row (default: per key, the modal y of empty rows > 10 from the door)")
    ap.add_argument("--strict", action="store_true", help="exit 3 if self-consistency FAILS")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        return selftest()
    files = expand(a.paths)
    if not files:
        ap.error("no .csv/.csv.gz files matched")
    A = analyse(files, caste=a.caste, surface=a.surface)
    return report(A, a.caste, strict=a.strict)


if __name__ == "__main__":
    sys.exit(main())
