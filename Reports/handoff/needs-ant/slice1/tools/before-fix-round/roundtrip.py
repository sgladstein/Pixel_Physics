"""roundtrip.py RUN [FROM TO]: the round trips the colony counter books (forage_max >= 8 cells, reset at
nest contact), found in the traced ants' ticks as a fall in forage_max between consecutive decisions,
keyed by what the ant held and (walk runs) its drive/job at the decision before the contact."""
import sys, csv, gzip, collections, os
run = sys.argv[1]
a = int(sys.argv[2]) if len(sys.argv) > 2 else 80000
b = int(sys.argv[3]) if len(sys.argv) > 3 else 100000
walk = {}
if os.path.exists(f"{run}/walk.csv.gz"):
    for r in csv.DictReader(gzip.open(f"{run}/walk.csv.gz", "rt")):
        walk[(int(r["frame"]), r["id"])] = (r["drive"], r["job"])
by = collections.defaultdict(list)
for r in csv.DictReader(gzip.open(f"{run}/ticks.csv.gz", "rt")):
    f = int(r["frame"])
    if a - 20000 <= f <= b:
        by[r["id"]].append(r)
cnt = collections.Counter(); depth = collections.defaultdict(list); ants = set()
for id_, rows in by.items():
    rows.sort(key=lambda r: int(r["frame"]))
    for p, c in zip(rows, rows[1:]):
        f = int(c["frame"])
        if not (a <= f <= b):
            continue
        ants.add(id_)
        try:
            pm, cm = int(p["forage_max"]), int(c["forage_max"])
        except ValueError:
            continue
        if pm >= 8 and cm < pm:
            load = "pellet" if p["spoil"] not in ("0", "") else ("food" if p["crop_cells"] not in ("0", "") else "empty")
            dj = walk.get((int(p["frame"]) + 1, id_), ("-", "-"))
            key = (load, f"{dj[0]}/{dj[1]}", p["zone"])
            cnt[key] += 1
            depth[key].append(pm)
tot = sum(cnt.values())
print(f"{run} {a}-{b}: {tot} round trips by {len(ants)} traced ants ({len({k for k in by})} in file)")
for k, v in cnt.most_common(14):
    d = sorted(depth[k])
    print(f"  {v:>4} held {k[0]:<6} {k[1]:<14} at {k[2]:<9} depth median {d[len(d)//2]}")
