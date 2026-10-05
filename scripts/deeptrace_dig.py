#!/usr/bin/env python3
"""The digging pass over a `deeptrace dig=1` run: why the nest ants do or do not
dig, where the soil they cut goes, the rooms it leaves, and every egg.

    python3 scripts/deeptrace_dig.py dig   OUT [OUT...]   # digrows.csv.gz: minutes a run
    python3 scripts/deeptrace_dig.py soil  OUT [OUT...]   # cells.csv.gz
    python3 scripts/deeptrace_dig.py rooms OUT [OUT...]   # nest_f*.txt
    python3 scripts/deeptrace_dig.py brood OUT [OUT...]   # broodlog.csv
    python3 scripts/deeptrace_dig.py journeys OUT [OUT...]  # cells.csv.gz
    python3 scripts/deeptrace_dig.py face  OUT [OUT...]   # digrows.csv.gz with ret_x, ret_y
    python3 scripts/deeptrace_dig.py fed   OUT [OUT...]   # walkrows.csv.gz (`walk=1`), colony.csv, brood.csv

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

`journeys`: every pellet set down after 100k, followed cell by cell to where
it stops, and every cut linked to the pellet its cutter next set down, so one
piece of soil can be followed across trips. A pellet that falls or slides
shows as its cell emptying with no cause and a cell within two filling on the
same frame; the follower chains those, preferring the cell below. Pieces
falling together in a column are interchangeable (only the top cell empties
and the bottom one fills), so inside a column it follows the soil, not the
grain, and a trail ends where no cell within two fills. Prints how much of
what is set down falls again, how much is dug again, what share of cuts take
soil set down before (by the trail, and by the cell having been cut before,
which needs no trail), and how much soil is carried out of the nest against
back in from the mound.

`face`: every cut under the old ground after 100k, followed to the end of the
digger's walk back to it (`dig_return`, recorded as `ret_x`, `ret_y` since
2026-10-05; older runs lack the columns). Each ending is read the way the
engine clears the walk: food in the crop or energy under half its start
(which leave no target) before arrival within two cells, then patience under
0.1; a new cut overwrites the face. Then how often the same ant's next cut is
at the face. Then the same walks split by where the soil was set down --
inside the nest, or carried out over the old ground line -- because a digger
that carries its soil out almost never gets back: how each walk ended (a cut
on the way split by whether the cut cell was in the nest, from cuts.csv) and
where the digger's next five cuts were; for soil carried out, how close the
walk back came to the door cell and whether it got into the nest. Last, of
the diggers that got back, how many arrived touching no ground (`ground8` 0),
and how often the next cut was at the face, workers and others apart.

`fed`: where the ants that can feed a larva are, and why (a `walk=1` run).
Fed means energy over the ant's start, the only ants that nurse or crop-feed
a larva; places are the top five rows under the door, the lane (3 columns
either side of the door) under them, and the room (the rest of the nest).
Prints, after 100k: where fed ants' nest decisions are; the colony census by
zone and the nest's fed ants and larvae by place, with the share of larvae
that have a fed ant's head within 2 cells; P(move) of empty ants in the top
rows by how full they are; the pulls on fed nest decisions; every nest visit
that holds a fed decision, from how it came in to how it went out, and how
soon an ant that came in fed turned hungry; energy given and taken between
two decisions of an ant carrying nothing (steps of 4 J or more are transfers:
shares given or taken, a larva nursed; the rest is upkeep), by place; and, at
a lane cell with a room cell beside it, how often an ant with no pull that
stepped took the room. Built for the question of why fed ants keep to the lane
under the door; numbers in fed-ants-lane-2026-10-05.md.

Written 2026-10-05 for the digging deep dive; the numbers it produced are in
/mnt/project-files/deep-trace/digging-trace-2026-10-05.md, and those from
`journeys` and `face` in soil-journeys-2026-10-05.md beside it.
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


# What the goal box's ground is made of: the names a pellet can land as.
GROUND = {"soil", "packedsoil", "spoil", "sand", "stone", "gravel", "clay"}


def follow(out):
    """Set-down pellets followed to where they stop, and cuts linked to them.

    Returns (nx, gy, pellets, cuts). A pellet: `f0` and `c0` (frame and cell
    it was set down), `who`, `path` [(frame, cell)], `end` (`dug again`,
    `lost`, `still there`), `end_f`, `end_at`, and `by` (the cut that took it
    again). A cut: `f`, `at`, `who`, `prev` (the pellet it took, or None for
    ground nobody had set down), `pellet` (the one its cutter set down next).
    """
    nx, gy = geo(out)
    by_frame = {}
    with gzip.open(f"{out}/cells.csv.gz", "rt") as fh:
        for r in csv.DictReader(fh):
            by_frame.setdefault(int(r["frame"]), []).append(
                (int(r["x"]), int(r["y"]), r["from"], r["to"], r["cause"], int(r["id"] or 0)))
    at, pellets, cuts, last_cut = {}, [], [], {}
    end = max(by_frame) if by_frame else 0
    for f in sorted(by_frame):
        evs = by_frame[f]
        filled = {(x, y): (c, who) for x, y, fr, to, c, who in evs if fr not in GROUND and to in GROUND}
        free = {k for k, (c, _) in filled.items() if c == ""}
        born = [(k, who, last_cut.pop(who, None)) for k, (c, who) in sorted(filled.items()) if c in ("drop", "lift")]
        moves = []
        for x, y, fr, to, c, who in evs:
            if not (fr in GROUND and to not in GROUND):
                continue
            p = at.pop((x, y), None)
            if c == "cut":
                cuts.append(dict(f=f, at=(x, y), who=who, prev=p))
                last_cut[who] = len(cuts) - 1
                if p is not None:
                    pellets[p].update(end="dug again", end_f=f, end_at=(x, y), by=len(cuts) - 1)
            elif p is not None:
                near = [k for k in free if max(abs(k[0] - x), abs(k[1] - y)) <= 2]
                if not near:
                    pellets[p].update(end="lost", end_f=f, end_at=(x, y))
                    continue
                k = min(near, key=lambda k: (k[1] <= y, max(abs(k[0] - x), abs(k[1] - y)), abs(k[0] - x)))
                free.discard(k)
                moves.append((p, k))
        for p, k in moves:
            pellets[p]["path"].append((f, k))
            at[k] = p
        for k, who, src in born:
            pellets.append(dict(f0=f, c0=k, who=who, path=[(f, k)], end=None))
            at[k] = len(pellets) - 1
            if src is not None:
                cuts[src]["pellet"] = len(pellets) - 1
    for k, p in at.items():
        pellets[p].update(end="still there", end_f=end, end_at=k)
    return nx, gy, pellets, cuts


def journeys(out):
    nx, gy, P, C = follow(out)
    inside = lambda c: c[1] > gy
    print(f"== {os.path.basename(out)}: soil set down after 100k, followed cell by cell (nest = under the old ground)")
    for name, here in (("nest", inside), ("mound", lambda c: not inside(c))):
        S = [p for p in P if p["f0"] >= 100000 and here(p["c0"])]
        fell = [p for p in S if len(p["path"]) > 1]
        print(f"  set down in the {name}: {len(S)}; fell again {pct(len(fell), len(S))}"
              f" (median {q([p['path'][1][0] - p['f0'] for p in fell], 0.5)} frames after, {q([len(p['path']) - 1 for p in fell], 0.5)} cells);"
              f" followed to where it stopped {pct(sum(p['end'] != 'lost' for p in S), len(S))};"
              f" dug again {pct(sum(p['end'] == 'dug again' for p in S), len(S))}")
    seen, old = set(), Counter()
    for c in sorted(C, key=lambda c: c["f"]):
        if c["f"] >= 100000:
            old[(inside(c["at"]), c["at"] in seen)] += 1
        seen.add(c["at"])
    for name, flag in (("nest", True), ("mound", False)):
        cs = [c for c in C if c["f"] >= 100000 and inside(c["at"]) == flag]
        print(f"  cuts in the {name}: {len(cs)}; of soil set down before {pct(sum(c['prev'] is not None for c in cs), len(cs))}"
              f" (trail), at a cell cut before {pct(old[(flag, True)], len(cs))} (no trail needed)")
    late = [c for c in C if c["f"] >= 100000 and "pellet" in c]
    out_ = sum(inside(c["at"]) and not inside(P[c["pellet"]]["c0"]) for c in late)
    back = sum(not inside(c["at"]) and inside(P[c["pellet"]]["c0"]) for c in late)
    print(f"  pellets cut in the nest and set down outside it: {out_}; cut outside and set down in the nest: {back}")


def face(out):
    nx, gy = geo(out)
    cheb = lambda a, b: max(abs(a[0] - b[0]), abs(a[1] - b[1]))
    walks, open_, waiting, last_row, cut_zones = [], {}, {}, {}, {}
    # Where each cut was, by the cut cell rather than the head (cuts.csv is
    # written beside digrows.csv.gz by every `dig=1` run).
    with open(f"{out}/cuts.csv") as fh:
        cut_zone_at = {(int(r["frame"]), int(r["id"])): r["zone"] for r in csv.DictReader(fh)}
    with gzip.open(f"{out}/digrows.csv.gz", "rt") as fh:
        col = {k: n for n, k in enumerate(next(fh).rstrip("\n").split(","))}
        if "ret_x" not in col:
            sys.exit(f"{out}: digrows.csv.gz has no ret_x column (recorded before 2026-10-05)")
        F, I, HX, HY, HX2, HY2 = (col[k] for k in ("frame", "id", "hx", "hy", "hx_after", "hy_after"))
        HOLD, E, DIG, DX, DY, RX, RY, PAT = (col[k] for k in ("hold", "energy", "dig", "dig_x", "dig_y", "ret_x", "ret_y", "patience"))
        ZONE, G8, WK = col["zone"], col["ground8"], col["worker"]
        for line in fh:
            a = line.rstrip("\n").split(",")
            f, i = int(a[F]), int(a[I])
            head, head2 = (int(a[HX]), int(a[HY])), (int(a[HX2]), int(a[HY2]))
            hold, energy, pat = a[HOLD], float(a[E]), float(a[PAT])
            ret = (int(a[RX]), int(a[RY])) if a[RX] else None
            w = open_.get(i)

            def close(why):
                w.update(end=why, end_f=f, touching=a[G8] != "0", worker=a[WK] == "1")
                del open_[i]
                waiting[i] = w

            if w is not None and w["phase"] == "walk":
                if ret == w["cut"]:
                    w["prev"] = (head, head2, pat)
                    w["door"] = min(w["door"], cheb(head, (nx, gy)))
                    w["entered"] |= a[ZONE] == "nest"
                elif ret is None:
                    ph, ph2, ppat = w["prev"]
                    # The engine clears the walk for a missing target (food, hunger)
                    # before it asks about arrival, so read those first.
                    if hold == "1":
                        close("food in its crop")
                    elif energy < 0.5:
                        close("too hungry")
                    elif min(cheb(ph, w["cut"]), cheb(ph2, w["cut"]), cheb(head, w["cut"])) <= 2:
                        close("got back to its face")
                    elif ppat < 0.1:
                        close("gave up (patience ran out)")
                    else:
                        close("other")
            elif w is not None and w["phase"] == "carry":
                if hold != "2":
                    w["drop_at"] = head
                    if ret == w["cut"]:
                        w.update(phase="walk", prev=(head, head2, pat), door=cheb(head, (nx, gy)), entered=a[ZONE] == "nest")
                    elif not w["ret_seen"] and w["cut"][1] <= gy:
                        close("cut in the open: no walk back")
                    elif ret is not None:
                        close("face changed while carrying")
                    elif hold == "1":
                        close("food in its crop")
                    elif energy < 0.5:
                        close("too hungry")
                    elif cheb(head, w["cut"]) <= 2:
                        close("set down at its face")
                    else:
                        close("other")
                elif ret == w["cut"]:
                    w["ret_seen"] = True
            if a[DIG] == "cut":
                cut = (int(a[DX]), int(a[DY]))
                zone = cut_zone_at.get((f, i), a[ZONE])
                cut_zones.setdefault(i, []).append((f, zone))
                w = open_.get(i)
                if w is not None:
                    close("cut at its face on the way" if cheb(cut, w["cut"]) <= 2 else "cut somewhere else on the way")
                    w["cut_zone"] = zone
                p = waiting.pop(i, None)
                if p is not None:
                    p["next_cut"] = cut
                w = dict(cut_f=f, cut=cut, phase="carry", ret_seen=False, id=i)
                open_[i] = w
                walks.append(w)
            last_row[i] = f
    end = max(last_row.values()) if last_row else 0
    for i, w in open_.items():
        w["end"] = "still walking at the end" if last_row[i] > end - 100 else "died"
    nest = [w for w in walks if w["cut_f"] >= 100000 and w["cut"][1] > gy]
    n = len(nest)
    print(f"== {os.path.basename(out)}: cuts under the old ground after 100k, by how the walk back to the face ended: {n}")
    ends = Counter(w["end"] for w in nest)
    for k, v in ends.most_common():
        print(f"  {k}: {v} ({pct(v, n)})")
    nc = [w for w in nest if "next_cut" in w]
    print(f"  same ant's next cut within 2 cells of this one: {pct(sum(cheb(w['next_cut'], w['cut']) <= 2 for w in nc), len(nc))} of {len(nc)}")
    back = [w for w in nc if w["end"] in ("got back to its face", "set down at its face")]
    print(f"  ...after getting back to its face: {pct(sum(cheb(w['next_cut'], w['cut']) <= 2 for w in back), len(back))} of {len(back)}")
    # Added 2026-10-05 (soil-journeys-2026-10-05.md, "Soil taken out loses its
    # digger"): the same walks split by where the soil was set down, because a
    # digger that carries its soil out almost never gets back, and that is the
    # number a soil-out rule has to be judged on.
    got = ("got back to its face", "set down at its face", "cut at its face on the way")
    for where, here in (("set it down inside the nest", lambda w: w["drop_at"][1] > gy), ("carried it out", lambda w: w["drop_at"][1] <= gy)):
        s = [w for w in nest if "drop_at" in w and here(w)]
        m = len(s)
        cut_in = sum(w["end"] == "cut somewhere else on the way" and w.get("cut_zone") == "nest" for w in s)
        cut_out = sum(w["end"] == "cut somewhere else on the way" and w.get("cut_zone") != "nest" for w in s)
        print(f"  {where}: {m}; got back {pct(sum(w['end'] in got for w in s), m)}; cut other soil in the nest {pct(cut_in, m)},"
              f" outside it {pct(cut_out, m)}; too hungry {pct(ends_n(s, 'too hungry'), m)}; food {pct(ends_n(s, 'food in its crop'), m)};"
              f" gave up {pct(ends_n(s, 'gave up (patience ran out)'), m)}")
        nxt = Counter()
        for w in s:
            later = [z for f, z in cut_zones.get(w["id"], []) if f > w["cut_f"]][:5]
            nxt.update("nest" if z == "nest" else "mound" if z == "mound_in" else "other" for z in later)
        t = sum(nxt.values())
        print(f"    next five cuts: in the nest {pct(nxt['nest'], t)}, in the mound {pct(nxt['mound'], t)}")
        if where == "carried it out":
            wk = [w for w in s if "door" in w]
            print(f"    walking back: closest to the door cell median {q([w['door'] for w in wk], 0.5)};"
                  f" within 3 of it {pct(sum(w['door'] <= 3 for w in wk), len(wk))}; got into the nest {pct(sum(w['entered'] for w in wk), len(wk))}")
    arr = [w for w in back if "touching" in w]
    tn = [w for w in arr if not w["touching"]]
    print(f"  arriving at its face, touching no ground: {pct(len(tn), len(arr))} of {len(arr)}; next cut at the face:")
    for lab, s in (("workers touching ground", [w for w in arr if w["worker"] and w["touching"]]), ("workers touching none", [w for w in arr if w["worker"] and not w["touching"]]),
                   ("others touching ground", [w for w in arr if not w["worker"] and w["touching"]]), ("others touching none", [w for w in arr if not w["worker"] and not w["touching"]])):
        print(f"    {lab}: {pct(sum(cheb(w['next_cut'], w['cut']) <= 2 for w in s), len(s))} of {len(s)}")


FED_PLACES = (("top", "the top five rows under the door"), ("lane", "the lane under them"), ("room", "the room"))
FED_BANDS = ((0.8, "under 0.8"), (0.9, "0.8-0.9"), (0.95, "0.9-0.95"), (1.0, "0.95-1"), (1.2, "1-1.2"), (9e9, "over 1.2"))
# `creature::DIRS`: E, NE, N, NW, W, SW, S, SE, y down.
FED_DX = (1, 1, 0, -1, -1, -1, 0, 1)
FED_DY = (0, -1, -1, -1, 0, 1, 1, 1)


def fed(out, f0=100000):
    path = f"{out}/walkrows.csv.gz"
    if not os.path.exists(path):
        sys.exit(f"{out}: no walkrows.csv.gz -- record with `deeptrace ants=0 walk=1`")
    nx, gy = geo(out)
    start_j = None
    for e in open(f"{out}/events.txt"):
        p = e.split()
        if len(p) > 1 and p[1] == "FOUNDED":
            start_j = dict(t.split("=") for t in p[2:] if "=" in t).get("start_j")
    if start_j is None:
        print(f"  ({out}: no start_j on the FOUNDED line, recorded before 2026-10-05: reading the census against 200 J, the shipped ant's start)")
    start_j = float(start_j or 200.0)

    def place(x, y):
        if y <= gy:
            return "out"
        if abs(x - nx) <= 3:
            return "top" if y - gy <= 5 else "lane"
        return "room"

    nest_fed = Counter()
    fed_pull = Counter()
    pm, pm_n = Counter(), Counter()
    flow = {}
    junc, junc_room = Counter(), Counter()
    last, vis, visits = {}, {}, []
    with gzip.open(path, "rt") as fh:
        col = {k: i for i, k in enumerate(next(fh).rstrip("\n").split(","))}
        F, I, W, HX, HY, E, LEG, PULL, PM, OPTS, CHOSE = (col[k] for k in ("frame", "id", "worker", "hx", "hy", "e", "leg", "pull", "p_move", "opts", "chose"))
        for line in fh:
            a = line.rstrip("\n").split(",")
            f = int(a[F])
            if f < f0:
                continue
            i = int(a[I])
            x, y = int(a[HX]), int(a[HY])
            e = float(a[E])
            leg = a[LEG]
            pl = place(x, y)
            is_fed = e > 1.0
            prev = last.get(i)
            last[i] = (f, e, pl, leg)
            if prev is not None and f - prev[0] <= 12 and prev[3] == "empty" and leg == "empty":
                d = e - prev[1]
                s = flow.setdefault((prev[2], prev[1] > 1.0), Counter())
                s["frames"] += f - prev[0]
                if d <= -0.02:
                    s["gave"] += -d
                elif d >= 0.02:
                    s["got"] += d
                else:
                    s["upkeep"] += -d
            v = vis.get(i)
            if v is not None and (pl == "out" or f - v["last"] > 60):
                v["out_door"] = pl == "out" and f - v["last"] <= 60
                visits.append(v)
                del vis[i]
                v = None
            if pl == "out":
                continue
            if v is None:
                came = prev is not None and f - prev[0] <= 60 and prev[2] == "out"
                v = vis[i] = dict(start=f, last=f, door=came, e0=e, worker=a[W] == "1", fed_n=0, hungry_at=None)
            v["last"] = f
            v["end"] = ("fed" if is_fed else "hungry", a[PULL])
            if is_fed:
                v["fed_n"] += 1
                nest_fed[pl] += 1
                fed_pull[a[PULL]] += 1
            elif v["e0"] > 1.0 and v["hungry_at"] is None:
                v["hungry_at"] = f - v["start"]
            if pl == "top" and leg == "empty":
                band = next(lab for hi, lab in FED_BANDS if e < hi)
                pm[band] += float(a[PM])
                pm_n[band] += 1
            if pl != "room" and a[PULL] == "none" and a[OPTS] != "" and a[CHOSE] != "":
                opts = int(a[OPTS])
                room = [d for d in range(8) if opts >> d & 1 and place(x + FED_DX[d], y + FED_DY[d]) == "room"]
                if room:
                    junc[is_fed] += 1
                    junc_room[is_fed] += int(a[CHOSE]) in room
    for v in vis.values():
        v["out_door"] = False
        visits.append(v)
    print(f"== {out}: after {f0}, nest x {nx}, old ground y {gy}, start {start_j:.0f} J")
    nf = sum(nest_fed.values())
    print(f"  fed ants' decisions in the nest: in the lane {pct(nest_fed['top'] + nest_fed['lane'], nf)} (in the top five rows {pct(nest_fed['top'], nf)}), in the room {pct(nest_fed['room'], nf)}, of {nf}")
    # the census: every ant and every brood cell every 1,000 frames
    zones = Counter()
    ants = {}
    with open(f"{out}/colony.csv") as fh:
        for r in csv.DictReader(fh):
            fr = int(r["frame"])
            if fr < f0:
                continue
            is_fed = float(r["energy_j"]) > start_j
            zones[(r["zone"], is_fed)] += 1
            ants.setdefault(fr, []).append((int(r["hx"]), int(r["hy"]), is_fed))
    larvae = {}
    with open(f"{out}/brood.csv") as fh:
        for r in csv.DictReader(fh):
            fr = int(r["frame"])
            if fr >= f0 and r["stage"] == "larva":
                larvae.setdefault(fr, []).append((int(r["x"]), int(r["y"])))
    frames = sorted(set(ants) & set(larvae))
    n = max(len(frames), 1)
    fed_at, lv, near = Counter(), Counter(), Counter()
    for fr in frames:
        # Counted per ant, not per cell: ants stand on nestmates as riders, so
        # a head cell can hold two (about 1 decision in 10 round the door).
        fed_heads = [(x, y) for x, y, fd in ants[fr] if fd]
        for x, y in fed_heads:
            fed_at[place(x, y)] += 1
        fed_cells = set(fed_heads)
        for x, y in larvae[fr]:
            pl = place(x, y)
            lv[pl] += 1
            near[pl] += any((x + dx, y + dy) in fed_cells for dx in range(-2, 3) for dy in range(-2, 3))
    nframes = len({fr for fr in ants})
    print("  census, per sample: fed by zone " + ", ".join(f"{z} {c / max(nframes, 1):.1f}" for (z, fd), c in sorted(zones.items(), key=lambda kv: -kv[1]) if fd) +
          "; hungry in the nest " + f"{zones[('nest', False)] / max(nframes, 1):.1f}")
    for key, lab in FED_PLACES:
        print(f"    {lab}: fed ants {fed_at[key] / n:.1f}, larvae {lv[key] / n:.1f}, larvae with a fed ant's head within 2 cells {pct(near[key], lv[key])}")
    print("  P(move), empty ants in the top five rows, by energy over start: " + ", ".join(f"{lab} {pm[lab] / pm_n[lab]:.2f}" for _, lab in FED_BANDS if pm_n[lab] >= 100))
    print("  pulls on fed decisions in the nest: " + ", ".join(f"{k} {pct(c, nf)}" for k, c in fed_pull.most_common(6)))
    fv = [v for v in visits if v["fed_n"] > 0]
    for wk, lab in ((False, "foragers"), (True, "nest workers")):
        s = [v for v in fv if v["worker"] == wk]
        if not s:
            continue
        came_fed = [v for v in s if v["e0"] > 1.0]
        turned = [v["hungry_at"] for v in came_fed if v["hungry_at"] is not None]
        ends = Counter(f"{v['end'][0]}, {v['end'][1]}" for v in s if v["out_door"])
        print(f"  {lab}' nest visits with a fed decision: {len(s)}; came in by the door {pct(sum(v['door'] for v in s), len(s))}, fed on entry {pct(len(came_fed), len(s))},"
              f" left by the door {pct(sum(v['out_door'] for v in s), len(s))}, median {q([v['last'] - v['start'] for v in s], 0.5)} frames;"
              f" came in fed and turned hungry inside {pct(len(turned), len(came_fed))}, median {q(turned, 0.5)} frames after coming in")
        print("    left by the door: " + ", ".join(f"{k} {pct(c, len(s))}" for k, c in ends.most_common(4)))
    print("  between two decisions of an ant carrying nothing, per 1,000 ant-frames (1.0 = its start):")
    for key, lab in FED_PLACES:
        fd, hu = flow.get((key, True), Counter()), flow.get((key, False), Counter())
        kf, kh = fd["frames"] / 1000.0, hu["frames"] / 1000.0
        if kf >= 1 and kh >= 1:
            print(f"    {lab}: fed ants gave {fd['gave'] / kf:.2f} against upkeep {fd['upkeep'] / kf:.2f}; hungry ants took in {hu['got'] / kh:.2f} and gave {hu['gave'] / kh:.2f}")
    print(f"  at a lane cell with a room cell beside it, an ant with no pull that stepped took the room: fed {pct(junc_room[True], junc[True])} of {junc[True]}, hungry {pct(junc_room[False], junc[False])} of {junc[False]}")


def ends_n(ws, end):
    return sum(w["end"] == end for w in ws)


if __name__ == "__main__":
    if len(sys.argv) < 3 or sys.argv[1] not in ("dig", "soil", "rooms", "brood", "journeys", "face", "fed"):
        sys.exit(__doc__)
    for out in sys.argv[2:]:
        globals()[sys.argv[1]](out)
