#!/usr/bin/env python3
"""The digging pass over a `deeptrace dig=1` run: why the nest ants do or do not
dig, where the soil they cut goes, the rooms it leaves, and every egg.

    python3 scripts/deeptrace_dig.py dig   OUT [OUT...]   # digrows.csv.gz: minutes a run
    python3 scripts/deeptrace_dig.py soil  OUT [OUT...]   # cells.csv.gz
    python3 scripts/deeptrace_dig.py rooms OUT [OUT...]   # nest_f*.txt
    python3 scripts/deeptrace_dig.py brood OUT [OUT...]   # broodlog.csv
    python3 scripts/deeptrace_dig.py journeys OUT [OUT...]  # cells.csv.gz
    python3 scripts/deeptrace_dig.py face  OUT [OUT...]   # digrows.csv.gz with ret_x, ret_y

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
at the face.

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
    walks, open_, waiting, last_row = [], {}, {}, {}
    with gzip.open(f"{out}/digrows.csv.gz", "rt") as fh:
        col = {k: n for n, k in enumerate(next(fh).rstrip("\n").split(","))}
        if "ret_x" not in col:
            sys.exit(f"{out}: digrows.csv.gz has no ret_x column (recorded before 2026-10-05)")
        F, I, HX, HY, HX2, HY2 = (col[k] for k in ("frame", "id", "hx", "hy", "hx_after", "hy_after"))
        HOLD, E, DIG, DX, DY, RX, RY, PAT = (col[k] for k in ("hold", "energy", "dig", "dig_x", "dig_y", "ret_x", "ret_y", "patience"))
        for line in fh:
            a = line.rstrip("\n").split(",")
            f, i = int(a[F]), int(a[I])
            head, head2 = (int(a[HX]), int(a[HY])), (int(a[HX2]), int(a[HY2]))
            hold, energy, pat = a[HOLD], float(a[E]), float(a[PAT])
            ret = (int(a[RX]), int(a[RY])) if a[RX] else None
            w = open_.get(i)

            def close(why):
                w.update(end=why, end_f=f)
                del open_[i]
                waiting[i] = w

            if w is not None and w["phase"] == "walk":
                if ret == w["cut"]:
                    w["prev"] = (head, head2, pat)
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
                        w.update(phase="walk", prev=(head, head2, pat))
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
                w = open_.get(i)
                if w is not None:
                    close("cut at its face on the way" if cheb(cut, w["cut"]) <= 2 else "cut somewhere else on the way")
                p = waiting.pop(i, None)
                if p is not None:
                    p["next_cut"] = cut
                w = dict(cut_f=f, cut=cut, phase="carry", ret_seen=False)
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


if __name__ == "__main__":
    if len(sys.argv) < 3 or sys.argv[1] not in ("dig", "soil", "rooms", "brood", "journeys", "face"):
        sys.exit(__doc__)
    for out in sys.argv[2:]:
        globals()[sys.argv[1]](out)
