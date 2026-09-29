#!/usr/bin/env python3
"""The pace of the colony's returns, read off the trace, as the parameters of the
design-level trail model.

`onetrail mode=stream` (Stage 0c of `Reports/food-trail-plan-2026-09-29.md`)
is the EXPECTED food trail: a stream of returning ants walking home at a pace,
one every so many frames, shuffling at the door, laying under a rule. Its
numbers must come from the live colony, not from a guess, or the comparison
`btrailchart --expect` draws is between the colony and an invented animal. This
reads `trailfollow decisioncsv` traces and prints them, per seed and pooled:

  1. the cargo's age at booking -- `since_trip` (ticks since the crop's last
     trip pickup) on the row where `trip_load` goes 1 -> 0 at the nest: median
     and p90. The p90 is the T criterion's input: a single odometer deposit,
     0.714 * T/(T + age) * DEPOSIT, must still clear TRAIL_HALF (1,024 raw).
  2. laden speed on the road -- for trip-laden ants (trip_load 1, leg laden)
     between the pile zone and the door (nest+3 < x < nest+gap-10), cells of x
     covered toward home per tick, per decision row (net: backtracking counts
     against it; gross beside it), and the per-trip transit from leaving the
     pile zone to reaching the door.
  3. the intervals between successive booked returns (frames: median, p75, p90,
     and the mean, which is what fixes the trail's deposit RATE), all bookings
     and those whose trip took food at the pile (a pickup within 10 of it).
  4. dwell -- ticks a trip-laden ant spends within 3 columns of the door before
     its booking, on the surface (y <= surface + 1, where the design model lays)
     and in all.

Then `key=value` lines per (gap, arm, tag), ready for `onetrail mode=stream`:
`span=`, `speed=` (the median trip's pace from leaving the pile zone to the
door; the pooled per-row pace is printed beside it and is lower, being
weighted by the trips that loiter), `every=` (the mean interval of
pile returns: the rate-preserving choice; the median is printed beside it),
`dwell=` (the median surface dwell), `tickframes=` (measured, not assumed), and
`age_p90=` with the smallest T in {16, 24, 32, 48} whose deposit at that age
clears TRAIL_HALF.

    python3 scripts/trailpace.py /home/user/runs/csv/b518p-90
    python3 scripts/trailpace.py 'runs/tf/*gap140*.csv.gz'
    python3 scripts/trailpace.py --selftest

**Old traces have no `since_trip`**, and the age is then ESTIMATED from frames:
the frames since the last pickup away from home during the trip (a row with
`bite_x`) over the measured frames per tick, printed as `age~`. On a `dwide`
trace both are computed and their agreement is printed: the estimator's own
positive control, so an old trace's `age~` can be trusted as far as that line
says and no further.

Keyed on (seed, gap, arm, tag) from the rows, with the cardinality printed
first (`CLAUDE.md`, *a parse is a measurement*). Controls (`--selftest`, exit 1
on failure): a synthetic colony whose ants walk home at exactly 0.5 cells a
tick, book every 300 frames, dwell a known number of rows and carry a known
age reads back every one of those numbers, from `since_trip` and, with the
column removed, from the estimate.
"""

import argparse
import csv
import glob
import gzip
import math
import os
import statistics as st
import sys
import tempfile
from collections import Counter, defaultdict

DEPOSIT = 40 * 256
TRAIL_HALF = DEPOSIT // 10
EMIT = 0.714          # `onetrail`'s default emit; the live brain's EmitB lays 7,314 raw, a hair above 0.714 x 10,240
TS = (16, 24, 32, 48)
BASE = ["seed", "gap", "arm", "tag", "frame", "id", "leg", "x", "y", "x2", "at_nest", "nest_x"]


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
    # and its bookings counted twice.
    return list(dict.fromkeys(os.path.realpath(f) for f in out))


def q(v, p):
    """Nearest-rank quantile, as `drivefade.py`."""
    v = sorted(v)
    return v[min(len(v) - 1, int(p * len(v)))] if v else math.nan


def med(v):
    return st.median(v) if v else math.nan


def deposit(t, age):
    """The odometer's single deposit at `age` ticks, as `onetrail`'s `laid` computes it."""
    return int(EMIT * (t / (t + age)) * DEPOSIT)


def read(path):
    """{key: {ant: [row tuples sorted by frame]}}, and which optional columns exist."""
    with (gzip.open(path, "rt") if path.endswith(".gz") else open(path, newline="")) as fh:
        rd = csv.reader(fh)
        h = next(rd)
        ix = {k: i for i, k in enumerate(h)}
        miss = [c for c in BASE if c not in ix]
        if miss:
            return None, miss, {}
        has = {c: c in ix for c in ("trip_load", "bite_x", "since_trip")}
        out = defaultdict(lambda: defaultdict(list))
        I = [ix[c] for c in BASE]
        tl, bx, stc = ix.get("trip_load"), ix.get("bite_x"), ix.get("since_trip")
        for r in rd:
            seed, gap, arm, tag, fr, aid, leg, x, y, x2, an, nx = (r[i] for i in I)
            key = (int(seed), int(gap), arm, tag)
            b = r[bx] if bx is not None else "-"
            out[key][int(aid)].append((int(fr), int(x), int(x2), int(y), float(an), leg,
                                       (r[tl] == "1") if tl is not None else None,
                                       int(b) if b not in ("-", "") else None,
                                       int(r[stc]) if stc is not None and r[stc] not in ("-", "") else None,
                                       int(nx)))
    for k in out:
        for a in out[k]:
            out[k][a].sort()
    return out, [], has


FR, X, X2, Y, AN, LEG, TL, BX, ST, NX = range(10)


def surface(ants):
    ys = Counter(r[Y] for rs in ants.values() for r in rs if r[LEG] == "empty" and abs(r[X] - r[NX]) > 10)
    return ys.most_common(1)[0][0] if ys else None


def pace(key, ants):
    """Everything for one key (one colony run)."""
    gap = key[1]
    gaps = Counter()
    for rs in ants.values():
        for p, r in zip(rs, rs[1:]):
            gaps[r[FR] - p[FR]] += 1
    tickf = gaps.most_common(1)[0][0] if gaps else 6
    surf = surface(ants)
    have_tl = any(r[TL] is not None for rs in ants.values() for r in rs[:1])
    R = dict(tickf=tickf, surf=surf, books=[], road_net=0, road_gross=0, road_rows=0, transit=[], have_tl=have_tl,
             have_st=any(r[ST] is not None for rs in ants.values() for r in rs[:1]))
    if not have_tl:
        return R
    for aid, rs in ants.items():
        trip = None   # dict while a trip mark stands
        for i, r in enumerate(rs):
            p = rs[i - 1] if i else None
            nx = r[NX]
            if r[TL] and not (p and p[TL]):
                trip = dict(start=r[FR], last_pick=r[FR], pile=False, dwell_s=0, dwell_a=0, leave=None, xleave=None, door=None)
            if trip is not None and r[TL]:
                if r[BX] is not None:
                    trip["last_pick"] = r[FR]
                    if r[BX] - nx >= gap - 10:
                        trip["pile"] = True
                if abs(r[X] - nx) <= 3:
                    trip["dwell_a"] += 1
                    if surf is None or r[Y] <= surf + 1:
                        trip["dwell_s"] += 1
                if r[LEG] == "laden" and nx + 3 < r[X] < nx + gap - 10:
                    R["road_net"] += r[X] - r[X2]
                    R["road_gross"] += abs(r[X2] - r[X])
                    R["road_rows"] += 1
                if trip["leave"] is None and trip["pile"] and r[X] < nx + gap - 10:
                    trip["leave"], trip["xleave"] = r[FR], r[X]
                if trip["door"] is None and trip["leave"] is not None and abs(r[X] - nx) <= 3:
                    trip["door"] = r[FR]
                    if r[FR] > trip["leave"]:
                        R["transit"].append((trip["xleave"] - r[X]) * tickf / (r[FR] - trip["leave"]))
            if p and p[TL] and not r[TL] and trip is not None:
                if r[AN] >= 0.5:
                    R["books"].append(dict(frame=r[FR], pile=trip["pile"], age=r[ST],
                                           est=(r[FR] - trip["last_pick"]) / tickf, dwell_s=trip["dwell_s"], dwell_a=trip["dwell_a"]))
                trip = None
    R["books"].sort(key=lambda b: b["frame"])
    return R


def intervals(books):
    f = [b["frame"] for b in books]
    return [b - a for a, b in zip(f, f[1:])]


def summary(Rs):
    """Pool several keys' results into the numbers the model takes."""
    books = [b for R in Rs for b in R["books"]]
    pile = [b for b in books if b["pile"]]
    age = [b["age"] for b in books if b["age"] is not None]
    est = [b["est"] for b in books]
    diff = [abs(b["age"] - b["est"]) for b in books if b["age"] is not None]
    iv_all = [x for R in Rs for x in intervals(R["books"])]
    iv_pile = [x for R in Rs for x in intervals([b for b in R["books"] if b["pile"]])]
    rows = sum(R["road_rows"] for R in Rs)
    return dict(n=len(books), n_pile=len(pile), age=age, est=est, diff=diff,
                net=sum(R["road_net"] for R in Rs) / rows if rows else math.nan,
                gross=sum(R["road_gross"] for R in Rs) / rows if rows else math.nan, rows=rows,
                transit=[t for R in Rs for t in R["transit"]], iv_all=iv_all, iv_pile=iv_pile,
                dwell_s=[b["dwell_s"] for b in books], dwell_a=[b["dwell_a"] for b in books],
                tickf=Counter(R["tickf"] for R in Rs).most_common(1)[0][0])


def fmt(v, spec=".1f"):
    return "n/a" if v is None or (isinstance(v, float) and math.isnan(v)) else format(v, spec)


def main():
    ap = argparse.ArgumentParser(description="The pace of returns, as onetrail mode=stream parameters; see the module doc.")
    ap.add_argument("paths", nargs="*")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        return selftest()
    files = expand(a.paths)
    if not files:
        ap.error("no .csv/.csv.gz files matched")
    return run(files)


def run(files):
    res = {}
    files_of = defaultdict(list)
    has_all = Counter()
    for f in files:
        data, miss, has = read(f)
        if data is None:
            print(f"trailpace: {f}: missing {miss} -- skipped")
            continue
        for c, v in has.items():
            has_all[c] += v
        for k, ants in data.items():
            if k in res:
                print(f"trailpace: ** key {k} in more than one file ({files_of[k][0]}, {f}): the second is read as a separate run "
                      f"and pooled -- split the input")
                k = k + (f,)
            res[k] = pace(k, ants)
            files_of[k].append(f)
    keys = sorted(res)
    nf = len(files)
    print(f"trailpace: {len(keys)} (seed,gap,arm,tag) keys from {nf} files")
    for i, d in enumerate(("seed", "gap", "arm", "tag")):
        vals = sorted({k[i] for k in keys})
        print(f"           {d:<5} {len(vals):>3}  {vals if len(vals) <= 12 else f'[{vals[0]}..{vals[-1]}]'}")
    print(f"           columns: trip_load in {has_all['trip_load']}/{nf} files, bite_x {has_all['bite_x']}/{nf},"
          f" since_trip {has_all['since_trip']}/{nf}")
    ticks = Counter(res[k]["tickf"] for k in keys)
    print(f"           frames per decision row (modal gap between an ant's rows): {dict(ticks)}")
    if not has_all["trip_load"]:
        print("trailpace: no trip_load column -- bookings, ages, dwell and road speed are all unavailable. Rerun with a "
              "trailfollow from 2026-09-29 or later.")
        return 1
    if not has_all["bite_x"]:
        print("           no bite_x: pile returns cannot be told from the rest, and the age estimate has no pickups to count from")
    have_st = has_all["since_trip"] > 0
    if not have_st:
        print("           no since_trip: the cargo's age is ESTIMATED from frames since the trip's last pickup (age~)")

    agecol = "age" if have_st else "age~"
    print(f"\n{'seed':>4} {'gap':>4} {'arm/tag':<12} {'books':>6} {'pile':>5} {agecol + ' med':>8} {'p90':>6} {'net/tick':>9}"
          f" {'gross':>6} {'transit':>8} {'pile iv med':>11} {'p75':>6} {'p90':>6} {'dwell med':>9} {'p90':>5}")
    groups = defaultdict(list)
    for k in keys:
        R = res[k]
        s = summary([R])
        ages = s["age"] if have_st else s["est"]
        print(f"{k[0]:>4} {k[1]:>4} {k[2] + '/' + k[3]:<12} {s['n']:>6} {s['n_pile']:>5} {fmt(med(ages)):>8} {fmt(q(ages, .9), '.0f'):>6}"
              f" {fmt(s['net'], '.3f'):>9} {fmt(s['gross'], '.3f'):>6} {fmt(med(s['transit']), '.3f'):>8}"
              f" {fmt(med(s['iv_pile']), '.0f'):>11} {fmt(q(s['iv_pile'], .75), '.0f'):>6} {fmt(q(s['iv_pile'], .9), '.0f'):>6}"
              f" {fmt(med(s['dwell_s'])):>9} {fmt(q(s['dwell_s'], .9), '.0f'):>5}")
        groups[(k[1], k[2], k[3])].append(R)

    print("\npooled per (gap, arm, tag):")
    for g, arm, tag in sorted(groups):
        s = summary(groups[(g, arm, tag)])
        ages = s["age"] if have_st else s["est"]
        print(f"\n  gap {g} {arm}/{tag}: {len(groups[(g, arm, tag)])} seeds, {s['n']} booked returns, {s['n_pile']} with food from the pile")
        print(f"    1. cargo age at booking ({'since_trip' if have_st else 'ESTIMATED from frames'}, ticks): median {fmt(med(ages))}"
              f"  p90 {fmt(q(ages, .9), '.0f')}  (n {len(ages)})")
        if have_st and s["diff"]:
            exact = sum(d <= 1 for d in s["diff"]) / len(s["diff"])
            print(f"       the frame estimate against since_trip: within 1 tick on {exact:.0%}, median |diff| {med(s['diff']):.1f}"
                  f" -- the trust an old trace's age~ earns")
        print(f"    2. laden road pace (nest+3 < x < nest+{g - 10}): net {fmt(s['net'], '.3f')} cells/tick toward home,"
              f" gross {fmt(s['gross'], '.3f')}, over {s['rows']} rows; per-trip transit pile->door median"
              f" {fmt(med(s['transit']), '.3f')} (n {len(s['transit'])})")
        for lab, iv in (("all returns", s["iv_all"]), ("pile returns", s["iv_pile"])):
            mean = sum(iv) / len(iv) if iv else math.nan
            print(f"    3. intervals, {lab:<12} (frames): median {fmt(med(iv), '.0f')}  p75 {fmt(q(iv, .75), '.0f')}"
                  f"  p90 {fmt(q(iv, .9), '.0f')}  mean {fmt(mean, '.0f')}  (n {len(iv)})")
        print(f"    4. dwell within 3 columns of the door before booking (ticks): surface median {fmt(med(s['dwell_s']))}"
              f" p90 {fmt(q(s['dwell_s'], .9), '.0f')}; all rows median {fmt(med(s['dwell_a']))} p90 {fmt(q(s['dwell_a'], .9), '.0f')}")
        iv = s["iv_pile"] or s["iv_all"]
        every = sum(iv) / len(iv) if iv else math.nan
        # **speed= is the median trip's pace, not the pooled row pace.** The
        # pooled rows are weighted by time on the road, so the few trips that
        # loiter there own most of them and drag it down (0.51 against 0.72 on
        # the 2026-09-29 pulsed bed); the model's ants are the typical trip,
        # and the check below says whether that trip reproduces the cargo age.
        pace_ = med(s["transit"]) if s["transit"] else s["net"]
        speed = min(1.0, max(0.01, pace_)) if not math.isnan(pace_) else math.nan
        dwell = med(s["dwell_s"])
        a90 = q(ages, .9)
        print(f"    onetrail mode=stream span={g} speed={fmt(speed, '.3f')} every={fmt(every, '.0f')} dwell={fmt(dwell, '.0f')}"
              f" tickframes={s['tickf']}")
        if not math.isnan(speed) and ages:
            print(f"      check: span/speed + dwell = {g}/{speed:.3f} + {fmt(dwell, '.0f')} = {g / speed + (0 if math.isnan(dwell) else dwell):.0f}"
                  f" ticks, against a median cargo age at booking of {med(ages):.0f}{'' if have_st else ' (estimated)'}")
        print(f"      every_median={fmt(med(iv), '.0f')}{'' if s['iv_pile'] else ' (NO pile returns: every= is from all returns)'}"
              f"  every= is the mean: the trail's deposit RATE is set by it, and a bursty stream's median understates the gap")
        if not math.isnan(a90):
            ok = [t for t in TS if deposit(t, a90) >= TRAIL_HALF]
            print(f"      age_p90={a90:.0f}{'' if have_st else ' (estimated)'} -> a single deposit at that age: "
                  + ", ".join(f"T={t} {deposit(t, a90)}" for t in TS)
                  + f" raw; clears TRAIL_HALF={TRAIL_HALF} at T >= {min(ok) if ok else 'none of ' + str(TS)}")
    return 0


# --- controls ------------------------------------------------------------


def _fixture(path, with_st=True, seeds=(1, 2)):
    """A colony whose answers are known. Returns what it must read back."""
    head = BASE + ["y2", "fill", "trip_load", "bite_x", "bite_y"] + (["since_trip"] if with_st else [])
    nx, gap, surf, tickf = 50, 90, 95, 6
    rows, want = [], dict(ages=[], dwell=[], every=300, net=0, gross=0, road=0)
    for seed in seeds:
        for trip in range(12):
            aid = 1 + trip          # one ant per trip, so no ant is in two places at once
            x, t = nx + gap + 1, 0
            # Pick up at the pile, then walk home at exactly 0.5 cells a tick:
            # one step every second row, a dwell of 4 + trip % 3 rows at the door.
            recs = [(x, x, "empty", nx + gap + 2)]    # the pickup row: trip_load set after act
            while x > nx:
                x2 = x - 1 if t % 2 else x
                recs.append((x, x2, "laden", None))
                x = x2
                t += 1
                # Odd trips turn back for two rows at mid-road: net and gross
                # pace then differ, so a readout that confused them would show.
                if trip % 2 and x == nx + 40 and t % 2 == 0:
                    recs += [(x, x + 1, "laden", None), (x + 1, x, "laden", None)]
            for _ in range(4 + trip % 3):
                recs.append((x, x, "laden", None))
            # Every fourth trip tops up at the pile on its fourth row, which
            # restarts `since_trip`: the age is from the LAST pickup.
            top = 3 if trip % 4 == 0 else 0
            if top:
                recs[top] = (recs[top][0], recs[top][1], recs[top][2], nx + gap + 3)
            n = len(recs)
            # The walk's length varies with the dwell, so the start is set back
            # by it: every booking lands exactly 300 frames after the last.
            start = 3000 + 300 * trip - n * tickf
            for i, (xx, x2, leg, bx) in enumerate(recs):
                rows.append(dict(seed=seed, gap=gap, arm="self", tag="t", frame=start + i * tickf, id=aid, leg=leg, x=xx,
                                 y=surf, x2=x2, at_nest="1.0000" if abs(xx - nx) <= 3 else "0.0000", nest_x=nx, y2=surf,
                                 fill="0.5", trip_load=1, bite_x="-" if bx is None else bx, bite_y="-" if bx is None else surf,
                                 since_trip=i - top if i >= top else i))
            rows.append(dict(seed=seed, gap=gap, arm="self", tag="t", frame=start + n * tickf, id=aid, leg="empty", x=nx, y=surf,
                             x2=nx, at_nest="1.0000", nest_x=nx, y2=surf, fill="0", trip_load=0, bite_x="-", bite_y="-", since_trip=n - top))
            want["ages"].append(n - top)
            for xx, x2, leg, _ in recs:
                if leg == "laden" and nx + 3 < xx < nx + gap - 10:
                    want["net"] += xx - x2
                    want["gross"] += abs(x2 - xx)
                    want["road"] += 1
            want["dwell"].append(sum(1 for r in recs if abs(r[0] - nx) <= 3))
            # A crop emptied on the road clears trip_load away from the nest:
            # that is not a booked return, and must not be read as one.
            if trip == 5:
                for i, tl in enumerate((1, 1, 0)):
                    rows.append(dict(seed=seed, gap=gap, arm="self", tag="t", frame=start + 6 * i, id=200 + aid,
                                     leg="laden" if tl else "empty", x=nx + 40, y=surf, x2=nx + 40, at_nest="0.0000",
                                     nest_x=nx, y2=surf, fill="0", trip_load=tl, bite_x="-", bite_y="-", since_trip=i))
                want["road"] += 2    # its two laden rows stand on the road: pace 0, and they count
            # Empty rows away from the door, so the surface is defined.
            for j in range(3):
                rows.append(dict(seed=seed, gap=gap, arm="self", tag="t", frame=start + 6 * j, id=100 + aid, leg="empty",
                                 x=nx + 20 + j, y=surf, x2=nx + 20 + j, at_nest="0.0000", nest_x=nx, y2=surf, fill="0",
                                 trip_load=0, bite_x="-", bite_y="-", since_trip=0))
    rows.sort(key=lambda r: (r["frame"], r["id"]))
    with open(path, "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(head)
        for r in rows:
            w.writerow([r[c] for c in head])
    return want


def selftest():
    try:
        _selftest()
    except AssertionError as e:
        print(f"trailpace selftest: FAIL -- {e}")
        return 1
    return 0


def _selftest():
    tmp = tempfile.mkdtemp(prefix="trailpace-")
    for with_st in (True, False):
        p = os.path.join(tmp, f"fx-{int(with_st)}.csv")
        want = _fixture(p, with_st)
        data, miss, has = read(p)
        assert not miss and has["since_trip"] == with_st
        Rs = [pace(k, a) for k, a in data.items()]
        assert all(R["tickf"] == 6 for R in Rs), f"frames per tick: {[R['tickf'] for R in Rs]}"
        s = summary(Rs)
        ages = s["age"] if with_st else s["est"]
        assert s["n"] == 24 and s["n_pile"] == 24, f"24 pile returns booked, read {s['n']} ({s['n_pile']} pile)"
        assert sorted(ages) == sorted(want["ages"]), f"cargo age: {sorted(ages)[:6]} against {sorted(want['ages'])[:6]}"
        if with_st:
            assert all(d == 0 for d in s["diff"]), f"the frame estimate disagrees with since_trip on a clean trace: {s['diff'][:5]}"
        assert want["net"] < want["gross"], "the control is not constructed: some trips must turn back"
        assert abs(s["net"] - want["net"] / want["road"]) < 1e-9 and abs(s["gross"] - want["gross"] / want["road"]) < 1e-9, \
            f"road pace: net {s['net']:.4f} gross {s['gross']:.4f} against {want['net'] / want['road']:.4f} {want['gross'] / want['road']:.4f}"
        assert abs(s["net"] - 0.5) < 0.02, f"a walk of one cell every second tick must read ~0.5 cells/tick, got {s['net']:.3f}"
        assert q(sorted(ages), 0.9) == sorted(want["ages"])[int(0.9 * len(want["ages"]))], "the p90 of the ages"
        assert abs(med(s["transit"]) - 0.5) < 0.03, f"transit pace: {med(s['transit'])}"
        assert set(s["iv_pile"]) == {300}, f"bookings 300 frames apart: {sorted(set(s['iv_pile']))}"
        assert sorted(s["dwell_s"]) == sorted(want["dwell"]), f"dwell: {sorted(s['dwell_s'])} against {sorted(want['dwell'])}"
    assert q(list(range(1, 11)), 0.9) == 10 and q(list(range(1, 11)), 0.5) == 6 and math.isnan(q([], 0.5)), "q()"
    # The T criterion's arithmetic against `onetrail`'s own first-step deposit.
    assert deposit(32, 0) == 7311 and deposit(16, 100) < TRAIL_HALF <= deposit(48, 100), \
        f"deposit(): {deposit(32, 0)} {deposit(16, 100)} {deposit(48, 100)}"
    # A trace with no trip_load says so and does not crash.
    p = os.path.join(tmp, "bare.csv")
    with open(p, "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(BASE)
        w.writerow([1, 90, "self", "t", 6, 1, "empty", 30, 95, 31, "0.0000", 50])
    so = sys.stdout
    sys.stdout = open(os.devnull, "w")
    try:
        rc = run([p])
    finally:
        sys.stdout.close()
        sys.stdout = so
    assert rc == 1, "a trace without trip_load must say it cannot answer, with a non-zero exit"
    print("trailpace selftest: ok -- a colony built to walk home at 0.5 cells/tick, book every 300 frames, dwell and")
    print("                    carry known ages reads every number back, from since_trip and from the frame estimate;")
    print("                    the deposit arithmetic matches onetrail's; a trace without trip_load says so")


if __name__ == "__main__":
    sys.exit(main())
