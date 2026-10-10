#!/usr/bin/env python3
"""How far a layer stands from the nearest larva when it lays (FEED_FIRST's first check).

usage: laydistance.py RUNDIR [RUNDIR ...]

Every egg's FIRST `laid` row in broodlog.csv (later `laid` rows are re-appearances; the
count matches stats.csv's eggs_laid), its parent's position at the lay, against the
larvae in the nearest earlier brood.csv snapshot (every 1,000 frames, so up to 1,000
frames stale). Every larva is short of its pupation target by definition (it pupates on
reaching it), so "nearest larva" is "nearest hungry larva" as brood::larva_scent reads
it. Distance is Chebyshev (the reach box nearest_hungry_larva and larva_scent scan).
Positive control: a lay with a larva in the same snapshot at the parent's own cell
would read 0; the 'larvae in snapshot' column says whether there was anything to find.
"""
import csv, sys, bisect, collections

REACHES = (2, 4, 6, 8, 10, 15, 20)

for run in sys.argv[1:]:
    snaps = collections.defaultdict(list)
    for r in csv.DictReader(open(f"{run}/brood.csv")):
        if r["stage"] == "larva":
            snaps[int(r["frame"])].append((int(r["x"]), int(r["y"])))
    frames = sorted(snaps)
    seen, dists, none_found = set(), [], 0
    for r in csv.DictReader(open(f"{run}/broodlog.csv")):
        if r["event"] != "laid" or r["id"] in seen or not r["parent_x"]:
            continue
        seen.add(r["id"])
        f, px, py = int(r["frame"]), int(r["parent_x"]), int(r["parent_y"])
        i = bisect.bisect_right(frames, f) - 1
        larvae = snaps[frames[i]] if i >= 0 else []
        if not larvae:
            none_found += 1
            continue
        dists.append(min(max(abs(x - px), abs(y - py)) for x, y in larvae))
    n = len(dists) + none_found
    dists.sort()
    med = dists[len(dists) // 2] if dists else None
    share = "  ".join(f"<={k}: {sum(d <= k for d in dists) / max(n,1):.0%}" for k in REACHES)
    print(f"{run}: eggs {n} (no larva anywhere at the lay: {none_found}) | median distance {med} | {share}")
