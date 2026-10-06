#!/usr/bin/env python3
"""The hunger pass over a `deeptrace hungry=1` run: why hungry ants above
ground do or do not walk to the food, read from every decision of every
hungry ant (`hungry.csv.gz`) and every ant's life (`ledger.csv`).

    python3 scripts/deeptrace_hunger.py funnel  OUT [OUT...] [--from F]
    python3 scripts/deeptrace_hunger.py walk    OUT [OUT...] [--from F] [--to F]
    python3 scripts/deeptrace_hunger.py starved OUT [OUT...] [--from F]

A *spell* is one ant's unbroken run of hungry decisions (energy under its
start grant). A gap of more than 25 frames between two of its rows means it
was fed, and starts a new spell. Ids are slots reused after a death, so an
ant is its id and its birth frame. The door, the food and the ground come
from the run's own `FOUNDED` line; "toward the food" is whichever side of the
door the food is on. Grown means not nest-bound.

`funnel`: every spell of a grown ant that starts on the spoil mound (`--from`,
default 20,000), booked once at the furthest stage it reached, in order:
sent out (its scout weight was on at least once), went toward the food (6 or
more columns past the door on the food's side), reached the food (zone
`food`), took food there (a pick-up within 12 columns of the food). Counts,
`of prev` and `of all`, how the spells that stopped at each stage ended
(refed, died, still hungry at the run's end), and where refed spells ended.
Almost every spell ends fed where the ant stands; who fed it is not in the
recording.

`walk`: every step a hungry, empty ant above ground took with no other pull
(frames `--from` 100,000 to `--to` 200,000), split by which side of its home
point it stood, west or east (the anchor, its last nest contact; within a
column of it is left out), whether its scout had given up, and whether the trail under the
heading it picked was lit (`route` over 0.5). Each line is the share of steps
toward home, away from it, and up or down. Then where those ants stood (west
of the door, at it, east) and how many were under half their grant.

`starved`: every grown ant that starved above ground after `--from`, in its
last hunger: where it died, whether it crossed the door's column, whether its
home point moved (it touched the nest) or it went below the old ground, whether
it ever foraged or reached the food, and the walk split over its own steps.

Added 2026-10-05 with the recorder, for the cost of the nurses switch: under
`NURSE_STAY` grown ants starved on the mound because the walk sends a hungry
ant away from home, which behind the door is away from the food, and a trail
under a given-up ant holds it there
(`/mnt/project-files/deep-trace/mound-hunger-2026-10-05.md`).
"""
import collections
import csv
import gzip
import math
import re
import statistics
import sys

GAP = 25          # frames between two hungry rows that end a spell
PAST_DOOR = 6     # columns past the door that count as "went toward the food"
FOOD_HALF = 12    # the food zone's half width, as the recorder's `zone` reads it
DOOR_HALF = 3     # columns either side of the door that count as crossing it
DX = [1, 1, 0, -1, -1, -1, 0, 1]   # creature::DIRS: E SE S SW W NW N NE
STAGES = ["hungry on mound", "sent out", "went toward food", "reached food", "took food there"]


def num(s):
    try:
        return float(s) if s != "" else math.nan
    except ValueError:
        return math.nan


def geo(out):
    with open(f"{out}/events.txt") as fh:
        for line in fh:
            m = re.search(r"FOUNDED nest_x=(-?\d+) food_x=(-?\d+) ground_y=(-?\d+)", line)
            if m:
                return int(m[1]), int(m[2]), int(m[3])
    sys.exit(f"{out}: no FOUNDED line in events.txt")


def rows(out):
    """`hungry.csv.gz` row by row; stops cleanly at a last row cut short by a
    run still writing. The recorder before 2026-10-05's fix booked a dying
    ant's last row to a fresh life born at frame 0: those are skipped (no ant
    in a lab box is born at 0)."""
    try:
        with gzip.open(f"{out}/hungry.csv.gz", "rt") as fh:
            for r in csv.DictReader(fh):
                if r.get("bite") is None:
                    return
                if int(r["frame"]) == int(r["age"]):
                    continue
                yield r
    except (EOFError, OSError):
        return


def ledger(out):
    with open(f"{out}/ledger.csv") as fh:
        return {(int(r["id"]), int(r["born"])): r for r in csv.DictReader(fh)}


def spells(out, g):
    """Every hunger spell in the run, with how it ended."""
    nest_x, food_x, _ = g
    sgn = 1 if food_x >= nest_x else -1
    led = ledger(out)
    done, cur = [], {}
    for r in rows(out):
        f = int(r["frame"])
        k = (int(r["id"]), f - int(r["age"]))
        sp = cur.get(k)
        if sp is not None and f - sp["last"] > GAP:
            done.append(sp)
            sp = None
        if sp is None:
            sp = cur[k] = dict(id=k, start=f, last=f, zone0=r["zone"], x0=int(r["hx"]), nb=r["nb"] == "1",
                               sent=False, toward=False, food=False, took=False, nest=False,
                               xmin=10**9, xmax=-(10**9), anchors=set())
        sp["last"] = f
        x = int(r["hx"])
        sp["xmin"], sp["xmax"] = min(sp["xmin"], x), max(sp["xmax"], x)
        sp["x_last"], sp["y_last"], sp["zone_last"] = x, int(r["hy"]), r["zone"]
        sp["anchors"].add((r["anchor_x"], r["anchor_y"]))
        sp["sent"] |= num(r["scout_w"]) > 0
        sp["toward"] |= (x - nest_x) * sgn >= PAST_DOOR
        sp["food"] |= r["zone"] == "food"
        sp["nest"] |= r["zone"] == "nest"
        if r["bite"] and abs(int(r["bite"].split(":")[0]) - food_x) <= FOOD_HALF:
            sp["took"] = True
    done.extend(cur.values())
    last = {}
    for sp in done:
        if sp["id"] not in last or sp["start"] > last[sp["id"]]["start"]:
            last[sp["id"]] = sp
    run_end = max((int(r["end"]) for r in led.values()), default=0)
    for sp in done:
        L = led.get(sp["id"])
        sp["ledger"] = L
        sp["cause"] = sp["zone_end"] = ""
        if last[sp["id"]] is sp and L is not None and L["died"] == "1" and int(L["end"]) - sp["last"] <= GAP:
            sp["end"], sp["cause"], sp["zone_end"] = "died", L["cause"], L["zone_end"]
        elif last[sp["id"]] is sp and run_end - sp["last"] <= GAP:
            sp["end"] = "run end"
        else:
            sp["end"] = "refed"
    return done


def stage(sp):
    s = 0
    for ok in (sp["sent"], sp["toward"], sp["food"], sp["took"]):
        if not ok:
            break
        s += 1
    return s


def flags(a, name, default):
    return int(a[a.index(name) + 1]) if name in a else default


def funnel(outs, a):
    f0 = flags(a, "--from", 20_000)
    for out in outs:
        g = geo(out)
        nest_x, _, ground = g
        mound = [sp for sp in spells(out, g) if sp["start"] >= f0 and not sp["nb"] and sp["zone0"].startswith("mound")]
        n0 = len(mound) or 1
        print(f"\n=== {out}: {len(mound)} hunger spells of grown ants starting on the mound, from frame {f0}")
        print(f"{'stage':<18}{'n':>7}{'of prev':>9}{'of all':>8}   stopped here: refed / died / run end")
        prev = n0
        for s, name in enumerate(STAGES):
            n = sum(1 for sp in mound if stage(sp) >= s)
            stop = collections.Counter(sp["end"] for sp in mound if stage(sp) == s)
            print(f"{name:<18}{n:>7}{100 * n / prev:>8.1f}%{100 * n / n0:>7.1f}%   "
                  f"{stop['refed']} / {stop['died']} / {stop['run end']}")
            prev = n or 1

        def where(sp):
            if sp["zone_last"] == "food":
                return "food"
            if sp["zone_last"] == "nest":
                return "nest"
            if abs(sp["x_last"] - nest_x) <= PAST_DOOR and sp["y_last"] >= ground - 10:
                return "by the door"
            return "mound" if sp["zone_last"].startswith("mound") else "open ground"

        ends = collections.Counter(where(sp) for sp in mound if sp["end"] == "refed")
        print("  refed where: " + ", ".join(f"{k} {v}" for k, v in ends.most_common()))


def split(rs, c):
    """Book one row of the walk split into `c`: side of the home point, scout
    state, trail, and the step's sign against home."""
    if r_pull_free(rs):
        hx, ax = int(rs["hx"]), int(rs["anchor_x"])
        if abs(ax - hx) <= 1:
            return
        side = "west of home" if ax > hx else "east of home"
        mode = "given up" if rs["scout_home"] == "1" else ("scouting" if num(rs["scout_w"]) > 0 else "no scout")
        lit = "on a trail" if num(rs["route"]) > 0.5 else "off a trail"
        dx = DX[int(rs["chose"])]
        home = 1 if ax > hx else -1
        c[(side, mode, lit, "home" if dx * home > 0 else ("away" if dx else "vertical"))] += 1


def r_pull_free(r):
    return r["pull"] == "none" and r["chose"] != "" and r["leg"] == "empty"


def print_split(c, indent="  "):
    for side in ("west of home", "east of home"):
        for mode in ("scouting", "given up"):
            for lit in ("on a trail", "off a trail"):
                h, w, v = (c[(side, mode, lit, s)] for s in ("home", "away", "vertical"))
                t = h + w + v
                if t:
                    print(f"{indent}{side:<12} {mode:<9} {lit:<11} {t:>8}: home {100 * h / t:3.0f}%  away {100 * w / t:3.0f}%  up/down {100 * v / t:3.0f}%")


def walk(outs, a):
    f0, f1 = flags(a, "--from", 100_000), flags(a, "--to", 200_000)
    for out in outs:
        nest_x, food_x, _ = geo(out)
        c, where = collections.Counter(), collections.Counter()
        for r in rows(out):
            f = int(r["frame"])
            if f < f0:
                continue
            if f >= f1:
                break
            if r["nb"] == "1" or r["zone"] == "nest":
                continue
            x = int(r["hx"])
            where["west" if x < nest_x - PAST_DOOR else ("east" if x >= nest_x + PAST_DOOR else "door")] += 1
            where["lean"] += num(r["e"]) < 0.5
            split(r, c)
        n = where["west"] + where["door"] + where["east"] or 1
        print(f"\n=== {out}, frames {f0}-{f1} (the food is {'east' if food_x >= nest_x else 'west'} of the door): {n} hungry "
              f"decisions of grown ants above ground: west of the door {100 * where['west'] / n:.0f}%, at it "
              f"{100 * where['door'] / n:.0f}%, east {100 * where['east'] / n:.0f}%; under half their grant {100 * where['lean'] / n:.0f}%")
        print("  steps of empty ants with no other pull, by the side of their home point they stood on:")
        print_split(c)


def starved(outs, a):
    f0 = flags(a, "--from", 20_000)
    for out in outs:
        g = geo(out)
        nest_x, food_x, _ = g
        dead = {sp["id"]: sp for sp in spells(out, g)
                if sp["end"] == "died" and sp["cause"] == "STARVED" and sp["zone_end"] != "nest" and not sp["nb"] and sp["last"] >= f0}
        n = len(dead)
        print(f"\n=== {out}: {n} grown ants starved above ground from frame {f0}")
        if not n:
            continue
        sps = list(dead.values())
        cnt = lambda p: sum(1 for sp in sps if p(sp))
        print(f"  died west of the door {cnt(lambda s: s['x_last'] < nest_x - PAST_DOOR)}, "
              f"east {cnt(lambda s: s['x_last'] >= nest_x + PAST_DOOR)} (the food is {'east' if food_x >= nest_x else 'west'})")
        print(f"  in their last hunger (median {statistics.median(s['last'] - s['start'] for s in sps):.0f} frames): "
              f"crossed the door's column {cnt(lambda s: s['xmin'] <= nest_x + DOOR_HALF and s['xmax'] >= nest_x - DOOR_HALF)}, "
              f"home point moved {cnt(lambda s: len(s['anchors']) > 1)}, went below the old ground {cnt(lambda s: s['nest'])}, "
              f"reached the food {cnt(lambda s: s['food'])}")
        print(f"  ever foraged {cnt(lambda s: s['ledger']['foraged'] == '1')}, ever at the food {cnt(lambda s: int(s['ledger']['last_heap']) > 0)}")
        c = collections.Counter()
        for r in rows(out):
            f = int(r["frame"])
            sp = dead.get((int(r["id"]), f - int(r["age"])))
            if sp is not None and sp["start"] <= f <= sp["last"]:
                split(r, c)
        print("  their own steps, empty with no other pull:")
        print_split(c, "    ")


if __name__ == "__main__":
    cmds = {"funnel": funnel, "walk": walk, "starved": starved}
    if len(sys.argv) < 3 or sys.argv[1] not in cmds:
        sys.exit(__doc__)
    args = sys.argv[2:]
    outs = [x for i, x in enumerate(args) if not x.startswith("--") and (i == 0 or args[i - 1] not in ("--from", "--to"))]
    cmds[sys.argv[1]](outs, args)
