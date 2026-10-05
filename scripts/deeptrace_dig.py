#!/usr/bin/env python3
"""The digging pass over a `deeptrace dig=1` run: why the nest ants do or do not
dig, where the soil they cut goes, the rooms it leaves, and every egg.

    python3 scripts/deeptrace_dig.py dig   OUT [OUT...]   # digrows.csv.gz: minutes a run
    python3 scripts/deeptrace_dig.py soil  OUT [OUT...]   # cells.csv.gz
    python3 scripts/deeptrace_dig.py rooms OUT [OUT...]   # nest_f*.txt
    python3 scripts/deeptrace_dig.py brood OUT [OUT...]   # broodlog.csv

`dig`: every turn taken by an ant standing in the nest (under the old ground
line), by how far its dig got -- holding soil or food first, then the trace's
`creature::DigWhy` verdict, with "nothing to cut" split by what the jaw met --
in three windows. Then every cell cut in the nest, followed to the frame its
digger let go of the pellet: where the digger stood then, and whether it was
under the lean line (energy below 0.5 of start, so LEAN_FORAGE's drop). A
pellet let go in the nest is soil put back. Last, how far each digger's next
nest cut is from its last: a tunnel is carried forward only when it is close.

`soil`: per 10,000 frames, cells cut out under the old ground line and pellets
set down there, and the put-back rate after 100k (pellets per 10 cells cut).

`rooms`: open space under the old ground line, the deepest open row, and the
rooms: a room cell is open with at least 7 of its 3x3 open (nestgoal's chamber
rule), a room is a 4-connected piece of them, and every room of 30+ cells other
than the largest is listed with how many steps of open path it is from the
largest (a lobe at 3 or under, a separate room down a passage beyond). Brood
pockets under 30 cells are not rooms.

`brood`: every egg laid after 20k, from its laying to its end. Brood an ant
walks over is lifted and put back in the same cell (PUSH_PAST), so the log
shows it `gone` then `seen`: that gap is time under a walker, not a move. A
move is any change of cell between two sightings: straight down is a fall,
anything else a carry.

Written 2026-10-05 for the digging deep dive; the numbers it produced are in
/mnt/project-files/deep-trace/digging-trace-2026-10-05.md.
"""
import csv
import glob
import gzip
import os
import sys
from collections import Counter, deque


def geo(out):
    for e in open(f"{out}/events.txt"):
        p = e.split()
        if len(p) > 1 and p[1] == "FOUNDED":
            kv = dict(t.split("=") for t in p[2:] if "=" in t)
            return int(kv["nest_x"]), int(kv["ground_y"])
    sys.exit(f"{out}: no FOUNDED line in events.txt (run still writing?)")


def q(xs, p):
    if not xs:
        return float("nan")
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))]


def pct(a, b):
    return f"{100 * a / b:.0f}%" if b else "-"


WINDOWS = [(6000, 50000, "6-50k"), (50000, 100000, "50-100k"), (100000, 10**9, "100k-end")]
FUNNEL = ["lean", "soil", "food", "not_asked", "roll_lost", "met_brood", "met_ant", "met_air", "met_other", "cue", "roof", "cut"]
LABEL = {
    "lean": "too hungry", "soil": "holding soil", "food": "food in crop", "not_asked": "not asked",
    "roll_lost": "lost the roll", "met_brood": "jaw met brood", "met_ant": "jaw met an ant", "met_air": "jaw met air",
    "met_other": "jaw met other", "cue": "heap cue", "roof": "roof", "cut": "cut",
}


def dig(out):
    funnel = {w: Counter() for _, _, w in WINDOWS}
    carrying = {}  # id -> cut frame
    last_cut = {}  # id -> (frame, x, y)
    let_go = {w: Counter() for w in ("early", "late")}
    next_cut = {w: [] for w in ("early", "late")}
    with gzip.open(f"{out}/digrows.csv.gz", "rt") as fh:
        for r in csv.DictReader(fh):
            f = int(r["frame"])
            i = r["id"]
            hold = r["hold"]
            if i in carrying and hold != "2":
                cf = carrying.pop(i)
                if cf >= 6000:
                    let_go["early" if cf < 100000 else "late"][(r["zone"], float(r["energy"]) < 0.5)] += 1
            if r["zone"] == "nest":
                w = next((n for a, b, n in WINDOWS if a <= f < b), None)
                if w:
                    if hold == "2":
                        k = "soil"
                    elif hold == "1":
                        k = "food"
                    elif r["dig"] == "no_ground":
                        k = "met_" + {"brood": "brood", "ant": "ant", "empty": "air"}.get(r["dig_mat"], "other")
                    else:
                        k = r["dig"]
                    funnel[w][k] += 1
            if r["dig"] == "cut":
                if r["zone"] == "nest":
                    x, y = int(r["dig_x"]), int(r["dig_y"])
                    if i in last_cut and f >= 6000:
                        pf, px, py = last_cut[i]
                        next_cut["early" if f < 100000 else "late"].append(max(abs(x - px), abs(y - py)))
                    last_cut[i] = (f, x, y)
                    carrying[i] = f
                else:
                    carrying.pop(i, None)
    print(f"== {os.path.basename(out)}: turns taken in the nest, by how far the dig got")
    for _, _, w in WINDOWS:
        c = funnel[w]
        n = sum(c.values())
        print(f"  {w:>8} n={n}: " + "  ".join(f"{LABEL[k]} {100 * c[k] / max(1, n):.1f}%" for k in FUNNEL if c[k]))
    print("  pellets cut in the nest, where the digger let go of them (and the share it was too hungry then):")
    for w, c in let_go.items():
        n = sum(c.values())
        zones = Counter()
        for (z, _), v in c.items():
            zones[z] += v
        print(f"  {w:>8} n={n}: " + "  ".join(f"{z} {pct(v, n)} (hungry {pct(c[(z, True)], v)})" for z, v in zones.most_common()))
    for w, d in next_cut.items():
        print(f"  {w:>8} same ant's next nest cut: {len(d)} pairs, cells away median {q(d, 0.5)}, within 2 cells {pct(sum(v <= 2 for v in d), len(d))}")


def soil(out):
    nx, gy = geo(out)
    cut, drop = Counter(), Counter()
    with gzip.open(f"{out}/cells.csv.gz", "rt") as fh:
        for r in csv.DictReader(fh):
            if int(r["y"]) <= gy:
                continue
            b = int(r["frame"]) // 10000
            if r["cause"] == "cut":
                cut[b] += 1
            elif r["cause"] == "drop":
                drop[b] += 1
    print(f"== {os.path.basename(out)}: under the old ground line, per 10,000 frames (cells cut out / pellets set down)")
    print("  " + "  ".join(f"{b * 10}k {cut[b]}/{drop[b]}" for b in sorted(set(cut) | set(drop))))
    c = sum(v for b, v in cut.items() if b >= 10)
    d = sum(v for b, v in drop.items() if b >= 10)
    print(f"  after 100k: {c} cut, {d} put back, {10 * d / max(1, c):.1f} put back per 10 cut")


OPEN = set("o.aelpf~?b")
BROOD = set("elpb")


def room_census(path, gy):
    with open(path) as fh:
        x0, y0, wd, ht = map(int, fh.readline().split())
        rows = [ln.rstrip("\n") for ln in fh]
    op = [[rows[y][x] in OPEN and y0 + y > gy for x in range(wd)] for y in range(ht)]
    room = [[False] * wd for _ in range(ht)]
    for y in range(1, ht - 1):
        for x in range(1, wd - 1):
            if op[y][x] and sum(op[y + dy][x + dx] for dy in (-1, 0, 1) for dx in (-1, 0, 1)) >= 7:
                room[y][x] = True
    seen = [[False] * wd for _ in range(ht)]
    comps = []
    for y in range(ht):
        for x in range(wd):
            if room[y][x] and not seen[y][x]:
                seen[y][x] = True
                stack, cells = [(x, y)], []
                while stack:
                    cx, cy = stack.pop()
                    cells.append((cx, cy))
                    for nx, ny in ((cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)):
                        if 0 <= nx < wd and 0 <= ny < ht and room[ny][nx] and not seen[ny][nx]:
                            seen[ny][nx] = True
                            stack.append((nx, ny))
                comps.append(cells)
    comps.sort(key=len, reverse=True)
    opened = sum(map(sum, op))
    deepest = max((y0 + y - gy for y in range(ht) for x in range(wd) if op[y][x]), default=0)
    brood = sum(rows[y][x] in BROOD for y in range(ht) for x in range(wd) if y0 + y > gy)
    if not comps:
        return opened, deepest, brood, None, []
    dist = {c: 0 for c in comps[0]}
    queue = deque(comps[0])
    while queue:
        cx, cy = queue.popleft()
        if dist[(cx, cy)] >= 60:
            continue
        for nx, ny in ((cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)):
            if 0 <= nx < wd and 0 <= ny < ht and op[ny][nx] and (nx, ny) not in dist:
                dist[(nx, ny)] = dist[(cx, cy)] + 1
                queue.append((nx, ny))
    others = [
        (len(c), min(dist.get(p, 999) for p in c), sum(rows[cy][cx] in BROOD for cx, cy in c)) for c in comps[1:] if len(c) >= 30
    ]
    big = (len(comps[0]), sum(rows[cy][cx] in BROOD for cx, cy in comps[0]))
    return opened, deepest, brood, big, others


def rooms(out):
    _, gy = geo(out)
    print(f"== {os.path.basename(out)}: frame, open under ground, deepest row, brood under ground | biggest room cells/brood | other rooms of 30+ cells: cells/steps away/brood")
    for path in sorted(glob.glob(f"{out}/nest_f*.txt")):
        f = int(os.path.basename(path)[6:12])
        if f % 10000:
            continue
        opened, deepest, brood, big, others = room_census(path, gy)
        b = f"{big[0]}/{big[1]}" if big else "-"
        print(f"  {f:6d} {opened:5d} {deepest:4d} {brood:5d} | {b:>9} | " + " ".join(f"{c}/{s}/{br}" for c, s, br in others))


def brood(out):
    nx, gy = geo(out)
    first, pos, fell, under, hid_at, last = {}, {}, Counter(), Counter(), {}, {}
    moves = Counter()
    end = 0
    for r in csv.DictReader(open(f"{out}/broodlog.csv")):
        f, i, ev = int(r["frame"]), r["id"], r["event"]
        x, y = int(r["x"]), int(r["y"])
        end = f
        if i not in first:
            first[i] = dict(f=f, ev=ev, x=x, y=y, zone=r["parent_zone"],
                            px=int(r["parent_x"]) if r["parent_x"] else None,
                            py=int(r["parent_y"]) if r["parent_y"] else None,
                            pe=float(r["parent_energy"]) if r["parent_energy"] else None)
            pos[i] = (x, y)
        elif ev in ("laid", "seen", "moved"):
            if i in hid_at:
                under[i] += f - hid_at.pop(i)
            px, py = pos[i]
            if (x, y) != (px, py):
                if x == px and y > py:
                    fell[i] += y - py
                else:
                    moves["up" if y < py else "sideways" if y == py else "down and across"] += 1
            pos[i] = (x, y)
        elif ev == "gone":
            hid_at[i] = f
        last[i] = (ev, f, r["stage"])
    laid = {i: v for i, v in first.items() if v["ev"] == "laid" and v["f"] >= 20000}
    n = len(laid)
    print(f"== {os.path.basename(out)}: {n} eggs laid after 20k")
    if not n:
        return
    zones = Counter(v["zone"] for v in laid.values())
    print("  layer's head in: " + ", ".join(f"{z or '?'} {pct(c, n)}" for z, c in zones.most_common()))
    lx = [abs(v["px"] - nx) for v in laid.values() if v["px"] is not None]
    ly = [gy - v["py"] for v in laid.values() if v["py"] is not None]
    le = [v["pe"] for v in laid.values() if v["pe"] is not None]
    print(f"  layer's head: within 4 columns of the door {pct(sum(d <= 4 for d in lx), len(lx))}, rows above the old ground median {q(ly, 0.5)}; energy after laying median {q(le, 0.5):.0f} J")
    ed = [v["y"] - gy for v in laid.values()]
    ex = [abs(v["x"] - nx) for v in laid.values()]
    print(f"  egg put down: rows under the old ground median {q(ed, 0.5)}, within 4 columns of the door {pct(sum(d <= 4 for d in ex), n)}")
    fl = [fell[i] for i in laid]
    print(f"  fell over its life: none {pct(sum(v == 0 for v in fl), n)}, 5+ rows {pct(sum(v >= 5 for v in fl), n)}, most {max(fl)}")
    print(f"  moves other than straight down, all brood, whole run: {sum(moves.values())} {dict(moves)}")
    life = {i: last[i][1] - laid[i]["f"] for i in laid}
    share = [under[i] / life[i] for i in laid if life[i] > 1000]
    print(f"  share of its life with an ant standing on it (living 1,000+ frames): median {pct(q(share, 0.5), 1)}, p90 {pct(q(share, 0.9), 1)}")
    ended = Counter(last[i][2] for i in laid if last[i][0] == "gone" and last[i][1] < end)
    m = sum(ended.values())
    print("  of those that ended, last seen as: " + ", ".join(f"{s} {pct(c, m)}" for s, c in ended.most_common()))


if __name__ == "__main__":
    if len(sys.argv) < 3 or sys.argv[1] not in ("dig", "soil", "rooms", "brood"):
        sys.exit(__doc__)
    for out in sys.argv[2:]:
        globals()[sys.argv[1]](out)
